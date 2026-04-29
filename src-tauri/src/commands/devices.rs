use std::{sync::Arc, time::Duration};

use matc::clusters::defs::*;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

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
    pub sw_version: String,
}

#[tauri::command]
pub async fn list_devices(state: State<'_, Arc<AppState>>) -> Result<Vec<DeviceDto>, String> {
    let dm = &state.devman;
    let devices = dm.list_devices().map_err(|e| e.to_string())?;
    Ok(devices
        .into_iter()
        .map(|d| DeviceDto { node_id: d.node_id, name: d.name, address: d.address })
        .collect())
}

#[tauri::command]
pub async fn get_device_info(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
) -> Result<DeviceInfoDto, String> {
    let state = state.inner().clone();

    let (name, address) = {
        let dm = &state.devman;
        let dev = dm
            .get_device(node_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("device {} not found", node_id))?;
        (dev.name.clone(), dev.address.clone())
    };

    let conn = get_conn_with_retry(&state, node_id).await?;

    let vendor_name = match tokio::time::timeout(
        Duration::from_secs(4),
        conn.read_request2(0, CLUSTER_ID_BASIC_INFORMATION, CLUSTER_BASIC_INFORMATION_ATTR_ID_VENDORNAME),
    ).await {
        Ok(Ok(matc::tlv::TlvItemValue::String(s))) => s,
        Ok(Ok(v)) => format!("{:?}", v),
        Ok(Err(e)) => {
            AppState::drop_connection(&state, node_id).await;
            return Err(e.to_string());
        }
        Err(_) => {
            AppState::drop_connection(&state, node_id).await;
            return Err("read timed out".to_string());
        }
    };

    let product_name = read_string(&conn, 0, CLUSTER_ID_BASIC_INFORMATION, CLUSTER_BASIC_INFORMATION_ATTR_ID_PRODUCTNAME).await;
    let sw_version = read_string(&conn, 0, CLUSTER_ID_BASIC_INFORMATION, CLUSTER_BASIC_INFORMATION_ATTR_ID_SOFTWAREVERSIONSTRING).await;

    Ok(DeviceInfoDto { node_id, name, address, vendor_name, product_name, sw_version })
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
    node_id: u64,
) -> Result<(), String> {
    let state = state.inner().clone();
    AppState::drop_connection(&state, node_id).await;
    let dm = &state.devman;
    dm.remove_device(node_id).map_err(|e| e.to_string())
}

async fn get_conn_with_retry(
    state: &Arc<AppState>,
    node_id: u64,
) -> Result<Arc<matc::controller::Connection>, String> {
    match AppState::get_connection_with_retry(state, node_id).await {
        Ok(c) => Ok(c),
        Err(_) => {
            AppState::drop_connection(state, node_id).await;
            AppState::get_connection(state, node_id).await.map_err(|e| e.to_string())
        }
    }
}
