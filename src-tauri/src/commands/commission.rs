use std::sync::Arc;

use tauri::State;

use crate::{commands::devices::DeviceDto, state::AppState};

#[derive(serde::Serialize)]
pub struct OpenCommissioningWindowResultDto {
    pub status: u32,
    pub manual_pairing_code: String,
    pub pin: u32,
    pub discriminator: u16,
    pub iterations: u32,
    pub timeout_secs: u16,
}

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
    AppState::drop_cache(&state, node_id).await;
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
    AppState::drop_cache(&state, node_id).await;
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
    AppState::drop_cache(&state, node_id).await;
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
pub async fn open_commissioning_window(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
    pin: u32,
    discriminator: u16,
    iterations: u32,
    timeout_secs: u16,
) -> Result<OpenCommissioningWindowResultDto, String> {
    use matc::clusters::codec::admin_commissioning_cluster::encode_open_commissioning_window;
    use matc::clusters::defs::{
        CLUSTER_ADMINISTRATOR_COMMISSIONING_CMD_ID_OPENCOMMISSIONINGWINDOW,
        CLUSTER_ID_ADMINISTRATOR_COMMISSIONING,
    };

    let state = state.inner().clone();

    let conn = match AppState::get_connection_with_retry(&state, node_id).await {
        Ok(c) => c,
        Err(_) => {
            AppState::drop_connection(&state, node_id).await;
            AppState::get_connection(&state, node_id)
                .await
                .map_err(|e| e.to_string())?
        }
    };

    let mut salt = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut salt);

    let key = matc::controller::pin_to_passcode(pin).map_err(|e| e.to_string())?;
    let verifier = matc::spake2p::Engine::create_passcode_verifier(&key, &salt, iterations);

    let payload = encode_open_commissioning_window(
        timeout_secs,
        verifier,
        discriminator,
        iterations,
        salt.to_vec(),
    )
    .map_err(|e| e.to_string())?;

    let resp = conn
        .invoke_request_timed(
            0,
            CLUSTER_ID_ADMINISTRATOR_COMMISSIONING,
            CLUSTER_ADMINISTRATOR_COMMISSIONING_CMD_ID_OPENCOMMISSIONINGWINDOW,
            &payload,
            6000,
        )
        .await
        .map_err(|e| e.to_string())?;

    let (_, status) = matc::messages::parse_im_invoke_resp(&resp.tlv).map_err(|e| e.to_string())?;

    let manual_pairing_code =
        matc::onboarding::encode_manual_pairing_code(&matc::onboarding::OnboardingInfo {
            discriminator,
            passcode: pin,
            is_short_discriminator: false,
            vendor_id: None,
            product_id: None,
            discovery_capabilities: None,
        });

    Ok(OpenCommissioningWindowResultDto {
        status,
        manual_pairing_code,
        pin,
        discriminator,
        iterations,
        timeout_secs,
    })
}
