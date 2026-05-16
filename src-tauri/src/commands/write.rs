use std::sync::Arc;

use matc::clusters::{codec, names};
use matc::tlv::TlvBuffer;
use tauri::State;

use crate::state::AppState;

fn cluster_label(cluster: u32) -> String {
    match names::get_cluster_name(cluster) {
        Some(n) => format!("0x{:04x}({})", cluster, n),
        None => format!("0x{:04x}", cluster),
    }
}

fn attr_label(cluster: u32, attr_id: u32) -> String {
    let name = codec::get_attribute_list(cluster)
        .into_iter()
        .find(|(id, _)| *id == attr_id)
        .map(|(_, n)| n);
    match name {
        Some(n) => format!("0x{:04x}({})", attr_id, n),
        None => format!("0x{:04x}", attr_id),
    }
}

#[tauri::command]
pub async fn write_attribute(
    state: State<'_, Arc<AppState>>,
    node_id: u64,
    endpoint: u16,
    cluster: u32,
    attr_id: u32,
    value_type: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut buf = TlvBuffer::new();
    match value_type.as_str() {
        "string" => {
            let s = value.as_str().ok_or("expected string value")?;
            buf.write_string(2, s).map_err(|e| e.to_string())?;
        }
        "integer" => {
            let n: i128 = if let Some(v) = value.as_i64() {
                v as i128
            } else if let Some(v) = value.as_u64() {
                v as i128
            } else if let Some(s) = value.as_str() {
                s.parse::<i128>()
                    .map_err(|_| "invalid integer string".to_string())?
            } else {
                return Err("expected integer value".to_string());
            };
            if n >= 0 {
                let u = n as u128;
                if u <= u8::MAX as u128 {
                    buf.write_uint8(2, u as u8).map_err(|e| e.to_string())?;
                } else if u <= u16::MAX as u128 {
                    buf.write_uint16(2, u as u16).map_err(|e| e.to_string())?;
                } else if u <= u32::MAX as u128 {
                    buf.write_uint32(2, u as u32).map_err(|e| e.to_string())?;
                } else if u <= u64::MAX as u128 {
                    buf.write_uint64(2, u as u64).map_err(|e| e.to_string())?;
                } else {
                    return Err("integer value out of range".to_string());
                }
            } else if n >= i8::MIN as i128 {
                buf.write_int8(2, n as i8).map_err(|e| e.to_string())?;
            } else if n >= i16::MIN as i128 {
                buf.write_int16(2, n as i16).map_err(|e| e.to_string())?;
            } else if n >= i32::MIN as i128 {
                buf.write_int32(2, n as i32).map_err(|e| e.to_string())?;
            } else if n >= i64::MIN as i128 {
                buf.write_int64(2, n as i64).map_err(|e| e.to_string())?;
            } else {
                return Err("integer value out of range".to_string());
            }
        }
        other => return Err(format!("unsupported value_type: {}", other)),
    }

    log::info!(
        "write: node={} ep={} cluster={} attr={} type={} value={}",
        node_id,
        endpoint,
        cluster_label(cluster),
        attr_label(cluster, attr_id),
        value_type,
        value,
    );

    let state = state.inner().clone();
    AppState::with_connection_retry(&state, node_id, |conn| {
        let data = buf.data.clone();
        async move {
            conn.write_request(endpoint, cluster, attr_id, &data)
                .await?;
            Ok(())
        }
    })
    .await
    .map(|()| {
        log::info!(
            "write ok: node={} ep={} cluster={} attr={}",
            node_id,
            endpoint,
            cluster_label(cluster),
            attr_label(cluster, attr_id),
        );
    })
    .map_err(|e| {
        log::error!(
            "write failed: node={} ep={} cluster={} attr={} err={}",
            node_id,
            endpoint,
            cluster_label(cluster),
            attr_label(cluster, attr_id),
            e,
        );
        e
    })
}
