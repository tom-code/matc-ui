use std::sync::Arc;

use matc::devman::{DeviceManager, ManagerConfig};
use tauri::Manager;

mod commands;
mod logging;
mod state;

use commands::{
    attributes::{read_attribute_tree, read_single_attribute},
    commission::{commission_ble, commission_by_address, commission_by_code},
    devices::{check_reachability, get_device_info, list_devices, remove_device, rename_device},
    discovery::{discover_mdns, scan_ble},
    invoke::invoke_command,
    logs::{clear_logs, get_log_level, get_recent_logs, set_log_level},
};
use state::AppState;

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
                        log::info!("Creating new DeviceManager at {} (reason: {})", data_path, e);
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
            app.manage(app_state);
            logging::attach(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_devices,
            get_device_info,
            rename_device,
            remove_device,
            check_reachability,
            commission_by_code,
            commission_by_address,
            commission_ble,
            read_attribute_tree,
            read_single_attribute,
            invoke_command,
            discover_mdns,
            scan_ble,
            get_recent_logs,
            clear_logs,
            set_log_level,
            get_log_level,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
