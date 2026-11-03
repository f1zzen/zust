//! zapret на Windows: winws.exe + WinDivert, запуск как служба "zapret"
//! (совместимо со службой из zapret-discord-youtube от flowseal).

use super::{Launch, Zapret};
use crate::Hub;
use crate::logger::info;
use crate::platform::{command, run};
use sha2::{Digest, Sha256};
use std::fs;
use winreg::RegKey;
use winreg::enums::*;

const SERVICE_KEY: &str = r"System\CurrentControlSet\Services\zapret";
const STUN_BIN: &[u8] = include_bytes!("../../../../src-tauri/zapret/bin/stun.bin");

// ENGINE_FILES: [(имя, sha256)] - генерируется build.rs из src-tauri/zapret/bin
include!(concat!(env!("OUT_DIR"), "/engine_hashes.rs"));

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn file_matches(path: &std::path::Path, hash: &str) -> bool {
    fs::read(path).is_ok_and(|d| sha256_hex(&d) == hash)
}

/// Движок запускается от SYSTEM, поэтому перед стартом сверяем его с хешами из сборки.
/// Если файл подменён или испорчен - возвращаем копию из установки Zust.
fn verify_engine(app: &Hub) -> Result<(), String> {
    for (name, hash) in ENGINE_FILES {
        let path = Zapret::path("bin").join(name);
        if file_matches(&path, hash) {
            continue;
        }
        let original = Zapret::storage("bin").join(name);
        if !file_matches(&original, hash) {
            return Err(format!(
                "{name} в установке Z.UST повреждён. Переустановите Z.UST."
            ));
        }
        fs::copy(&original, &path).map_err(|e| format!("{name}: {e}"))?;
        info(
            app,
            &format!("{name} не совпадал с версией из сборки - восстановлен."),
        );
    }
    Ok(())
}

fn reg_value(name: &str) -> Option<String> {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(SERVICE_KEY)
        .and_then(|k| k.get_value(name))
        .ok()
}

pub fn is_running() -> bool {
    command("sc")
        .args(["query", "zapret"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("RUNNING"))
        .unwrap_or(false)
}

pub fn current_strategy() -> Option<String> {
    reg_value("zapret-discord-youtube")
}

pub fn current_ipset() -> Option<String> {
    reg_value("zapret-ipset-config")
}

pub async fn prepare(app: &Hub) -> Result<(), String> {
    let stun = Zapret::path("bin").join("stun.bin");
    if !stun.exists() {
        fs::write(&stun, STUN_BIN).map_err(|e| format!("stun.bin: {e}"))?;
        info(app, "Файл stun.bin восстановлен.");
    }
    verify_engine(app)
}

/// Аргумент для командной строки: значения с пробелами берём в кавычки.
fn quote_arg(a: &str) -> String {
    if !a.contains(' ') {
        return a.to_string();
    }
    match a.split_once('=') {
        Some((k, v)) if k.starts_with("--") => format!("{k}=\"{v}\""),
        _ => format!("\"{a}\""),
    }
}

pub fn start(
    app: &Hub,
    name: &str,
    ipset: &str,
    raw: &str,
    game_filter: bool,
) -> Result<(), String> {
    // проверяем стратегию до остановки работающего zapret
    let Launch { args, .. } = Zapret::build_launch(raw, &Zapret::real_dirs(), ipset, game_filter)?;
    stop(app)?;
    let args_line = args
        .iter()
        .map(|a| quote_arg(a))
        .collect::<Vec<_>>()
        .join(" ");
    info(app, &format!("Аргументы winws: {args_line}"));

    let _ = run(
        "netsh",
        &["interface", "tcp", "set", "global", "timestamps=enabled"],
    );

    // winws.exe запускается службой напрямую, без cmd.exe: никакие символы в аргументах
    // не интерпретируются оболочкой (так же делает service.bat у flowseal).
    let bin = Zapret::path("bin").join("winws.exe");
    let binary_path = format!("\"{}\" {}", bin.display(), args_line);
    // одинарные кавычки внутри строки PowerShell удваиваются
    let ps = format!(
        "New-Service -Name 'zapret' -BinaryPathName '{}' -DisplayName 'zapret' -StartupType Automatic | Out-Null",
        binary_path.replace('\'', "''")
    );
    if !run(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", &ps],
    ) {
        return Err("Не удалось создать службу zapret".into());
    }
    let _ = run("sc", &["start", "zapret"]);

    if let Ok((key, _)) = RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey(SERVICE_KEY) {
        let _ = key.set_value("zapret-discord-youtube", &name);
        let _ = key.set_value("zapret-ipset-config", &ipset);
    }
    Ok(())
}

pub fn stop(app: &Hub) -> Result<(), String> {
    let _ = run("taskkill", &["/F", "/IM", "winws.exe", "/T"]);
    for svc in ["zapret", "WinDivert", "WinDivert14"] {
        let _ = run("net", &["stop", svc]);
        let _ = run("sc", &["delete", svc]);
    }
    info(app, "Службы zapret остановлены и удалены.");
    Ok(())
}
