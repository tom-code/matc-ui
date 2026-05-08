use std::sync::Arc;

use matc::{
    clusters::{codec, defs::*, names},
    tlv::TlvItemValue,
};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

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

#[tauri::command]
pub async fn read_attribute_tree(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
) -> Result<EndpointTree, String> {
    let state = state.inner().clone();
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

    let mut ep_nodes = Vec::new();
    for ep in endpoints {
        let cluster_tlv = conn
            .read_request2(
                ep,
                CLUSTER_ID_DESCRIPTOR,
                CLUSTER_DESCRIPTOR_ATTR_ID_SERVERLIST,
            )
            .await;
        let cluster_ids: Vec<u32> = match cluster_tlv {
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

        let mut cluster_nodes = Vec::new();
        for cluster_id in cluster_ids {
            let attr_list = codec::get_attribute_list(cluster_id);
            let cluster_name = names::get_cluster_name(cluster_id)
                .unwrap_or("Unknown")
                .to_string();

            let mut attr_nodes = Vec::new();
            for (attr_id, attr_name) in attr_list {
                let (value, error) = match conn.read_request2(ep, cluster_id, attr_id).await {
                    Ok(v) => (
                        Some(codec::decode_attribute_json(cluster_id, attr_id, &v)),
                        None,
                    ),
                    Err(e) => (None, Some(e.to_string())),
                };
                attr_nodes.push(AttrNode {
                    id: attr_id,
                    name: attr_name.to_string(),
                    value,
                    error,
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

    Ok(EndpointTree {
        endpoints: ep_nodes,
    })
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
