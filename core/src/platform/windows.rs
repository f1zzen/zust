use super::{PROXY_HOST, PROXY_PORT};
use std::ffi::OsStr;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use winreg::RegKey;
use winreg::enums::*;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const INTERNET_SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
const TASK_NAME: &str = "Zust";

pub const EXE_SUFFIX: &str = ".exe";

pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut c = Command::new(program);
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

/// Запускает команду и возвращает true при нулевом коде выхода.
pub fn run(program: &str, args: &[&str]) -> bool {
    command(program)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn process_alive(pid: u32, name: &str) -> bool {
    let filter = format!("PID eq {pid}");
    command("tasklist")
        .args(["/NH", "/FI", &filter])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .to_lowercase()
                .contains(&format!("{}{}", name.to_lowercase(), EXE_SUFFIX))
        })
        .unwrap_or(false)
}

pub fn kill_process(pid: u32) {
    let pid = pid.to_string();
    let _ = run("taskkill", &["/F", "/T", "/PID", &pid]);
}

pub fn hosts_path() -> PathBuf {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    PathBuf::from(root).join(r"System32\drivers\etc\hosts")
}

pub fn write_hosts(content: &str) -> Result<(), String> {
    std::fs::write(hosts_path(), content).map_err(|e| e.to_string())
}

pub fn flush_dns() -> Result<(), String> {
    if run("ipconfig", &["/flushdns"]) {
        Ok(())
    } else {
        Err("ipconfig /flushdns завершился с ошибкой".into())
    }
}

fn proxy_value() -> String {
    format!("socks={PROXY_HOST}:{PROXY_PORT}")
}

pub fn set_system_proxy(enable: bool) -> Result<(), String> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(INTERNET_SETTINGS, KEY_SET_VALUE)
        .map_err(|e| e.to_string())?;
    if enable {
        key.set_value("ProxyServer", &proxy_value())
            .map_err(|e| e.to_string())?;
        key.set_value("ProxyEnable", &1u32)
            .map_err(|e| e.to_string())?;
    } else {
        key.set_value("ProxyEnable", &0u32)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Выключает системный прокси, только если он указывает на наш TOR -
/// чтобы не ломать пользователю его собственные настройки.
pub fn disable_system_proxy_if_ours() {
    let ours = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(INTERNET_SETTINGS)
        .and_then(|k| k.get_value::<String, _>("ProxyServer"))
        .map(|v| v.contains(&format!("{PROXY_HOST}:{PROXY_PORT}")))
        .unwrap_or(false);
    if ours {
        let _ = set_system_proxy(false);
    }
}

/// Автозапуск через Планировщик задач с наивысшими правами:
/// в отличие от ключа Run, не показывает окно UAC при каждом входе в систему.
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    // старый способ (до 2.1.0) - убираем, чтобы не было двойного запуска
    if let Ok(run_key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_SET_VALUE,
    ) {
        let _ = run_key.delete_value("zust");
    }

    if !enabled {
        let _ = run("schtasks", &["/Delete", "/TN", TASK_NAME, "/F"]);
        return Ok(());
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$a = New-ScheduledTaskAction -Execute '{exe}' -Argument '--autostart'; \
         $t = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME; \
         $s = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero); \
         $p = New-ScheduledTaskPrincipal -UserId $env:USERNAME -RunLevel Highest -LogonType Interactive; \
         Register-ScheduledTask -TaskName '{TASK_NAME}' -Action $a -Trigger $t -Settings $s -Principal $p -Force | Out-Null"
    );
    if run(
        "powershell",
        &["-NoProfile", "-NonInteractive", "-Command", &script],
    ) {
        Ok(())
    } else {
        Err("Не удалось создать задачу автозапуска в Планировщике".into())
    }
}

pub fn register_url_protocol() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let (key, _) = hkcr.create_subkey("zust").map_err(|e| e.to_string())?;
    key.set_value("", &"URL:Z.UST Protocol")
        .map_err(|e| e.to_string())?;
    key.set_value("URL Protocol", &"")
        .map_err(|e| e.to_string())?;
    let (cmd, _) = key
        .create_subkey(r"shell\open\command")
        .map_err(|e| e.to_string())?;
    cmd.set_value("", &format!("\"{}\" \"%1\"", exe.display()))
        .map_err(|e| e.to_string())
}

