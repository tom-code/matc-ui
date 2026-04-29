use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

use matc::{controller::Connection, devman::DeviceManager};

pub struct AppState {
    pub devman: Arc<DeviceManager>,
    pub connections: Mutex<HashMap<u64, Arc<Connection>>>,
}

impl AppState {
    pub fn new(devman: DeviceManager) -> Self {
        Self {
            devman: Arc::new(devman),
            connections: Mutex::new(HashMap::new()),
        }
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
        let conn = timeout(Duration::from_secs(12), state.devman.connect(node_id))
            .await
            .map_err(|_| anyhow::anyhow!("connection timed out"))??;
        let conn = Arc::new(conn);
        state.connections.lock().await.insert(node_id, conn.clone());
        Ok(conn)
    }

    /// Connect, falling back to a fresh connection if cache fails.
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
        let conn = timeout(Duration::from_secs(12), state.devman.connect(node_id))
            .await
            .map_err(|_| anyhow::anyhow!("connection timed out"))??;
        let conn = Arc::new(conn);
        state.connections.lock().await.insert(node_id, conn.clone());
        Ok(conn)
    }

    pub async fn drop_connection(state: &Arc<AppState>, node_id: u64) {
        state.connections.lock().await.remove(&node_id);
    }
}
