use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use chrono::Local;

const LOG_DIR: &str = "logs";

/// Инициализация папки для логов
pub fn init() -> std::io::Result<()> {
    if !Path::new(LOG_DIR).exists() {
        fs::create_dir(LOG_DIR)?;
    }
    Ok(())
}

/// Получить имя файла лога на текущую дату
fn log_filename() -> String {
    let now = Local::now();
    format!("{}/side_{}.sdlog", LOG_DIR, now.format("%Y%m%d"))
}

/// Записать сообщение в лог с уровнем
fn log(level: &str, message: &str) {
    let filename = log_filename();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filename)
        .unwrap_or_else(|_| File::create(&filename).unwrap());
    
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    writeln!(file, "[{}] [{}] {}", timestamp, level, message).ok();
}

/// Логирование информационных сообщений
pub fn info(message: &str) {
    log("INFO", message);
}

/// Логирование ошибок
pub fn error(message: &str) {
    log("ERROR", message);
}

/// Логирование предупреждений
pub fn warn(message: &str) {
    log("WARN", message);
}

/// Логирование отладочных сообщений (только в debug-режиме)
pub fn debug(message: &str) {
    if cfg!(debug_assertions) {
        log("DEBUG", message);
    }
}