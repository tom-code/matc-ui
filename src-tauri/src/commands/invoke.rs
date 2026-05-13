use std::sync::Arc;

use matc::clusters::{codec, names};
use matc::tlv::TlvItem;
use tauri::State;

use crate::state::AppState;

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

fn cluster_label(cluster: u32) -> String {
    match names::get_cluster_name(cluster) {
        Some(n) => format!("0x{:04x}({})", cluster, n),
        None => format!("0x{:04x}", cluster),
    }
}

fn command_label(cluster: u32, command: u32) -> String {
    let name = codec::get_command_list(cluster)
        .into_iter()
        .find(|(id, _)| *id == command)
        .map(|(_, n)| n);
    match name {
        Some(n) => format!("0x{:02x}({})", command, n),
        None => format!("0x{:02x}", command),
    }
}

fn log_result(tag: &str, node_id: u64, dto: &InvokeResultDto) {
    match dto {
        InvokeResultDto::Status { code } if *code == 0 => {
            log::info!("{}: node={} -> status=0x{:04x} SUCCESS", tag, node_id, code);
        }
        InvokeResultDto::Status { code } => {
            log::warn!("{}: node={} -> status=0x{:04x} FAILURE", tag, node_id, code);
        }
        InvokeResultDto::Data { .. } => {
            log::info!("{}: node={} -> data response", tag, node_id);
        }
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

    let payload_str = if payload.is_empty() {
        "(none)".to_string()
    } else {
        hex::encode(&payload)
    };
    log::info!(
        "invoke: node={} ep={} cluster={} cmd={} payload={}",
        node_id,
        endpoint,
        cluster_label(cluster),
        command_label(cluster, command),
        payload_str,
    );

    let state = state.inner().clone();
    let dto = AppState::with_connection_retry(&state, node_id, |conn| {
        let payload = payload.clone();
        async move {
            let msg = conn.invoke_request(endpoint, cluster, command, &payload).await?;
            Ok(format_invoke_result(&msg.tlv))
        }
    })
    .await
    .map_err(|e| {
        log::error!("invoke failed: node={} err={}", node_id, e);
        e
    })?;
    log_result("invoke", node_id, &dto);
    Ok(dto)
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
    log::info!(
        "invoke_typed: node={} ep={} cluster={} cmd={} args={}",
        node_id,
        endpoint,
        cluster_label(cluster),
        command_label(cluster, command),
        args,
    );

    let payload = codec::encode_command_json(cluster, command, &args).map_err(|e| {
        log::error!("invoke_typed encode failed: node={} err={}", node_id, e);
        format!("encode: {}", e)
    })?;

    let state = state.inner().clone();
    let dto = AppState::with_connection_retry(&state, node_id, |conn| {
        let payload = payload.clone();
        async move {
            let msg = conn.invoke_request(endpoint, cluster, command, &payload).await?;
            Ok(format_invoke_result(&msg.tlv))
        }
    })
    .await
    .map_err(|e| {
        log::error!("invoke_typed failed: node={} err={}", node_id, e);
        e
    })?;
    log_result("invoke_typed", node_id, &dto);
    Ok(dto)
}
