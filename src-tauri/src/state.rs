use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, OnceLock};
use tokio::sync::{Mutex, Notify, RwLock};
use tokio::time::{timeout, Duration};

use matc::{controller::Connection, devman::DeviceManager};
use tauri::Emitter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceStatus {
    Unknown,
    Checking,
    Connected,
    Failed,
    Removed,
}

pub struct StatusEntry {
    pub status: DeviceStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceStatusDto {
    pub node_id: u64,
    pub status: DeviceStatus,
    pub error: Option<String>,
    // Serialized DeviceInfoDto, present only when status == Connected and info is known.
    pub info: Option<serde_json::Value>,
}

pub fn emit_device_status(app: &tauri::AppHandle, dto: DeviceStatusDto) {
    let _ = app.emit("device://status", dto);
}

#[derive(Default)]
struct DeviceCache {
    info: Option<serde_json::Value>,
    structure: Option<serde_json::Value>,
    attributes: Option<serde_json::Value>,
    device_tree: Option<serde_json::Value>,
}

pub struct AppState {
    pub devman: Arc<DeviceManager>,
    pub connections: Mutex<HashMap<u64, Arc<Connection>>>,
    // Serializes concurrent connect() calls for the same node_id.
    // Without this, two concurrent callers both call transport.create_connection()
    // for the same address; the second replaces the first's receive channel, causing
    // the first's retransmit loop to busy-spin on immediate Err("eof") for up to
    // MAX_RETRANSMIT_TIME (10s) at 100% CPU.
    connect_locks: std::sync::Mutex<HashMap<u64, Arc<Mutex<()>>>>,
    device_cache: RwLock<HashMap<u64, DeviceCache>>,
    pub device_status: Mutex<HashMap<u64, StatusEntry>>,
    // Serializes concurrent probe_device() calls for the same node_id so that
    // status transitions don't race between the background sweep and manual probes.
    probe_locks: std::sync::Mutex<HashMap<u64, Arc<Mutex<()>>>>,
    pub probe_kick: Notify,
    app_handle: OnceLock<tauri::AppHandle>,
}

/// Heuristic: does this error signal a defunct CASE session (transport timeout,
/// crypto failure) requiring a reconnect? Or is it a protocol/application-level
/// rejection from the device (IM status code, unsupported attribute, etc.)
/// where the session is healthy and no reconnect is needed?
///
/// rust-matc returns anyhow::Error with no structural variants, so we match on
/// the rendered message. Conservative whitelist: unknown errors are treated as
/// protocol-level (propagate, do not reconnect) to avoid spurious reauths.
fn is_session_defunct(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        if cause.downcast_ref::<std::io::Error>().is_some() {
            return true;
        }
        let s = cause.to_string();
        // Retransmit loop gave up (active_connection.rs in rust-matc).
        s.contains("channel closed while waiting for response")
            // Crypto / session-state failures (session.rs in rust-matc).
            || s.contains("decrypt")
            || s.contains("decode_message")
            || s.contains("encode_message")
    })
}

impl AppState {
    pub fn new(devman: DeviceManager) -> Self {
        Self {
            devman: Arc::new(devman),
            connections: Mutex::new(HashMap::new()),
            connect_locks: std::sync::Mutex::new(HashMap::new()),
            device_cache: RwLock::new(HashMap::new()),
            device_status: Mutex::new(HashMap::new()),
            probe_locks: std::sync::Mutex::new(HashMap::new()),
            probe_kick: Notify::new(),
            app_handle: OnceLock::new(),
        }
    }

    pub fn set_app_handle(&self, h: tauri::AppHandle) {
        let _ = self.app_handle.set(h);
    }

    pub async fn mark_failed_and_emit(&self, node_id: u64, error: String) {
        let changed = self
            .set_status(node_id, DeviceStatus::Failed, Some(error.clone()))
            .await;
        // Clear the cached DeviceInfoDto so the next sweep probe cannot shortcut
        // to a stale cache hit. do_probe returns Ok(cached) when info is present,
        // bypassing any actual network read.
        if let Some(e) = self.device_cache.write().await.get_mut(&node_id) {
            e.info = None;
        }
        if changed {
            log::warn!("status: node={} -> failed (retry): {}", node_id, error);
            if let Some(app) = self.app_handle.get() {
                emit_device_status(
                    app,
                    DeviceStatusDto {
                        node_id,
                        status: DeviceStatus::Failed,
                        error: Some(error),
                        info: None,
                    },
                );
            }
        }
    }

