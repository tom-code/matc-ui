use std::{collections::HashMap, sync::Arc};

use matc::{
    clusters::{codec, defs::*, names},
    tlv::TlvItemValue,
};
use serde::{Deserialize, Serialize};
use tauri::{ipc::Channel, State};

use crate::state::AppState;

const ATTR_ID_GENERATED_COMMAND_LIST: u32 = 0xFFF8;
const ATTR_ID_ACCEPTED_COMMAND_LIST: u32 = 0xFFF9;
const ATTR_ID_EVENT_LIST: u32 = 0xFFFA;
const ATTR_ID_ATTRIBUTE_LIST: u32 = 0xFFFB;
const ATTR_ID_FEATURE_MAP: u32 = 0xFFFC;
const ATTR_ID_CLUSTER_REVISION: u32 = 0xFFFD;

fn decode_global_attr(attr_id: u32, tlv: &TlvItemValue) -> Option<String> {
    match attr_id {
        ATTR_ID_GENERATED_COMMAND_LIST
        | ATTR_ID_ACCEPTED_COMMAND_LIST
        | ATTR_ID_EVENT_LIST
        | ATTR_ID_ATTRIBUTE_LIST => {
            let TlvItemValue::List(items) = tlv else {
                return None;
            };
            let ids: Vec<String> = items
                .iter()
                .filter_map(|i| {
                    if let TlvItemValue::Int(v) = i.value {
                        Some((v as u32).to_string())
                    } else {
                        None
                    }
                })
                .collect();
            Some(format!("[{}]", ids.join(",")))
        }
        ATTR_ID_FEATURE_MAP | ATTR_ID_CLUSTER_REVISION => {
            let TlvItemValue::Int(v) = tlv else {
                return None;
            };
            Some(v.to_string())
        }
        _ => None,
    }
}

