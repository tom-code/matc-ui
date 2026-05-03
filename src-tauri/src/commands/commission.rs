use std::sync::Arc;

use tauri::State;

use crate::{commands::devices::DeviceDto, state::AppState};

#[tauri::command]
pub async fn commission_by_code(
    state: State<'_, Arc<AppState>>,
    pairing_code: String,
    node_id: u64,
    name: String,
) -> Result<DeviceDto, String> {
    let state = state.inner().clone();
    let conn = {
        let dm = &state.devman;
        dm.commission_with_code(&pairing_code, node_id, &name)
            .await
            .map_err(|e| e.to_string())?
    };
    let conn = Arc::new(conn);
    state.connections.lock().await.insert(node_id, conn);

    let dev = state
        .devman
        .get_device(node_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "device not found after commission".to_string())?;
    Ok(DeviceDto {
        node_id: dev.node_id,
        name: dev.name,
        address: dev.address,
    })
}

#[tauri::command]
pub async fn commission_by_address(
    state: State<'_, Arc<AppState>>,
    address: String,
    pin: u32,
    node_id: u64,
    name: String,
) -> Result<DeviceDto, String> {
    let state = state.inner().clone();
    let conn = {
        let dm = &state.devman;
        dm.commission(&address, pin, node_id, &name)
            .await
            .map_err(|e| e.to_string())?
    };
    let conn = Arc::new(conn);
    state.connections.lock().await.insert(node_id, conn);

    let dev = state
        .devman
        .get_device(node_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "device not found after commission".to_string())?;
    Ok(DeviceDto {
        node_id: dev.node_id,
        name: dev.name,
        address: dev.address,
    })
}

#[cfg(feature = "ble")]
#[tauri::command]
pub async fn commission_ble(
    state: State<'_, Arc<AppState>>,
    pairing_code: String,
    node_id: u64,
    name: String,
    wifi_ssid: String,
    wifi_password: String,
) -> Result<DeviceDto, String> {
    use matc::NetworkCreds;

    let creds = NetworkCreds::WiFi {
        ssid: wifi_ssid.into(),
        creds: wifi_password.into(),
    };
    let state = state.inner().clone();
    let conn = {
        let dm = &state.devman;
        dm.commission_ble_with_code(&pairing_code, node_id, &name, creds)
            .await
            .map_err(|e| e.to_string())?
    };
    let conn = Arc::new(conn);
    state.connections.lock().await.insert(node_id, conn);

    let dev = state
        .devman
        .get_device(node_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "device not found after commission".to_string())?;
    Ok(DeviceDto {
        node_id: dev.node_id,
        name: dev.name,
        address: dev.address,
    })
}

#[cfg(not(feature = "ble"))]
#[tauri::command]
pub async fn commission_ble(
    _state: State<'_, Arc<AppState>>,
    _pairing_code: String,
    _node_id: u64,
    _name: String,
    _wifi_ssid: String,
    _wifi_password: String,
) -> Result<DeviceDto, String> {
    Err("BLE support not compiled in".to_string())
}
