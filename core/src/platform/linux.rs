use super::unix::home;
pub use super::unix::{command, is_root, kill_process, run, shell_quote};
use super::{PROXY_HOST, PROXY_PORT, find_in_path};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const EXE_SUFFIX: &str = "";

pub fn process_alive(pid: u32, name: &str) -> bool {
    // /proc/<pid>/comm обрезается ядром до 15 символов
    let expected: String = name.chars().take(15).collect();
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|c| c.trim() == expected)
        .unwrap_or(false)
}

/// Выполняет sh-скрипт от root: через pkexec (графический запрос пароля),
/// либо напрямую, если приложение уже запущено от root.
pub fn run_privileged(script: &str) -> Result<String, String> {
    let path = std::env::temp_dir().join(format!("zust-{}.sh", uuid::Uuid::new_v4()));
    fs::write(&path, script).map_err(|e| e.to_string())?;

    let output = if is_root() {
        Command::new("/bin/sh").arg(&path).output()
    } else {
        if find_in_path("pkexec").is_none() {
            let _ = fs::remove_file(&path);
            return Err(
                "Не найден pkexec. Установите polkit (и агент polkit для вашего окружения).".into(),
            );
        }
        Command::new("pkexec").arg("/bin/sh").arg(&path).output()
    };
    let _ = fs::remove_file(&path);

    let out = output.map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    match out.status.code() {
        Some(0) => Ok(stdout),
        Some(126) => Err("Запрос прав администратора отменён".into()),
        Some(127) if stderr.is_empty() => {
            Err("Не удалось получить права администратора (нет агента polkit?)".into())
        }
        _ => Err(if stderr.is_empty() { stdout } else { stderr }),
    }
}

pub fn hosts_path() -> PathBuf {
    PathBuf::from("/etc/hosts")
}

pub fn write_hosts(content: &str) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("zust-hosts-{}", uuid::Uuid::new_v4()));
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    let tmp_q = shell_quote(&tmp.to_string_lossy());
    // cat > сохраняет владельца и права /etc/hosts
    let result = run_privileged(&format!("set -e\ncat {tmp_q} > /etc/hosts\n"));
    let _ = fs::remove_file(&tmp);
    result.map(|_| ())
}

pub fn flush_dns() -> Result<(), String> {
    if find_in_path("resolvectl").is_some() && run("resolvectl", &["flush-caches"]) {
        return Ok(());
    }
    // без systemd-resolved кэша DNS обычно нет - считаем, что чистить нечего
    Ok(())
}

fn gsettings(args: &[&str]) -> bool {
    find_in_path("gsettings").is_some() && run("gsettings", args)
}

fn kde_proxy(enable: bool) -> bool {
    let tool = ["kwriteconfig6", "kwriteconfig5"]
        .into_iter()
        .find(|t| find_in_path(t).is_some());
    let Some(tool) = tool else { return false };
    let base = ["--file", "kioslaverc", "--group", "Proxy Settings", "--key"];
    let socks = format!("socks://{PROXY_HOST} {PROXY_PORT}");
    let ok = run(
        tool,
        &[&base[..], &["ProxyType", if enable { "1" } else { "0" }]].concat(),
    );
    if enable {
        run(tool, &[&base[..], &["socksProxy", &socks]].concat());
    }
    ok
}

pub fn set_system_proxy(enable: bool) -> Result<(), String> {
    let port = PROXY_PORT.to_string();
    let gnome = if enable {
        gsettings(&["set", "org.gnome.system.proxy.socks", "host", PROXY_HOST])
            && gsettings(&["set", "org.gnome.system.proxy.socks", "port", &port])
            && gsettings(&["set", "org.gnome.system.proxy", "mode", "manual"])
    } else {
        gsettings(&["set", "org.gnome.system.proxy", "mode", "none"])
    };
    let kde = kde_proxy(enable);
    if gnome || kde {
        Ok(())
    } else {
        Err("Системный прокси поддерживается в GNOME/KDE. В остальных окружениях укажите SOCKS5 127.0.0.1:9050 вручную.".into())
    }
}

pub fn disable_system_proxy_if_ours() {
    let host = command("gsettings")
        .args(["get", "org.gnome.system.proxy.socks", "host"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(PROXY_HOST))
        .unwrap_or(false);
    if host {
        gsettings(&["set", "org.gnome.system.proxy", "mode", "none"]);
    }
    kde_proxy(false);
}

/// Путь для ярлыков: у AppImage current_exe указывает во временную точку монтирования.
fn launch_exe() -> String {
    std::env::var("APPIMAGE").ok().unwrap_or_else(|| {
        std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "zust".into())
    })
}

/// Экранирование значения Exec= по спецификации .desktop
fn desktop_quote(s: &str) -> String {
    let escaped: String = s
        .chars()
        .flat_map(|c| match c {
            '"' | '`' | '$' | '\\' => vec!['\\', c],
            _ => vec![c],
        })
        .collect();
    format!("\"{escaped}\"")
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let dir = home().join(".config/autostart");
    let file = dir.join("zust.desktop");
    if !enabled {
        let _ = fs::remove_file(file);
        return Ok(());
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Z.UST\nExec={} --autostart\nIcon=zust\nX-GNOME-Autostart-enabled=true\nTerminal=false\n",
        desktop_quote(&launch_exe())
    );
    fs::write(file, entry).map_err(|e| e.to_string())
}

