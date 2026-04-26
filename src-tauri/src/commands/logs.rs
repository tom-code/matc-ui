use crate::logging::{self, LogEntry};

#[tauri::command]
pub fn get_recent_logs(limit: Option<usize>) -> Vec<LogEntry> {
    logging::snapshot(limit)
}

#[tauri::command]
pub fn clear_logs() {
    logging::clear();
}

#[tauri::command]
pub fn set_log_level(level: String) -> Result<(), String> {
    logging::set_level(&level)
}

#[tauri::command]
pub fn get_log_level() -> String {
    logging::current_level().to_string()
}
