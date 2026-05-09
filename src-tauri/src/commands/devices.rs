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
    pub hw_version: String,
    pub sw_version: String,
    pub has_aggregator: bool,
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
    node_id: u64,
    force_refresh: Option<bool>,
) -> Result<DeviceInfoDto, String> {
    let force_refresh = force_refresh.unwrap_or(false);
    let state = state.inner().clone();

    let (name, address) = {
        let dm = &state.devman;
        let dev = dm
            .get_device(node_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("device {} not found", node_id))?;
        (dev.name.clone(), dev.address.clone())
    };

    // Establishing the connection serves as the liveness check even when returning
    // cached attribute data. If the device is unreachable, this returns an error
    // and the frontend marks the device as failed.
    let conn = get_conn_with_retry(&state, node_id).await?;

    if !force_refresh {
        if let Some(mut cached) = state.cache_get_info::<DeviceInfoDto>(node_id).await {
            // name and address come from the registry and may change; always use fresh values.
            cached.name = name;
            cached.address = address;
            return Ok(cached);
        }
    }

    let vendor_name = match tokio::time::timeout(
        Duration::from_secs(4),
        conn.read_request2(
            0,
            CLUSTER_ID_BASIC_INFORMATION,
            CLUSTER_BASIC_INFORMATION_ATTR_ID_VENDORNAME,
        ),
    )
    .await
    {
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
pub async fn remove_device(state: State<'_, Arc<AppState>>, node_id: u64) -> Result<(), String> {
    let state = state.inner().clone();
    AppState::drop_connection(&state, node_id).await;
    AppState::drop_cache(&state, node_id).await;
    let dm = &state.devman;
    dm.remove_device(node_id).map_err(|e| e.to_string())
}

async fn get_conn_with_retry(
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
