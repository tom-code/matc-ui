use std::{sync::Arc, time::Duration};

use matc::clusters::defs::*;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::{emit_device_status, AppState, DeviceStatus, DeviceStatusDto};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDto {
    pub node_id: u64,
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfoDto {
    pub node_id: u64,
    pub name: String,
    pub address: String,
    pub vendor_name: String,
    pub product_name: String,
    pub hw_version: String,
    pub sw_version: String,
    pub has_aggregator: bool,
}

/// Probe a single device: acquires the per-node probe lock, updates backend status,
/// emits device://status events, and returns the DeviceInfoDto on success.
/// Called by get_device_info (manual) and the background sweep (automatic).
pub(crate) async fn probe_device(
    state: &Arc<AppState>,
    app: &tauri::AppHandle,
    node_id: u64,
    force_refresh: bool,
) -> Result<DeviceInfoDto, String> {
    let probe_lock = AppState::probe_lock_for(state, node_id);
    let _guard = probe_lock.lock().await;

    let changed = state
        .set_status(node_id, DeviceStatus::Checking, None)
        .await;
    if changed {
        log::debug!("status: node={} -> checking", node_id);
        emit_device_status(
            app,
            DeviceStatusDto {
                node_id,
                status: DeviceStatus::Checking,
                error: None,
                info: None,
            },
        );
    }

    let result = do_probe(state, node_id, force_refresh).await;

    match result {
        Ok(ref info) => {
            state
                .set_status(node_id, DeviceStatus::Connected, None)
                .await;
            log::info!("status: node={} -> connected ({})", node_id, info.address);
            emit_device_status(
                app,
                DeviceStatusDto {
                    node_id,
                    status: DeviceStatus::Connected,
                    error: None,
                    info: serde_json::to_value(info).ok(),
                },
            );
        }
        Err(ref e) => {
            state
                .set_status(node_id, DeviceStatus::Failed, Some(e.clone()))
                .await;
            log::warn!("status: node={} -> failed err={}", node_id, e);
            emit_device_status(
                app,
                DeviceStatusDto {
                    node_id,
                    status: DeviceStatus::Failed,
                    error: Some(e.clone()),
                    info: None,
                },
            );
        }
    }

    result
}

async fn do_probe(
    state: &Arc<AppState>,
    node_id: u64,
    force_refresh: bool,
) -> Result<DeviceInfoDto, String> {
    let (name, address) = {
        let dm = &state.devman;
        let dev = dm
            .get_device(node_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("device {} not found", node_id))?;
        (dev.name.clone(), dev.address.clone())
    };

    // Establishing the connection serves as the liveness check for the
    // force_refresh=false + cache-hit path. If the CASE session is missing
    // (e.g., first probe after restart), this does the handshake.
    // For cached sessions, stale-session recovery happens below in the reads,
    // via with_connection_retry.
    AppState::get_connection(state, node_id)
        .await
        .map_err(|e| e.to_string())?;

    if !force_refresh {
        if let Some(mut cached) = state.cache_get_info::<DeviceInfoDto>(node_id).await {
            // name and address come from the registry and may change; always use fresh values.
            cached.name = name;
            cached.address = address;
            return Ok(cached);
        }
    }

    // Cache miss or force_refresh: read from device.
    // with_connection_retry handles a stale session from power-cycle.
    let vendor_name = match tokio::time::timeout(
        Duration::from_secs(4),
        AppState::with_connection_retry(state, node_id, |conn| async move {
            conn.read_request2(
                0,
                CLUSTER_ID_BASIC_INFORMATION,
                CLUSTER_BASIC_INFORMATION_ATTR_ID_VENDORNAME,
            )
            .await
        }),
    )
    .await
    {
        Ok(Ok(matc::tlv::TlvItemValue::String(s))) => s,
        Ok(Ok(v)) => format!("{:?}", v),
        Ok(Err(e)) => return Err(e),
        Err(_) => {
            AppState::drop_connection(state, node_id).await;
            return Err("read timed out".to_string());
        }
    };

    // with_connection_retry may have reconnected; get_connection returns the
    // now-cached (possibly fresh) conn for the remaining reads.
    let conn = AppState::get_connection(state, node_id)
        .await
        .map_err(|e| e.to_string())?;

    let product_name = read_string(
        &conn,
        0,
        CLUSTER_ID_BASIC_INFORMATION,
        CLUSTER_BASIC_INFORMATION_ATTR_ID_PRODUCTNAME,
    )
    .await;
    let hw_version = read_string(
        &conn,
        0,
        CLUSTER_ID_BASIC_INFORMATION,
        CLUSTER_BASIC_INFORMATION_ATTR_ID_HARDWAREVERSIONSTRING,
    )
    .await;
    let sw_version = read_string(
        &conn,
        0,
        CLUSTER_ID_BASIC_INFORMATION,
        CLUSTER_BASIC_INFORMATION_ATTR_ID_SOFTWAREVERSIONSTRING,
    )
    .await;

    let has_aggregator = read_has_aggregator(&conn).await;

    let result = DeviceInfoDto {
        node_id,
        name,
        address,
        vendor_name,
        product_name,
        hw_version,
        sw_version,
        has_aggregator,
    };
    state.cache_set_info(node_id, &result).await;
    Ok(result)
}

#[tauri::command]
pub async fn list_devices(state: State<'_, Arc<AppState>>) -> Result<Vec<DeviceDto>, String> {
    let dm = &state.devman;
    let devices = dm.list_devices().map_err(|e| e.to_string())?;
    Ok(devices
        .into_iter()
        .map(|d| DeviceDto {
            node_id: d.node_id,
            name: d.name,
            address: d.address,
        })
        .collect())
}

#[tauri::command]
pub async fn get_device_info(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    node_id: u64,
    force_refresh: Option<bool>,
) -> Result<DeviceInfoDto, String> {
    let force_refresh = force_refresh.unwrap_or(false);
    let state = state.inner().clone();
    probe_device(&state, &app, node_id, force_refresh).await
}

#[tauri::command]
pub async fn get_device_statuses(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<DeviceStatusDto>, String> {
    let devices = state.devman.list_devices().map_err(|e| e.to_string())?;
    let snapshots: Vec<(u64, DeviceStatus, Option<String>)> = {
        let map = state.device_status.lock().await;
        devices
            .iter()
            .map(|d| {
                let entry = map.get(&d.node_id);
                (
                    d.node_id,
                    entry.map(|e| e.status).unwrap_or(DeviceStatus::Unknown),
                    entry.and_then(|e| e.error.clone()),
                )
            })
            .collect()
    };
    let mut result = Vec::new();
    for (node_id, status, error) in snapshots {
        let info = if status == DeviceStatus::Connected {
            state
                .cache_get_info::<DeviceInfoDto>(node_id)
                .await
                .and_then(|i| serde_json::to_value(i).ok())
        } else {
            None
        };
        result.push(DeviceStatusDto {
            node_id,
            status,
            error,
            info,
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn rename_device(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
    name: String,
) -> Result<(), String> {
    let dm = &state.devman;
    dm.rename_device(node_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_device(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    node_id: u64,
) -> Result<(), String> {
    let state = state.inner().clone();
    log::info!("remove: node={}", node_id);
    AppState::drop_connection(&state, node_id).await;
    AppState::drop_cache(&state, node_id).await;
    state.remove_status(node_id).await;
    let dm = &state.devman;
    dm.remove_device(node_id).map_err(|e| e.to_string())?;
    log::info!("status: node={} -> removed", node_id);
    emit_device_status(
        &app,
        DeviceStatusDto {
            node_id,
            status: DeviceStatus::Removed,
            error: None,
            info: None,
        },
    );
    Ok(())
}

async fn read_string(
    conn: &Arc<matc::controller::Connection>,
    endpoint: u16,
    cluster: u32,
    attr: u32,
) -> String {
    match conn.read_request2(endpoint, cluster, attr).await {
        Ok(matc::tlv::TlvItemValue::String(s)) => s,
        Ok(v) => format!("{:?}", v),
        Err(_) => String::new(),
    }
}

const DEVICE_TYPE_AGGREGATOR: u64 = 0x000E;

async fn read_has_aggregator(conn: &Arc<matc::controller::Connection>) -> bool {
    // EP 0's PartsList contains all other endpoint IDs on the device.
    let mut endpoints = vec![0u16];
    if let Ok(matc::tlv::TlvItemValue::List(items)) = conn
        .read_request2(
            0,
            CLUSTER_ID_DESCRIPTOR,
            CLUSTER_DESCRIPTOR_ATTR_ID_PARTSLIST,
        )
        .await
    {
        for item in &items {
            if let matc::tlv::TlvItemValue::Int(i) = &item.value {
                endpoints.push(*i as u16);
            }
        }
    }

    for ep in endpoints {
        if let Ok(matc::tlv::TlvItemValue::List(items)) = conn
            .read_request2(
                ep,
                CLUSTER_ID_DESCRIPTOR,
                CLUSTER_DESCRIPTOR_ATTR_ID_DEVICETYPELIST,
            )
            .await
        {
            if items
                .iter()
                .any(|item| item.get_int(&[0]) == Some(DEVICE_TYPE_AGGREGATOR))
            {
                return true;
            }
        }
    }
    false
}