    pub async fn mark_connected_and_emit(&self, node_id: u64) {
        let changed = self
            .set_status(node_id, DeviceStatus::Connected, None)
            .await;
        if changed {
            log::info!("status: node={} -> connected (recovery)", node_id);
            if let Some(app) = self.app_handle.get() {
                emit_device_status(
                    app,
                    DeviceStatusDto {
                        node_id,
                        status: DeviceStatus::Connected,
                        error: None,
                        info: None,
                    },
                );
            }
        }
    }

    fn connect_lock_for(state: &Arc<AppState>, node_id: u64) -> Arc<Mutex<()>> {
        state
            .connect_locks
            .lock()
            .unwrap()
            .entry(node_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    pub fn probe_lock_for(state: &Arc<AppState>, node_id: u64) -> Arc<Mutex<()>> {
        state
            .probe_locks
            .lock()
            .unwrap()
            .entry(node_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Update the stored status for a node; returns true if status or error changed.
    pub async fn set_status(
        &self,
        node_id: u64,
        status: DeviceStatus,
        error: Option<String>,
    ) -> bool {
        let mut map = self.device_status.lock().await;
        let entry = map.entry(node_id).or_insert_with(|| StatusEntry {
            status: DeviceStatus::Unknown,
            error: None,
        });
        if entry.status == status && entry.error == error {
            return false;
        }
        entry.status = status;
        entry.error = error;
        true
    }

    pub async fn remove_status(&self, node_id: u64) {
        self.device_status.lock().await.remove(&node_id);
    }

    /// Connect or reuse cached connection for a device.
    pub async fn get_connection(
        state: &Arc<AppState>,
        node_id: u64,
    ) -> anyhow::Result<Arc<Connection>> {
        {
            let conns = state.connections.lock().await;
            if let Some(conn) = conns.get(&node_id) {
                log::debug!("conn: cache hit node={}", node_id);
                return Ok(conn.clone());
            }
        }
        let connect_lock = Self::connect_lock_for(state, node_id);
        let _guard = connect_lock.lock().await;
        // Re-check: a concurrent caller may have connected while we waited for the lock.
        {
            let conns = state.connections.lock().await;
            if let Some(conn) = conns.get(&node_id) {
                log::debug!("conn: cache hit (post-lock) node={}", node_id);
                return Ok(conn.clone());
            }
        }
        log::debug!("conn: opening CASE session node={}", node_id);
        // Budget for full connect: 5 BUSY retries * up to 60 s each + handshake overhead.
        let conn = timeout(Duration::from_secs(360), state.devman.connect(node_id))
            .await
            .map_err(|_| anyhow::anyhow!("connection timed out"))??;
        let conn = Arc::new(conn);
        state.connections.lock().await.insert(node_id, conn.clone());
        log::info!("conn: established node={}", node_id);
        Ok(conn)
    }

    /// Run an async operation against a device's CASE session. On error,
    /// classify it: if the error signals a defunct session (transport timeout,
    /// crypto failure) drop the cached connection, reconnect, and retry once.
    /// If the error is a protocol/application-level rejection from the device
    /// (IM status code, unsupported attribute, etc.) propagate it as-is --
    /// the session is healthy, no reconnect needed.
    pub async fn with_connection_retry<T, F, Fut>(
        state: &Arc<AppState>,
        node_id: u64,
        f: F,
    ) -> Result<T, String>
    where
        F: Fn(Arc<Connection>) -> Fut,
        Fut: Future<Output = anyhow::Result<T>>,
    {
        let conn = Self::get_connection(state, node_id)
            .await
            .map_err(|e| e.to_string())?;
        match f(conn.clone()).await {
            Ok(v) => Ok(v),
            Err(e) => {
                log::debug!("op failed: node={} err={}", node_id, e);
                if !is_session_defunct(&e) {
                    return Err(e.to_string());
                }
                log::warn!("session defunct, reauthing: node={} err={}", node_id, e);
                // Mark failed immediately so the UI stops showing Connected while
                // we attempt recovery (which may take up to the 360 s reconnect budget).
                state
                    .mark_failed_and_emit(node_id, format!("session defunct: {}", e))
                    .await;

                // Serialize reauth with connect so concurrent callers don't race.
                let connect_lock = Self::connect_lock_for(state, node_id);
                let _guard = connect_lock.lock().await;

                // Try in-place reauth: keeps the transport channel registered, no drop gap,
                // no channel-replace race. auth_sigma_with_busy_retry handles BUSY backoff.
                if state.devman.reauth(&conn, node_id).await.is_ok() {
                    log::debug!("reauth succeeded, retrying: node={}", node_id);
                    drop(_guard);
                    let result = f(conn).await.map_err(|e| e.to_string());
                    if result.is_ok() {
                        log::debug!("retry: succeeded node={}", node_id);
                        state.mark_connected_and_emit(node_id).await;
                    }
                    return result;
                }

                // In-place reauth failed (e.g. device changed IP). Fall back to full reconnect.
                log::warn!("reauth failed, dropping and reconnecting: node={}", node_id);
                drop(conn);
                drop(_guard);
                Self::drop_connection(state, node_id).await;
                let conn = Self::get_connection(state, node_id)
                    .await
                    .map_err(|e2| format!("reconnect failed: {} (original: {})", e2, e))?;
                let result = f(conn).await.map_err(|e| e.to_string());
                if result.is_ok() {
                    log::debug!("retry: succeeded node={}", node_id);
                    state.mark_connected_and_emit(node_id).await;
                }
                result
            }
        }
    }

    pub async fn drop_connection(state: &Arc<AppState>, node_id: u64) {
        log::debug!("conn: dropped node={}", node_id);
        state.connections.lock().await.remove(&node_id);
    }

    /// Clear all cached data for a node (call on remove_device or after a connection error).
    pub async fn drop_cache(state: &Arc<AppState>, node_id: u64) {
        state.device_cache.write().await.remove(&node_id);
    }

    pub async fn cache_get_info<T: for<'de> serde::Deserialize<'de>>(
        &self,
        node_id: u64,
    ) -> Option<T> {
        let guard = self.device_cache.read().await;
        let entry = guard.get(&node_id)?;
        serde_json::from_value(entry.info.clone()?).ok()
    }

    pub async fn cache_set_info<T: serde::Serialize>(&self, node_id: u64, val: &T) {
        if let Ok(v) = serde_json::to_value(val) {
            self.device_cache
                .write()
                .await
                .entry(node_id)
                .or_default()
                .info = Some(v);
        }
    }

    pub async fn cache_get_structure<T: for<'de> serde::Deserialize<'de>>(
        &self,
        node_id: u64,
    ) -> Option<T> {
        let guard = self.device_cache.read().await;
        let entry = guard.get(&node_id)?;
        serde_json::from_value(entry.structure.clone()?).ok()
    }

    pub async fn cache_set_structure<T: serde::Serialize>(&self, node_id: u64, val: &T) {
        if let Ok(v) = serde_json::to_value(val) {
            self.device_cache
                .write()
                .await
                .entry(node_id)
                .or_default()
                .structure = Some(v);
        }
    }

    pub async fn cache_get_attributes<T: for<'de> serde::Deserialize<'de>>(
        &self,
        node_id: u64,
    ) -> Option<T> {
        let guard = self.device_cache.read().await;
        let entry = guard.get(&node_id)?;
        serde_json::from_value(entry.attributes.clone()?).ok()
    }

    pub async fn cache_set_attributes<T: serde::Serialize>(&self, node_id: u64, val: &T) {
        if let Ok(v) = serde_json::to_value(val) {
            self.device_cache
                .write()
                .await
                .entry(node_id)
                .or_default()
                .attributes = Some(v);
        }
    }

    pub async fn cache_get_device_tree<T: for<'de> serde::Deserialize<'de>>(
        &self,
        node_id: u64,
    ) -> Option<T> {
        let guard = self.device_cache.read().await;
        let entry = guard.get(&node_id)?;
        serde_json::from_value(entry.device_tree.clone()?).ok()
    }

    pub async fn cache_set_device_tree<T: serde::Serialize>(&self, node_id: u64, val: &T) {
        if let Ok(v) = serde_json::to_value(val) {
            self.device_cache
                .write()
                .await
                .entry(node_id)
                .or_default()
                .device_tree = Some(v);
        }
    }
}