pub fn register_url_protocol() -> Result<(), String> {
    let dir = home().join(".local/share/applications");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName=Z.UST (ссылки zust://)\nExec={} %u\nIcon=zust\nNoDisplay=true\nTerminal=false\nMimeType=x-scheme-handler/zust;\n",
        desktop_quote(&launch_exe())
    );
    fs::write(dir.join("zust-url-handler.desktop"), entry).map_err(|e| e.to_string())?;
    if find_in_path("xdg-mime").is_some() {
        run(
            "xdg-mime",
            &[
                "default",
                "zust-url-handler.desktop",
                "x-scheme-handler/zust",
            ],
        );
    }
    Ok(())
}

pub fn prepare_environment() {
    // Для WebKitGTK с чёрным окном. По умолчанию выключен: без GPU интерфейс идёт на 20-30 FPS.
    if std::env::var_os("ZUST_SAFE_RENDER").is_some_and(|v| v == "1") {
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }
}

// ---------- ярлык в меню приложений (из архива Z.UST в меню сам не попадает) ----------

fn launcher_file() -> PathBuf {
    home().join(".local/share/applications/zust.desktop")
}

fn launcher_icon() -> PathBuf {
    home().join(".local/share/icons/hicolor/128x128/apps/zust.png")
}

/// Можно ли (и нужно ли) создавать ярлык: не нужно, если Zust поставлен пакетом
/// (deb/rpm/AUR кладут свой .desktop в /usr/share/applications).
pub fn launcher_supported() -> bool {
    if std::env::var_os("APPIMAGE").is_some() {
        return true;
    }
    let exe = launch_exe();
    !(exe.starts_with("/usr/") || exe.starts_with("/opt/"))
}

pub fn launcher_installed() -> bool {
    launcher_file().exists()
}

/// Содержимое zust.desktop. StartupWMClass = app id окна (slint::set_xdg_app_id):
/// по нему niri, GNOME и KDE связывают окно с ярлыком и показывают правильный значок.
fn launcher_entry() -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Z.UST\nGenericName=Обход блокировок\n\
         Comment=zapret, TOR и прокси для Telegram\nExec={}\nIcon=zust\nTerminal=false\n\
         Categories=Network;Utility;\nKeywords=zapret;dpi;tor;proxy;discord;youtube;zust;\n\
         StartupWMClass=zust\nStartupNotify=true\n",
        desktop_quote(&launch_exe())
    )
}

/// Создаёт (или обновляет) ярлык и значок. `icon_png` - значок 128×128 (PNG).
pub fn install_launcher(icon_png: &[u8]) -> Result<(), String> {
    let icon = launcher_icon();
    if let Some(dir) = icon.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    fs::write(&icon, icon_png).map_err(|e| e.to_string())?;
    let file = launcher_file();
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let entry = launcher_entry();
    fs::write(&file, entry).map_err(|e| e.to_string())?;
    if find_in_path("update-desktop-database").is_some() {
        let dir = file
            .parent()
            .map(|d| d.to_string_lossy().to_string())
            .unwrap_or_default();
        run("update-desktop-database", &[&dir]);
    }
    Ok(())
}

pub fn remove_launcher() -> Result<(), String> {
    let _ = fs::remove_file(launcher_icon());
    match fs::remove_file(launcher_file()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Если ярлык уже есть, но устарел (Z.UST переместили - например, новая версия
/// AppImage в другой папке, - или изменилось содержимое ярлыка), тихо обновляем его.
pub fn refresh_launcher(icon_png: &[u8]) {
    let Ok(content) = fs::read_to_string(launcher_file()) else {
        return;
    };
    if content != launcher_entry() {
        let _ = install_launcher(icon_png);
    }
}

// ---------- DNS-серверы (Comss.one DNS): через NetworkManager, только до перезагрузки ----------

fn nm_devices() -> Vec<String> {
    command("nmcli")
        .args(["-t", "-f", "DEVICE,STATE,TYPE", "device"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| {
                    let mut it = l.split(':');
                    let (dev, state, kind) = (it.next()?, it.next()?, it.next()?);
                    // только обычные сети: VPN и "externally" (чужие tun) не трогаем
                    (state == "connected" && matches!(kind, "ethernet" | "wifi"))
                        .then(|| dev.to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn dns_uses(ip: &str) -> bool {
    find_in_path("nmcli").is_some()
        && command("nmcli")
            .args(["-t", "-f", "IP4.DNS", "device", "show"])
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(ip))
}

/// `nmcli device modify` меняет только текущее подключение: после `reapply`
/// или переподключения вернутся прежние DNS, поэтому `backup` не нужен.
pub fn set_dns(servers: &[&str], _doh: &str, _backup: &std::path::Path) -> Result<(), String> {
    if find_in_path("nmcli").is_none() {
        return Err(format!(
            "Нужен NetworkManager (nmcli). Или укажите DNS вручную: {}",
            servers.join(", ")
        ));
    }
    let devices = nm_devices();
    if devices.is_empty() {
        return Err("Нет активных подключений".into());
    }
    let list = servers.join(",");
    let ok = devices
        .iter()
        .filter(|dev| {
            run(
                "nmcli",
                &[
                    "device",
                    "modify",
                    dev,
                    "ipv4.dns",
                    &list,
                    "ipv4.ignore-auto-dns",
                    "yes",
                ],
            )
        })
        .count();
    if ok == 0 {
        return Err("nmcli не смог изменить DNS".into());
    }
    flush_dns()
}

pub fn restore_dns(_backup: &std::path::Path, _marker: &str) -> Result<(), String> {
    for dev in nm_devices() {
        run("nmcli", &["device", "reapply", &dev]);
    }
    flush_dns()
}
