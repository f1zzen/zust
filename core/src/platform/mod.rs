//! Всё, что зависит от ОС.

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
compile_error!("Zust поддерживает только Windows, Linux и macOS");

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(unix)]
mod unix;

use std::fs;
use std::path::{Path, PathBuf};

pub const PROXY_HOST: &str = "127.0.0.1";
pub const PROXY_PORT: u16 = 9050;

/// Ищет исполняемый файл в PATH.
#[allow(dead_code)]
pub fn find_in_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

/// PID-файл: "pid" или "pid extra". Нужен, чтобы управлять только своими
/// процессами, а не убивать все tor.exe/frpc.exe в системе.
pub struct PidFile;

impl PidFile {
    pub fn write(path: &Path, pid: u32, extra: &str) {
        let _ = fs::write(path, format!("{pid} {extra}").trim());
    }

    pub fn read(path: &Path) -> Option<(u32, String)> {
        let text = fs::read_to_string(path).ok()?;
        let mut parts = text.split_whitespace();
        let pid = parts.next()?.parse().ok()?;
        Some((pid, parts.collect::<Vec<_>>().join(" ")))
    }

    /// PID, если процесс с таким именем ещё жив.
    pub fn alive(path: &Path, name: &str) -> Option<(u32, String)> {
        let (pid, extra) = Self::read(path)?;
        process_alive(pid, name).then_some((pid, extra))
    }

    /// Завершает процесс из PID-файла (если он жив) и удаляет файл.
    pub fn kill(path: &Path, name: &str) {
        if let Some((pid, _)) = Self::alive(path, name) {
            kill_process(pid);
        }
        let _ = fs::remove_file(path);
    }
}

/// Windows: перезапуск с правами администратора через UAC. Linux и macOS: ничего (root - по месту, через pkexec или osascript).
pub fn ensure_admin() {
    #[cfg(windows)]
    if !is_elevated::is_elevated() {
        if let Ok(exe) = std::env::current_exe() {
            let _ = runas::Command::new(exe)
                .args(&std::env::args().skip(1).collect::<Vec<_>>())
                .status();
        }
        std::process::exit(0);
    }
}
