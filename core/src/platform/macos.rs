use super::unix::home;
pub use super::unix::{command, is_root, kill_process, run, shell_quote};
use super::{PROXY_HOST, PROXY_PORT};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const EXE_SUFFIX: &str = "";

pub fn process_alive(pid: u32, name: &str) -> bool {
    command("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .map(|o| {
            let comm = String::from_utf8_lossy(&o.stdout);
            Path::new(comm.trim()).file_name() == Some(name.as_ref())
        })
        .unwrap_or(false)
}

/// Строка AppleScript в кавычках.
fn applescript_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Выполняет sh-скрипт от root: системный запрос пароля (или Touch ID) через osascript,
/// либо напрямую, если приложение уже запущено от root.
pub fn run_privileged(script: &str) -> Result<String, String> {
    let path = std::env::temp_dir().join(format!("zust-{}.sh", uuid::Uuid::new_v4()));
    fs::write(&path, script).map_err(|e| e.to_string())?;

    let output = if is_root() {
        Command::new("/bin/sh").arg(&path).output()
    } else {
        let cmd = format!("/bin/sh {}", shell_quote(&path.to_string_lossy()));
        Command::new("/usr/bin/osascript")
            .arg("-e")
            .arg(format!(
                "do shell script {} with prompt {} with administrator privileges",
                applescript_quote(&cmd),
                applescript_quote("Z.UST нужны права администратора")
            ))
            .output()
    };
    let _ = fs::remove_file(&path);

    let out = output.map_err(|e| e.to_string())?;
    // do shell script разделяет строки через \r
    let stdout = String::from_utf8_lossy(&out.stdout)
        .replace('\r', "\n")
        .trim()
        .to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if out.status.success() {
        Ok(stdout)
    } else if stderr.contains("(-128)") {
        Err("Запрос прав администратора отменён".into())
    } else {
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

pub fn hosts_path() -> PathBuf {
    PathBuf::from("/etc/hosts")
}

pub fn write_hosts(content: &str) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("zust-hosts-{}", uuid::Uuid::new_v4()));
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    let tmp_q = shell_quote(&tmp.to_string_lossy());
    // кэш DNS сбрасываем тут же, чтобы не спрашивать пароль второй раз
    let result = run_privileged(&format!(
        "set -e\ncat {tmp_q} > /etc/hosts\n\
         dscacheutil -flushcache || true\nkillall -HUP mDNSResponder 2>/dev/null || true\n"
    ));
    let _ = fs::remove_file(&tmp);
    result.map(|_| ())
}

pub fn flush_dns() -> Result<(), String> {
    run("/usr/bin/dscacheutil", &["-flushcache"]);
    Ok(())
}

// ---------- системный прокси: SOCKS для всех сетевых служб ----------

fn network_services() -> Vec<String> {
    command("/usr/sbin/networksetup")
        .arg("-listallnetworkservices")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .skip(1) // "An asterisk (*) denotes..."
                .filter(|l| !l.is_empty() && !l.starts_with('*'))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn proxy_commands(enable: bool) -> Vec<Vec<String>> {
    let mut cmds = Vec::new();
    for svc in network_services() {
        if enable {
            cmds.push(vec![
                "-setsocksfirewallproxy".into(),
                svc.clone(),
                PROXY_HOST.into(),
                PROXY_PORT.to_string(),
            ]);
        }
        let state = if enable { "on" } else { "off" };
        cmds.push(vec![
            "-setsocksfirewallproxystate".into(),
            svc,
            state.into(),
        ]);
    }
    cmds
}

fn networksetup(args: &[String]) -> bool {
    command("/usr/sbin/networksetup")
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn set_system_proxy(enable: bool) -> Result<(), String> {
    let cmds = proxy_commands(enable);
    if cmds.is_empty() {
        return Err("Не найдено ни одной сетевой службы".into());
    }
    networksetup_all(&cmds)
}

pub fn disable_system_proxy_if_ours() {
    for svc in network_services() {
        let ours = command("/usr/sbin/networksetup")
            .args(["-getsocksfirewallproxy", &svc])
            .output()
            .map(|o| {
                let s = String::from_utf8_lossy(&o.stdout);
                s.contains("Enabled: Yes")
                    && s.contains(&format!("Server: {PROXY_HOST}"))
                    && s.contains(&format!("Port: {PROXY_PORT}"))
            })
            .unwrap_or(false);
        if ours {
            networksetup(&["-setsocksfirewallproxystate".into(), svc, "off".into()]);
        }
    }
}

// ---------- автозапуск: LaunchAgent ----------

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn agent_file() -> PathBuf {
    home().join("Library/LaunchAgents/com.f1zzen.zust.plist")
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let file = agent_file();
    if !enabled {
        let _ = fs::remove_file(file);
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         \x20   <key>Label</key><string>com.f1zzen.zust</string>\n\
         \x20   <key>ProgramArguments</key>\n\
         \x20   <array><string>{}</string><string>--autostart</string></array>\n\
         \x20   <key>RunAtLoad</key><true/>\n\
         </dict>\n</plist>\n",
        xml_escape(&exe.to_string_lossy())
    );
    fs::write(file, plist).map_err(|e| e.to_string())
}

/// Ссылки zust:// регистрирует Info.plist внутри Z.UST.app.
pub fn register_url_protocol() -> Result<(), String> {
    Ok(())
}

/// Из Finder программа запускается с урезанным PATH - добавляем Homebrew (tor, lyrebird, frpc).
pub fn prepare_environment() {
    let mut paths: Vec<PathBuf> = ["/opt/homebrew/bin", "/usr/local/bin"]
        .into_iter()
        .map(PathBuf::from)
        .collect();
    if let Some(old) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&old));
    }
    if let Ok(joined) = std::env::join_paths(paths) {
        unsafe { std::env::set_var("PATH", joined) };
    }
}

