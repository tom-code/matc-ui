use std::sync::Arc;

use matc::devman::{DeviceManager, ManagerConfig};
use tauri::Manager;

mod commands;
mod logging;
mod state;

use commands::{
    attributes::{
        read_attribute_tree, read_device_tree, read_endpoint_structure, read_single_attribute,
    },
    clusters::{
        get_command_schema, get_device_type_name, list_cluster_attributes, list_cluster_commands,
    },
    commission::{
        commission_ble, commission_by_address, commission_by_code, open_commissioning_window,
        parse_pairing_code,
    },
    devices::{
        get_device_info, get_device_statuses, list_devices, probe_device, remove_device,
        rename_device,
    },
    discovery::{discover_mdns, scan_ble},
    invoke::{invoke_command, invoke_command_typed},
    logs::{
        clear_logs, get_log_level, get_recent_logs, get_stdout_logging, set_log_level,
        set_stdout_logging,
    },
    write::write_attribute,
};
use state::{AppState, DeviceStatus};

async fn sweep(state: &Arc<AppState>, app: &tauri::AppHandle) {
    let devices = match state.devman.list_devices() {
        Ok(d) => d,
        Err(e) => {
            log::warn!("sweep: list_devices failed: {}", e);
            return;
        }
    };

    let needs_probe: Vec<u64> = {
        let map = state.device_status.lock().await;
        devices
            .iter()
            .filter(|d| {
                matches!(
                    map.get(&d.node_id).map(|e| e.status),
                    None | Some(DeviceStatus::Unknown) | Some(DeviceStatus::Failed)
                )
            })
            .map(|d| d.node_id)
            .collect()
    };

    if needs_probe.is_empty() {
        return;
    }

    log::debug!("sweep: probing {} device(s)", needs_probe.len());
    let sem = Arc::new(tokio::sync::Semaphore::new(4));
    let mut set = tokio::task::JoinSet::new();
    for node_id in needs_probe {
        let sem = sem.clone();
        let state = state.clone();
        let app = app.clone();
        set.spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let _ = probe_device(&state, &app, node_id, false).await;
        });
    }
    while set.join_next().await.is_some() {}
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::install();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("app data dir")
                .join("matc");
            let data_path = data_dir.to_string_lossy().into_owned();

            let devman = tauri::async_runtime::block_on(async {
                match DeviceManager::load(&data_path).await {
                    Ok(dm) => {
                        log::info!("Loaded DeviceManager from {}", data_path);
                        dm
                    }
                    Err(e) => {
                        log::info!(
                            "Creating new DeviceManager at {} (reason: {})",
                            data_path,
                            e
                        );
                        let config = ManagerConfig {
                            fabric_id: 1000,
                            controller_id: 100,
                            local_address: "0.0.0.0:5555".to_string(),
                        };
                        DeviceManager::create(&data_path, config)
                            .await
                            .expect("failed to create DeviceManager")
                    }
                }
            });

            let app_state = Arc::new(AppState::new(devman));
            app.manage(app_state.clone());
            app_state.set_app_handle(app.handle().clone());
            logging::attach(app.handle().clone());

            let st = app_state.clone();
            let ah = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut tick = tokio::time::interval(std::time::Duration::from_secs(10));
                loop {
                    tokio::select! {
                        _ = tick.tick() => sweep(&st, &ah).await,
                        _ = st.probe_kick.notified() => sweep(&st, &ah).await,
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_devices,
            get_device_info,
            get_device_statuses,
            rename_device,
            remove_device,
            commission_by_code,
            commission_by_address,
            commission_ble,
            open_commissioning_window,
            parse_pairing_code,
            read_attribute_tree,
            read_device_tree,
            read_endpoint_structure,
            read_single_attribute,
            invoke_command,
            invoke_command_typed,
            discover_mdns,
            scan_ble,
            get_recent_logs,
            clear_logs,
            set_log_level,
            get_log_level,
            set_stdout_logging,
            get_stdout_logging,
            list_cluster_commands,
            list_cluster_attributes,
            get_command_schema,
            get_device_type_name,
            write_attribute,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
