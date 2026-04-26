use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Mutex, OnceLock,
};
use std::time::{SystemTime, UNIX_EPOCH};

use log::{LevelFilter, Log, Metadata, Record};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub ts_ms: u64,
    pub level: String,
    pub target: String,
    pub message: String,
}

const BUFFER_CAP: usize = 5000;

fn filter_to_u8(f: LevelFilter) -> u8 {
    match f {
        LevelFilter::Off => 0,
        LevelFilter::Error => 1,
        LevelFilter::Warn => 2,
        LevelFilter::Info => 3,
        LevelFilter::Debug => 4,
        LevelFilter::Trace => 5,
    }
}

fn u8_to_filter(v: u8) -> LevelFilter {
    match v {
        0 => LevelFilter::Off,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

struct UiLogger {
    buffer: Mutex<VecDeque<LogEntry>>,
    app: OnceLock<AppHandle>,
    level: AtomicU8,
}

static LOGGER: OnceLock<UiLogger> = OnceLock::new();

impl Log for UiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let max = u8_to_filter(self.level.load(Ordering::Relaxed));
        if max == LevelFilter::Off {
            return false;
        }
        metadata.level() <= max
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let entry = LogEntry {
            ts_ms,
            level: record.level().to_string(),
            target: record.target().to_string(),
            message: format!("{}", record.args()),
        };
        {
            let mut buf = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            if buf.len() >= BUFFER_CAP {
                buf.pop_front();
            }
            buf.push_back(entry.clone());
        }
        if let Some(handle) = self.app.get() {
            let _ = handle.emit("log://entry", &entry);
        }
    }

    fn flush(&self) {}
}

pub fn install() {
    let logger = LOGGER.get_or_init(|| UiLogger {
        buffer: Mutex::new(VecDeque::new()),
        app: OnceLock::new(),
        level: AtomicU8::new(filter_to_u8(LevelFilter::Debug)),
    });
    let _ = log::set_logger(logger);
    log::set_max_level(LevelFilter::Trace);
}

pub fn attach(handle: AppHandle) {
    if let Some(logger) = LOGGER.get() {
        let _ = logger.app.set(handle);
    }
}

pub fn snapshot(limit: Option<usize>) -> Vec<LogEntry> {
    let Some(logger) = LOGGER.get() else {
        return vec![];
    };
    let buf = logger.buffer.lock().unwrap_or_else(|e| e.into_inner());
    let all: Vec<LogEntry> = buf.iter().cloned().collect();
    match limit {
        Some(n) if n < all.len() => all[all.len() - n..].to_vec(),
        _ => all,
    }
}

pub fn clear() {
    if let Some(logger) = LOGGER.get() {
        logger.buffer.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}

pub fn set_level(level: &str) -> Result<(), String> {
    let filter = match level.to_lowercase().as_str() {
        "off" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        other => return Err(format!("unknown level: {}", other)),
    };
    if let Some(logger) = LOGGER.get() {
        logger.level.store(filter_to_u8(filter), Ordering::Relaxed);
    }
    Ok(())
}

pub fn current_level() -> &'static str {
    let v = LOGGER
        .get()
        .map_or(filter_to_u8(LevelFilter::Debug), |l| {
            l.level.load(Ordering::Relaxed)
        });
    match u8_to_filter(v) {
        LevelFilter::Off => "off",
        LevelFilter::Error => "error",
        LevelFilter::Warn => "warn",
        LevelFilter::Info => "info",
        LevelFilter::Debug => "debug",
        LevelFilter::Trace => "trace",
    }
}