// Z.UST.app сам появляется в Launchpad, ярлык не нужен
pub fn launcher_supported() -> bool {
    false
}

pub fn launcher_installed() -> bool {
    false
}

pub fn install_launcher(_icon_png: &[u8]) -> Result<(), String> {
    Ok(())
}

pub fn remove_launcher() -> Result<(), String> {
    Ok(())
}

pub fn refresh_launcher(_icon_png: &[u8]) {}

// ---------- DNS-серверы (Comss.one DNS) ----------

fn dns_servers(svc: &str) -> String {
    command("/usr/sbin/networksetup")
        .args(["-getdnsservers", svc])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

pub fn dns_uses(ip: &str) -> bool {
    network_services()
        .iter()
        .any(|s| dns_servers(s).contains(ip))
}

/// networksetup для каждой команды; без прав - одним запросом пароля от root.
fn networksetup_all(cmds: &[Vec<String>]) -> Result<(), String> {
    if cmds.iter().all(|c| networksetup(c)) {
        return Ok(());
    }
    let script: String = cmds
        .iter()
        .map(|c| {
            let args: Vec<String> = c.iter().map(|a| shell_quote(a)).collect();
            format!("/usr/sbin/networksetup {}\n", args.join(" "))
        })
        .collect();
    run_privileged(&script).map(|_| ())
}

/// Прежние DNS каждой службы ("Empty" - от DHCP) сохраняются в `backup`.
pub fn set_dns(servers: &[&str], _doh: &str, backup: &Path) -> Result<(), String> {
    let mut saved = String::new();
    let mut cmds = Vec::new();
    for svc in network_services() {
        let cur = dns_servers(&svc);
        let old: Vec<&str> = cur
            .lines()
            .filter(|l| l.parse::<std::net::IpAddr>().is_ok())
            .collect();
        let old = if old.is_empty() {
            "Empty".to_string()
        } else {
            old.join(" ")
        };
        saved.push_str(&format!("{svc}\t{old}\n"));
        let mut c = vec!["-setdnsservers".to_string(), svc];
        c.extend(servers.iter().map(|s| s.to_string()));
        cmds.push(c);
    }
    if cmds.is_empty() {
        return Err("Не найдено ни одной сетевой службы".into());
    }
    fs::write(backup, saved).map_err(|e| e.to_string())?;
    networksetup_all(&cmds)?;
    flush_dns()
}

pub fn restore_dns(backup: &Path, marker: &str) -> Result<(), String> {
    let saved = fs::read_to_string(backup).unwrap_or_default();
    let mut cmds = Vec::new();
    for svc in network_services() {
        if !dns_servers(&svc).contains(marker) {
            continue;
        }
        let old = saved
            .lines()
            .find_map(|l| l.strip_prefix(&format!("{svc}\t")))
            .unwrap_or("Empty");
        let mut c = vec!["-setdnsservers".to_string(), svc];
        c.extend(old.split_whitespace().map(str::to_string));
        cmds.push(c);
    }
    networksetup_all(&cmds)?;
    let _ = fs::remove_file(backup);
    flush_dns()
}