/// Платформенная настройка окружения до запуска WebView.
pub fn prepare_environment() {
    unsafe {
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--no-sandbox");
        std::env::set_var("WEBVIEW2_IGNORE_WEBVIEW_VERSION_MISMATCH", "1");
    }
}

// ---------- ярлык в меню приложений ----------
// На Windows ярлыки создаёт установщик (NSIS/MSI), здесь ничего не нужно.

pub fn launcher_supported() -> bool {
    false
}

pub fn launcher_installed() -> bool {
    false
}

pub fn install_launcher(_icon_png: &[u8]) -> Result<(), String> {
    Err("На Windows ярлык создаёт установщик".into())
}

pub fn remove_launcher() -> Result<(), String> {
    Ok(())
}

pub fn refresh_launcher(_icon_png: &[u8]) {}

// ---------- DNS-серверы (Comss.one DNS) ----------

const TCPIP_INTERFACES: &str = r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces";

fn powershell(script: &str) -> Result<String, String> {
    let out = command("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Хоть один сетевой адаптер использует DNS `ip`.
pub fn dns_uses(ip: &str) -> bool {
    let Ok(root) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(TCPIP_INTERFACES) else {
        return false;
    };
    root.enum_keys().flatten().any(|k| {
        root.open_subkey(&k)
            .and_then(|s| s.get_value::<String, _>("NameServer"))
            .is_ok_and(|v| v.contains(ip))
    })
}

/// DNS `servers` на всех активных адаптерах; на Windows 11 ещё и шифрование (DoH).
/// Прежние адреса (пусто = от DHCP) сохраняются в `backup` для `restore_dns`.
pub fn set_dns(servers: &[&str], doh: &str, backup: &std::path::Path) -> Result<(), String> {
    let ips = servers
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        "$ErrorActionPreference = 'Stop'; $ips = @({ips}); $saved = @{{}}; \
         foreach ($a in Get-NetAdapter -Physical | Where-Object {{ $_.Status -eq 'Up' }}) {{ \
           $p = 'HKLM:\\{TCPIP_INTERFACES}\\' + $a.InterfaceGuid; \
           $saved[$a.InterfaceGuid] = [string](Get-ItemProperty $p -Name NameServer -ErrorAction SilentlyContinue).NameServer; \
           try {{ Set-DnsClientServerAddress -InterfaceIndex $a.ifIndex -ServerAddresses $ips }} catch {{ $saved.Remove($a.InterfaceGuid) }} }}; \
         if (Get-Command Add-DnsClientDohServerAddress -ErrorAction SilentlyContinue) {{ \
           foreach ($ip in $ips) {{ if (-not (Get-DnsClientDohServerAddress -ServerAddress $ip -ErrorAction SilentlyContinue)) {{ \
             Add-DnsClientDohServerAddress -ServerAddress $ip -DohTemplate '{doh}' -AllowFallbackToUdp $true -AutoUpgrade $true | Out-Null }} }} }}; \
         Clear-DnsClientCache; $saved | ConvertTo-Json -Compress"
    );
    let saved = powershell(&script)?;
    std::fs::write(backup, saved).map_err(|e| e.to_string())
}

/// Вернуть DNS, сохранённые `set_dns`, на адаптерах, где сейчас стоит `marker`.
pub fn restore_dns(backup: &std::path::Path, marker: &str) -> Result<(), String> {
    let saved = std::fs::read_to_string(backup).unwrap_or_else(|_| "{}".into());
    let script = format!(
        "$ErrorActionPreference = 'Stop'; $saved = ConvertFrom-Json '{json}'; \
         foreach ($a in Get-NetAdapter) {{ \
           $p = 'HKLM:\\{TCPIP_INTERFACES}\\' + $a.InterfaceGuid; \
           $cur = [string](Get-ItemProperty $p -Name NameServer -ErrorAction SilentlyContinue).NameServer; \
           if ($cur -notlike '*{marker}*') {{ continue }}; \
           $old = [string]$saved.($a.InterfaceGuid); \
           if ($old) {{ Set-DnsClientServerAddress -InterfaceIndex $a.ifIndex -ServerAddresses ($old -split '[, ]+') }} \
           else {{ Set-DnsClientServerAddress -InterfaceIndex $a.ifIndex -ResetServerAddresses }} }}; \
         Clear-DnsClientCache",
        json = if saved.trim().is_empty() {
            "{}".into()
        } else {
            saved.trim().replace('\'', "''")
        },
    );
    powershell(&script)?;
    let _ = std::fs::remove_file(backup);
    Ok(())
}
