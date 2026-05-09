use std::sync::Arc;

use matc::tlv::TlvItem;
use tauri::State;

use crate::state::AppState;
use matc::clusters::codec;

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InvokeResultDto {
    Status { code: u16 },
    Data { tlv: String },
}

fn format_invoke_result(tlv: &TlvItem) -> InvokeResultDto {
    if let Some(code) = tlv.get_int(&[1, 0, 1, 1, 0]) {
        return InvokeResultDto::Status { code: code as u16 };
    }
    InvokeResultDto::Data {
        tlv: format!("{:#?}", tlv),
    }
}

#[tauri::command]
pub async fn invoke_command(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
    endpoint: u16,
    cluster: u32,
    command: u32,
    payload_hex: String,
) -> Result<InvokeResultDto, String> {
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
        Ok(msg) => Ok(format_invoke_result(&msg.tlv)),
        Err(e) => {
            // Drop our Arc first so the old read loop is cancelled before we open
            // a new socket to the same address (see state.rs:11-15).
            drop(conn);
            AppState::drop_connection(&state, node_id).await;
            let conn = AppState::get_connection(&state, node_id)
                .await
                .map_err(|e2| format!("reconnect failed: {} (original: {})", e2, e))?;
            let msg = conn
                .invoke_request(endpoint, cluster, command, &payload)
                .await
                .map_err(|e| e.to_string())?;
            Ok(format_invoke_result(&msg.tlv))
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
) -> Result<InvokeResultDto, String> {
    let payload = codec::encode_command_json(cluster, command, &args)
        .map_err(|e| format!("encode: {}", e))?;

    let state = state.inner().clone();
    let conn = get_conn(&state, node_id).await?;

    let result = conn
        .invoke_request(endpoint, cluster, command, &payload)
        .await;

    match result {
        Ok(msg) => Ok(format_invoke_result(&msg.tlv)),
        Err(e) => {
            drop(conn);
            AppState::drop_connection(&state, node_id).await;
            let conn = AppState::get_connection(&state, node_id)
                .await
                .map_err(|e2| format!("reconnect failed: {} (original: {})", e2, e))?;
            let msg = conn
                .invoke_request(endpoint, cluster, command, &payload)
                .await
                .map_err(|e| e.to_string())?;
            Ok(format_invoke_result(&msg.tlv))
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