fn attr_name(cluster_id: u32, attr_id: u32) -> String {
    match attr_id {
        ATTR_ID_GENERATED_COMMAND_LIST => return "GeneratedCommandList".to_string(),
        ATTR_ID_ACCEPTED_COMMAND_LIST => return "AcceptedCommandList".to_string(),
        ATTR_ID_EVENT_LIST => return "EventList".to_string(),
        ATTR_ID_ATTRIBUTE_LIST => return "AttributeList".to_string(),
        ATTR_ID_FEATURE_MAP => return "FeatureMap".to_string(),
        ATTR_ID_CLUSTER_REVISION => return "ClusterRevision".to_string(),
        _ => {}
    }
    let known: HashMap<u32, &'static str> =
        codec::get_attribute_list(cluster_id).into_iter().collect();
    known
        .get(&attr_id)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("Attr 0x{:04X}", attr_id))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttrNode {
    pub id: u32,
    pub name: String,
    pub value: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    pub id: u32,
    pub name: String,
    pub attributes: Vec<AttrNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointNode {
    pub id: u16,
    pub clusters: Vec<ClusterNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointTree {
    pub endpoints: Vec<EndpointNode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttrProgressEvent {
    pub phase: &'static str,
    pub endpoint_index: usize,
    pub endpoint_count: usize,
    pub endpoint_id: Option<u16>,
    pub endpoint_attr_index: usize,
    pub endpoint_attr_total: usize,
    pub current_cluster: Option<String>,
}

#[tauri::command]
pub async fn read_attribute_tree(
    state: State<'_, Arc<AppState>>,
    channel: Channel<AttrProgressEvent>,
    node_id: u64,
    force_refresh: Option<bool>,
) -> Result<EndpointTree, String> {
    let force_refresh = force_refresh.unwrap_or(false);
    let state = state.inner().clone();

    if !force_refresh {
        if let Some(cached) = state.cache_get_attributes::<EndpointTree>(node_id).await {
            return Ok(cached);
        }
    }

    let _ = channel.send(AttrProgressEvent {
        phase: "connecting",
        endpoint_index: 0,
        endpoint_count: 0,
        endpoint_id: None,
        endpoint_attr_index: 0,
        endpoint_attr_total: 0,
        current_cluster: None,
    });

    let conn = get_conn(&state, node_id).await?;

    let parts_tlv = conn
        .read_request2(
            0,
            CLUSTER_ID_DESCRIPTOR,
            CLUSTER_DESCRIPTOR_ATTR_ID_PARTSLIST,
        )
        .await
        .map_err(|e| e.to_string())?;

    let mut endpoints: Vec<u16> = match parts_tlv {
        TlvItemValue::List(items) => items
            .iter()
            .filter_map(|i| {
                if let TlvItemValue::Int(v) = i.value {
                    Some(v as u16)
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    };
    endpoints.push(0);
    endpoints.sort();
    endpoints.dedup();

    let endpoint_count = endpoints.len();

    // Phase 1: discover all endpoints - read ServerList and AttributeList for each cluster
    let mut ep_discover: Vec<(u16, Vec<(u32, String, Vec<u32>)>)> = Vec::new();

    for (ep_idx, &ep) in endpoints.iter().enumerate() {
        let _ = channel.send(AttrProgressEvent {
            phase: "discover",
            endpoint_index: ep_idx,
            endpoint_count,
            endpoint_id: Some(ep),
            endpoint_attr_index: 0,
            endpoint_attr_total: 0,
            current_cluster: None,
        });

        let cluster_ids: Vec<u32> = match conn
            .read_request2(
                ep,
                CLUSTER_ID_DESCRIPTOR,
                CLUSTER_DESCRIPTOR_ATTR_ID_SERVERLIST,
            )
            .await
        {
            Ok(TlvItemValue::List(items)) => items
                .iter()
                .filter_map(|i| {
                    if let TlvItemValue::Int(v) = i.value {
                        Some(v as u32)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => continue,
        };

        let mut cluster_attr_lists: Vec<(u32, String, Vec<u32>)> = Vec::new();
        for cluster_id in cluster_ids {
            let cluster_name = names::get_cluster_name(cluster_id)
                .unwrap_or("Unknown")
                .to_string();
            let attr_ids: Vec<u32> = match conn
                .read_request2(ep, cluster_id, ATTR_ID_ATTRIBUTE_LIST)
                .await
            {
                Ok(TlvItemValue::List(items)) => items
                    .iter()
                    .filter_map(|i| {
                        if let TlvItemValue::Int(v) = i.value {
                            Some(v as u32)
                        } else {
                            None
                        }
                    })
                    .collect(),
                _ => vec![],
            };
            cluster_attr_lists.push((cluster_id, cluster_name, attr_ids));
        }
        ep_discover.push((ep, cluster_attr_lists));
    }

    // Grand total is now known: read phase can show a single 0->100% bar without resets
    let grand_total: usize = ep_discover
        .iter()
        .flat_map(|(_, cls)| cls.iter())
        .map(|(_, _, ids)| ids.len())
        .sum();

    // Phase 2: read all attribute values, reporting cumulative progress
    let mut global_attr_idx: usize = 0;
    let mut ep_nodes = Vec::new();

    for (ep, cluster_attr_lists) in ep_discover {
        let mut cluster_nodes = Vec::new();
        for (cluster_id, cluster_name, attr_ids) in cluster_attr_lists {
            let mut attr_nodes = Vec::new();
            for attr_id in attr_ids {
                let name = attr_name(cluster_id, attr_id);
                let (value, error) = match conn.read_request2(ep, cluster_id, attr_id).await {
                    Ok(v) => {
                        let decoded = decode_global_attr(attr_id, &v).unwrap_or_else(|| {
                            codec::decode_attribute_json(cluster_id, attr_id, &v)
                        });
                        (Some(decoded), None)
                    }
                    Err(e) => (None, Some(e.to_string())),
                };
                attr_nodes.push(AttrNode {
                    id: attr_id,
                    name,
                    value,
                    error,
                });
                global_attr_idx += 1;
                let _ = channel.send(AttrProgressEvent {
                    phase: "read",
                    endpoint_index: 0,
                    endpoint_count: 0,
                    endpoint_id: Some(ep),
                    endpoint_attr_index: global_attr_idx,
                    endpoint_attr_total: grand_total,
                    current_cluster: Some(cluster_name.clone()),
                });
            }
            cluster_nodes.push(ClusterNode {
                id: cluster_id,
                name: cluster_name,
                attributes: attr_nodes,
            });
        }
        ep_nodes.push(EndpointNode {
            id: ep,
            clusters: cluster_nodes,
        });
    }

    let _ = channel.send(AttrProgressEvent {
        phase: "done",
        endpoint_index: endpoint_count,
        endpoint_count,
        endpoint_id: None,
        endpoint_attr_index: 0,
        endpoint_attr_total: 0,
        current_cluster: None,
    });

    let tree = EndpointTree {
        endpoints: ep_nodes,
    };
    state.cache_set_attributes(node_id, &tree).await;
    Ok(tree)
}

#[tauri::command]
pub async fn read_device_tree(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
    force_refresh: Option<bool>,
) -> Result<EndpointTree, String> {
    let force_refresh = force_refresh.unwrap_or(false);
    let state = state.inner().clone();

    if !force_refresh {
        if let Some(cached) = state.cache_get_device_tree::<EndpointTree>(node_id).await {
            return Ok(cached);
        }
    }

    let conn = get_conn(&state, node_id).await?;

    // EP 0 PartsList enumerates all other endpoints on the device
    let mut endpoint_ids: Vec<u16> = vec![0];
    if let Ok(TlvItemValue::List(items)) = conn
        .read_request2(
            0,
            CLUSTER_ID_DESCRIPTOR,
            CLUSTER_DESCRIPTOR_ATTR_ID_PARTSLIST,
        )
        .await
    {
        for item in &items {
            if let TlvItemValue::Int(v) = item.value {
                endpoint_ids.push(v as u16);
            }
        }
    }
    endpoint_ids.sort();
    endpoint_ids.dedup();

    let mut ep_nodes = Vec::new();
    for ep in endpoint_ids {
        let server_list: Vec<u32> = match conn
            .read_request2(
                ep,
                CLUSTER_ID_DESCRIPTOR,
                CLUSTER_DESCRIPTOR_ATTR_ID_SERVERLIST,
            )
            .await
        {
            Ok(TlvItemValue::List(items)) => items
                .iter()
                .filter_map(|i| {
                    if let TlvItemValue::Int(v) = i.value {
                        Some(v as u32)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => continue,
        };

        let mut clusters = Vec::new();

        // Descriptor: DeviceTypeList and PartsList are what buildDeviceTree needs
        let descriptor_attrs = [
            (CLUSTER_DESCRIPTOR_ATTR_ID_DEVICETYPELIST, "DeviceTypeList"),
            (CLUSTER_DESCRIPTOR_ATTR_ID_PARTSLIST, "PartsList"),
        ];
        let mut desc_attr_nodes = Vec::new();
        for (attr_id, name) in descriptor_attrs {
            let (value, error) = match conn.read_request2(ep, CLUSTER_ID_DESCRIPTOR, attr_id).await
            {
                Ok(v) => (
                    Some(codec::decode_attribute_json(
                        CLUSTER_ID_DESCRIPTOR,
                        attr_id,
                        &v,
                    )),
                    None,
                ),
                Err(e) => (None, Some(e.to_string())),
            };
            desc_attr_nodes.push(AttrNode {
                id: attr_id,
                name: name.to_string(),
                value,
                error,
            });
        }
        clusters.push(ClusterNode {
            id: CLUSTER_ID_DESCRIPTOR,
            name: "Descriptor".to_string(),
            attributes: desc_attr_nodes,
        });

        // Info cluster: prefer BridgedDeviceBasicInformation, fall back to BasicInformation
        let info_cluster = if server_list.contains(&CLUSTER_ID_BRIDGED_DEVICE_BASIC_INFORMATION) {
            Some((
                CLUSTER_ID_BRIDGED_DEVICE_BASIC_INFORMATION,
                "BridgedDeviceBasicInformation",
            ))
        } else if server_list.contains(&CLUSTER_ID_BASIC_INFORMATION) {
            Some((CLUSTER_ID_BASIC_INFORMATION, "BasicInformation"))
        } else {
            None
        };

        if let Some((cluster_id, cluster_name)) = info_cluster {
            let info_attrs = [
                (CLUSTER_BASIC_INFORMATION_ATTR_ID_VENDORNAME, "VendorName"),
                (CLUSTER_BASIC_INFORMATION_ATTR_ID_PRODUCTNAME, "ProductName"),
                (CLUSTER_BASIC_INFORMATION_ATTR_ID_NODELABEL, "NodeLabel"),
                (CLUSTER_BASIC_INFORMATION_ATTR_ID_REACHABLE, "Reachable"),
            ];
            let mut info_attr_nodes = Vec::new();
            for (attr_id, name) in info_attrs {
                let (value, error) = match conn.read_request2(ep, cluster_id, attr_id).await {
                    Ok(v) => (
                        Some(codec::decode_attribute_json(cluster_id, attr_id, &v)),
                        None,
                    ),
                    Err(_) => (None, None),
                };
                info_attr_nodes.push(AttrNode {
                    id: attr_id,
                    name: name.to_string(),
                    value,
                    error,
                });
            }
            clusters.push(ClusterNode {
                id: cluster_id,
                name: cluster_name.to_string(),
                attributes: info_attr_nodes,
            });
        }

        ep_nodes.push(EndpointNode { id: ep, clusters });
    }

    let tree = EndpointTree {
        endpoints: ep_nodes,
    };
    state.cache_set_device_tree(node_id, &tree).await;
    Ok(tree)
}

#[tauri::command]
pub async fn read_endpoint_structure(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
    force_refresh: Option<bool>,
) -> Result<EndpointTree, String> {
    let force_refresh = force_refresh.unwrap_or(false);
    let state = state.inner().clone();

    if !force_refresh {
        if let Some(cached) = state.cache_get_structure::<EndpointTree>(node_id).await {
            return Ok(cached);
        }
    }

    let conn = get_conn(&state, node_id).await?;

    let mut endpoint_ids: Vec<u16> = vec![0];
    if let Ok(TlvItemValue::List(items)) = conn
        .read_request2(
            0,
            CLUSTER_ID_DESCRIPTOR,
            CLUSTER_DESCRIPTOR_ATTR_ID_PARTSLIST,
        )
        .await
    {
        for item in &items {
            if let TlvItemValue::Int(v) = item.value {
                endpoint_ids.push(v as u16);
            }
        }
    }
    endpoint_ids.sort();
    endpoint_ids.dedup();

    let mut ep_nodes = Vec::new();
    for ep in endpoint_ids {
        let cluster_ids: Vec<u32> = match conn
            .read_request2(
                ep,
                CLUSTER_ID_DESCRIPTOR,
                CLUSTER_DESCRIPTOR_ATTR_ID_SERVERLIST,
            )
            .await
        {
            Ok(TlvItemValue::List(items)) => items
                .iter()
                .filter_map(|i| {
                    if let TlvItemValue::Int(v) = i.value {
                        Some(v as u32)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => continue,
        };

        let clusters = cluster_ids
            .into_iter()
            .map(|cluster_id| ClusterNode {
                id: cluster_id,
                name: names::get_cluster_name(cluster_id)
                    .unwrap_or("Unknown")
                    .to_string(),
                attributes: vec![],
            })
            .collect();

        ep_nodes.push(EndpointNode { id: ep, clusters });
    }

    let tree = EndpointTree {
        endpoints: ep_nodes,
    };
    state.cache_set_structure(node_id, &tree).await;
    Ok(tree)
}

#[tauri::command]
pub async fn read_single_attribute(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
    endpoint: u16,
    cluster: u32,
    attr_id: u32,
) -> Result<String, String> {
    let state = state.inner().clone();
    let conn = get_conn(&state, node_id).await?;
    let value = conn
        .read_request2(endpoint, cluster, attr_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(codec::decode_attribute_json(cluster, attr_id, &value))
}

async fn get_conn(
    state: &Arc<AppState>,
    node_id: u64,
) -> Result<Arc<matc::controller::Connection>, String> {
    let has_cache = state.connections.lock().await.contains_key(&node_id);
    if has_cache {
        match AppState::get_connection_with_retry(state, node_id).await {
            Ok(c) => return Ok(c),
            Err(_) => AppState::drop_connection(state, node_id).await,
        }
    }
    AppState::get_connection(state, node_id)
        .await
        .map_err(|e| e.to_string())
}
