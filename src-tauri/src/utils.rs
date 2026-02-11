use chrono::Local;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;

#[macro_export]
macro_rules! sh {
    ($cmd:expr, $($arg:expr),*) => {{
        use std::os::windows::process::CommandExt;
        std::process::Command::new($cmd)
            .args([$($arg),*])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .status()
    }};
}
fn write_to_log(app: &AppHandle, text: &str) {
    let mut log_path = app.path().executable_dir().unwrap_or_else(|_| {
        let mut p = env::current_exe().unwrap_or_else(|_| env::current_dir().unwrap());
        p.pop();
        p
    });
    log_path.push("latest.log");
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "{}", text);
    }
}

pub fn info(app: &AppHandle, text: &str) {
    let now = Local::now();
    let time_str = now.format("%d.%m %H:%M:%S").to_string();
    let log = format!("[{}] {}", time_str, text);
    write_to_log(app, &log);
    let _ = app.emit("log-event", &log);
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn list_files(path: PathBuf, ext: &str) -> Vec<String> {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(ext))
                .collect()
        })
        .unwrap_or_default()
}
