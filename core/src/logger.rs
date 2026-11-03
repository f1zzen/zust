use crate::Hub;
use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn log_path() -> PathBuf {
    crate::paths::data_dir().join("latest.log")
}

/// Начинает новый лог-файл при запуске приложения.
pub fn init() {
    let _ = fs::write(log_path(), "");
}

/// Пишет строку в latest.log и отправляет её в интерфейс (вкладка "Логи").
pub fn info(app: &Hub, text: &str) {
    let line = format!("[{}] {}", Local::now().format("%d.%m %H:%M:%S"), text);
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = writeln!(f, "{line}");
    }
    let _ = app.emit("log-event", &line);
}
