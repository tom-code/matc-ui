use std::sync::Arc;

use tauri::State;

use crate::state::AppState;
use matc::clusters::codec;

#[tauri::command]
pub async fn invoke_command(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
    endpoint: u16,
    cluster: u32,
    command: u32,
    payload_hex: String,
) -> Result<String, String> {
    let payload = if payload_hex.is_empty() {
        vec![]
    } else {
        hex::decode(payload_hex.trim()).map_err(|e| format!("hex decode: {}", e))?
    };

    let state = state.inner().clone();
    let conn = get_conn(&state, node_id).await?;

    let result = conn
        .invoke_request(endpoint, cluster, command, &payload)
        .await;

    match result {
        Ok(msg) => {
            let json = serde_json::to_string_pretty(&format!("{:?}", msg.tlv))
                .unwrap_or_else(|_| "OK".to_string());
            Ok(json)
        }
        Err(e) => {
            AppState::drop_connection(&state, node_id).await;
            // retry with fresh connection
            let conn = AppState::get_connection(&state, node_id)
                .await
                .map_err(|e2| format!("reconnect failed: {} (original: {})", e2, e))?;
            let msg = conn
                .invoke_request(endpoint, cluster, command, &payload)
                .await
                .map_err(|e| e.to_string())?;
            Ok(format!("{:?}", msg.tlv))
        }
    }
}

#[tauri::command]
pub async fn invoke_command_typed(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
    endpoint: u16,
    cluster: u32,
    command: u32,
    args: serde_json::Value,
) -> Result<String, String> {
    let payload = codec::encode_command_json(cluster, command, &args)
        .map_err(|e| format!("encode: {}", e))?;

    let state = state.inner().clone();
    let conn = get_conn(&state, node_id).await?;

    let result = conn
        .invoke_request(endpoint, cluster, command, &payload)
        .await;

    match result {
        Ok(msg) => {
            let json = serde_json::to_string_pretty(&format!("{:?}", msg.tlv))
                .unwrap_or_else(|_| "OK".to_string());
            Ok(json)
        }
        Err(e) => {
            AppState::drop_connection(&state, node_id).await;
            let conn = AppState::get_connection(&state, node_id)
                .await
                .map_err(|e2| format!("reconnect failed: {} (original: {})", e2, e))?;
            let msg = conn
                .invoke_request(endpoint, cluster, command, &payload)
                .await
                .map_err(|e| e.to_string())?;
            Ok(format!("{:?}", msg.tlv))
        }
    }
}

async fn get_conn(
    state: &Arc<AppState>,
    node_id: u64,
) -> Result<Arc<matc::controller::Connection>, String> {
    match AppState::get_connection_with_retry(state, node_id).await {
        Ok(c) => Ok(c),
        Err(_) => {
            AppState::drop_connection(state, node_id).await;
            AppState::get_connection(state, node_id)
                .await
                .map_err(|e| e.to_string())
        }
    }
}
