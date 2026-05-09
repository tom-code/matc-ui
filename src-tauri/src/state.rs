use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{timeout, Duration};

use matc::{controller::Connection, devman::DeviceManager};

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
}

impl AppState {
    pub fn new(devman: DeviceManager) -> Self {
        Self {
            devman: Arc::new(devman),
            connections: Mutex::new(HashMap::new()),
            connect_locks: std::sync::Mutex::new(HashMap::new()),
            device_cache: RwLock::new(HashMap::new()),
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

    /// Connect or reuse cached connection for a device.
    pub async fn get_connection(
        state: &Arc<AppState>,
        node_id: u64,
    ) -> anyhow::Result<Arc<Connection>> {
        {
            let conns = state.connections.lock().await;
            if let Some(conn) = conns.get(&node_id) {
                return Ok(conn.clone());
            }
        }
        let connect_lock = Self::connect_lock_for(state, node_id);
        let _guard = connect_lock.lock().await;
        // Re-check: a concurrent caller may have connected while we waited for the lock.
        {
            let conns = state.connections.lock().await;
            if let Some(conn) = conns.get(&node_id) {
                return Ok(conn.clone());
            }
        }
        let conn = timeout(Duration::from_secs(12), state.devman.connect(node_id))
            .await
            .map_err(|_| anyhow::anyhow!("connection timed out"))??;
        let conn = Arc::new(conn);
        state.connections.lock().await.insert(node_id, conn.clone());
        Ok(conn)
    }

    /// Return cached connection if present; does not attempt a fresh connect.
    /// Callers use this to check for a live session before falling back to get_connection.
    pub async fn get_connection_with_retry(
        state: &Arc<AppState>,
        node_id: u64,
    ) -> anyhow::Result<Arc<Connection>> {
        let cached = {
            let conns = state.connections.lock().await;
            conns.get(&node_id).cloned()
        };
        if let Some(conn) = cached {
            return Ok(conn);
        }
        Self::get_connection(state, node_id).await
    }

    pub async fn drop_connection(state: &Arc<AppState>, node_id: u64) {
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
