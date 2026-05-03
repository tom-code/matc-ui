use std::time::Duration;

use matc::discover;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDeviceDto {
    pub instance: String,
    pub device: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub discriminator: Option<String>,
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleDeviceDto {
    pub discriminator: u16,
    pub vendor_id: u16,
    pub product_id: u16,
    pub cm_flag: bool,
    pub rssi: Option<i16>,
    pub name: Option<String>,
    pub tx_power: Option<i16>,
    pub address: String,
}

#[tauri::command]
pub async fn discover_mdns(timeout_secs: u64) -> Result<Vec<DiscoveredDeviceDto>, String> {
    let timeout = Duration::from_secs(timeout_secs.clamp(2, 30));
    let devices = discover::discover_commissionable(timeout)
        .await
        .map_err(|e| e.to_string())?;

    let result: Vec<DiscoveredDeviceDto> = devices
        .into_iter()
        .map(|d| DiscoveredDeviceDto {
            instance: d.instance,
            device: d.device,
            addresses: d.ips.iter().map(|ip| ip.to_string()).collect(),
            port: d.port.unwrap_or(5540),
            discriminator: d.discriminator,
            vendor_id: d.vendor_id,
            product_id: d.product_id,
            name: d.name,
        })
        .collect();

    Ok(result)
}

#[cfg(feature = "ble")]
#[tauri::command]
pub async fn scan_ble(timeout_secs: u64) -> Result<Vec<BleDeviceDto>, String> {
    let timeout = Duration::from_secs(timeout_secs.clamp(2, 30));
    let devices = matc::ble::scan_commissionable(timeout)
        .await
        .map_err(|e| e.to_string())?;

    Ok(devices
        .into_iter()
        .map(|d| BleDeviceDto {
            discriminator: d.discriminator,
            vendor_id: d.vendor_id,
            product_id: d.product_id,
            cm_flag: d.cm_flag,
            rssi: d.rssi,
            name: d.name,
            tx_power: d.tx_power,
            address: d.address,
        })
        .collect())
}

#[cfg(not(feature = "ble"))]
#[tauri::command]
pub async fn scan_ble(_timeout_secs: u64) -> Result<Vec<BleDeviceDto>, String> {
    Err("BLE support not compiled in".to_string())
}
