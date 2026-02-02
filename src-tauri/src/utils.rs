//use process_list::for_each_process;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use winreg::RegKey;
use winreg::enums::*;

// NO_WINDOW flag
const CREATE_NO_WINDOW: u32 = 0x08000000;
const HKLM_PATH: &str = r"System\CurrentControlSet\Services\zapret";
const CONFIG_EXTENSION: &str = ".zapret";
const HOSTS_URL: &str =
    "https://raw.githubusercontent.com/ImMALWARE/dns.malw.link/refs/heads/master/hosts";

macro_rules! sh {
    ($cmd:expr, $($arg:expr),*) => {
        Command::new($cmd)
            .args([$($arg),*])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
    };
}

pub struct Hosts;

impl Hosts {
    // малв, я надеюсь ты это менять не будешь. 🥹
    const HOSTS_START_MARKER: &str = "### dns.malw.link: hosts file";
    const HOSTS_MARKER_END: &str = "### dns.malw.link: end hosts file";

    fn get_path() -> PathBuf {
        let sys_root = env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
        Path::new(&sys_root).join(r"System32\drivers\etc\hosts")
    }

    pub async fn fetch(app: &AppHandle) -> Result<String, String> {
        let response = reqwest::get(HOSTS_URL).await.map_err(|e| {
            let err_msg = format!("err internet: {}", e);
            info(app, &err_msg);
            err_msg
        })?;

        let text = response.text().await.map_err(|e| {
            let err_msg = format!("err read: {}", e);
            info(app, &err_msg);
            err_msg
        })?;

        Ok(text)
    }

    pub fn clean(content: &str) -> String {
        let pattern = format!(
            r"(?s){}.*?{}",
            regex::escape(Self::HOSTS_START_MARKER),
            regex::escape(Self::HOSTS_MARKER_END)
        );
        let re = Regex::new(&pattern).unwrap();
        re.replace_all(content, "").trim().to_string()
    }

    pub fn write(app: &AppHandle, new_data: &str) -> Result<(), String> {
        let path = Self::get_path();
        let current_content = fs::read_to_string(&path).unwrap_or_default();
        if current_content.contains(Self::HOSTS_START_MARKER) {
            info(app, "hosts block already exists, updating...");
        } else {
            info(app, "hosts block not found, creating new...");
        }
        let cleaned_base = Self::clean(&current_content);
        let final_content = format!(
            "{}\n{}\n{}\n\n{}",
            Self::HOSTS_START_MARKER,
            new_data.trim(),
            Self::HOSTS_MARKER_END,
            cleaned_base
        );
        fs::write(&path, final_content).map_err(|e| format!("err write: {}", e))?;

        let result = sh!("ipconfig", "/flushdns");
        info(app, &format!("hosts success {:?}", result));
        Ok(())
    }

    pub fn get_update_date(content: &str) -> String {
        content
            .lines()
            .find(|line| line.contains("Последнее обновление"))
            .map(|line| {
                line.trim_start_matches('#')
                    .replace("Последнее обновление:", "")
                    .trim()
                    .to_string()
            })
            .unwrap_or_else(|| "Неизвестно".to_string())
    }

    pub fn get_categories(content: &str) -> HashMap<String, Vec<String>> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        let mut inside_block = false;
        let mut current_category = "Базовая".to_string();

        for line in content.lines() {
            let line = line.trim();
            if line == Self::HOSTS_START_MARKER {
                inside_block = true;
                continue;
            }
            if line == Self::HOSTS_MARKER_END {
                break;
            }

            if inside_block && !line.is_empty() {
                if line.starts_with('#') {
                    let comment = line.trim_start_matches('#').trim();
                    if comment.contains("Последнее обновление") || comment.is_empty()
                    {
                        continue;
                    }

                    current_category = if comment.to_lowercase().starts_with("базов") {
                        "Базовая".to_string()
                    } else {
                        comment.to_string()
                    };
                    continue;
                }
                map.entry(current_category.clone())
                    .or_default()
                    .push(line.to_string());
            }
        }
        map
    }
}

fn write_to_file(app: &AppHandle, text: &str) {
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

pub fn get_custom_ipset_files(app: &AppHandle) -> Vec<String> {
    let path = zapret_path(app, "ipset-configs");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    list_files(path, ".txt")
}

pub fn game_filter_toggle(enabled: bool, app: &AppHandle) -> Result<(), String> {
    let path = zapret_path(app, "utils/game_filter.enabled");

    let result = if enabled {
        fs::File::create(path).map(|_| ())
    } else {
        if path.exists() {
            fs::remove_file(path)
        } else {
            Ok(())
        }
    };
    result.map_err(|e| e.to_string())
}

pub fn create_strategy_file(name: String, content: &str, app: &AppHandle) -> Result<(), String> {
    let path = zapret_path(app, "strategies");
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    let file_path = path.join(name);
    fs::write(file_path, content.as_bytes()).map_err(|e| e.to_string())
}

pub async fn convert_multiple_bats(
    app: &tauri::AppHandle,
    paths: Vec<String>,
) -> Result<(), String> {
    for path_str in paths {
        let path = Path::new(&path_str);
        let content =
            fs::read_to_string(path).map_err(|e| format!("Ошибка чтения {}: {}", path_str, e))?;
        if let Some(start_idx) = content.find("winws.exe\"") {
            let args = &content[start_idx + 11..].trim();
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            let new_file_name = format!("{}{CONFIG_EXTENSION}", file_name);
            create_strategy_file(new_file_name, args, app)?;
        }
    }
    Ok(())
}

pub fn info(app: &AppHandle, text: &str) {
    let log = format!("[ZUST] {}", text);
    write_to_file(app, &log);
    let _ = app.emit("log-event", &log);
}

pub fn zapret_path(app: &AppHandle, sub: &str) -> PathBuf {
    let path = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("failed to get project root")
            .join("zapret")
    } else {
        let res_dir = app
            .path()
            .resource_dir()
            .expect("failed to get resource dir");
        let direct = res_dir.join("zapret");
        let up_aliased = res_dir.join("_up_").join("zapret");
        if direct.exists() { direct } else { up_aliased }
    };
    let final_path = path.join(sub);
    let path_str = final_path.to_string_lossy();
    let clean_path = if path_str.starts_with(r"\\?\") {
        path_str.trim_start_matches(r"\\?\").to_string()
    } else {
        path_str.into_owned()
    };

    PathBuf::from(clean_path)
}

fn list_files(path: PathBuf, ext: &str) -> Vec<String> {
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

pub fn get_list_strategies(app: &AppHandle) -> Vec<String> {
    list_files(zapret_path(app, "strategies"), CONFIG_EXTENSION)
}

pub fn get_strategy() -> String {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(HKLM_PATH)
        .and_then(|k| k.get_value("zapret-discord-youtube"))
        .unwrap_or_else(|_| "Отсутствует".to_string())
}

pub fn get_files_strategies(app: &AppHandle) -> Vec<String> {
    fs::read_dir(zapret_path(app, "lists"))
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".txt") && !n.contains("-hide"))
                .collect()
        })
        .unwrap_or_default()
}

pub fn build_full_args(app: &AppHandle, raw: &str, custom_ipset: Option<String>) -> String {
    let lists_dir = zapret_path(app, "lists");
    let game_filter_enabled = zapret_path(app, "utils/game_filter.enabled").exists();
    info(
        app,
        &format!(
            "Проверка фильтра: {}",
            if game_filter_enabled {
                "Игровой"
            } else {
                "Обычный"
            }
        ),
    );
    let filter = if game_filter_enabled {
        "1024-65535"
    } else {
        "12"
    };

    let mut args = raw
        .replace("%GameFilter%", filter)
        .replace("%BIN%", &format!("{}\\", zapret_path(app, "bin").display()));

    let ipset_path = match custom_ipset.as_deref() {
        Some("none") => lists_dir.join("ipset-none-hide.txt"),
        Some("any") => lists_dir.join("ipset-any-hide.txt"),
        Some(file) => {
            let custom = zapret_path(app, "ipset-configs").join(file);
            if custom.exists() {
                custom
            } else {
                lists_dir.join(file)
            }
        }
        _ => lists_dir.join("ipset-all.txt"),
    };
    info(app, &format!("%IPSET%: {}", ipset_path.display()));

    args = args.replace("%IPSET%", &format!("--ipset=\"{}\"", ipset_path.display()));

    if args.contains("%LISTS%") {
        info(app, &format!("%LISTS%: {}", lists_dir.display()));
        args = args.replace("%LISTS%", &format!("{}\\", lists_dir.display()));
    } else if let Ok(entries) = fs::read_dir(&lists_dir) {
        info(app, &format!("%LISTS%: fallback hostlist"));
        let hosts: String = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("list-") && n.ends_with(".txt") && !n.contains("excluded"))
            .map(|n| format!(" --hostlist=\"{}\"", lists_dir.join(n).display()))
            .collect();
        args.push_str(&hosts);
    }
    args.trim().to_string()
}

pub fn start_service(app: &AppHandle, index: i32, ipset_config: Option<String>) {
    stop_service(app);
    let list = get_list_strategies(app);
    let name = match list.get((index - 1).max(0) as usize) {
        Some(n) => n,
        None => {
            info(app, "Ошибка: Стратегия не найдена.");
            return;
        }
    };

    let strategy_raw =
        fs::read_to_string(zapret_path(app, "strategies").join(name)).unwrap_or_default();
    let final_args = build_full_args(app, strategy_raw.trim(), ipset_config);
    let bin = zapret_path(app, "bin/winws.exe");

    let _ = sh!(
        "netsh",
        "interface",
        "tcp",
        "set",
        "global",
        "timestamps=enabled"
    );

    let cmd = format!(
        "New-Service -Name 'zapret' -BinaryPathName '\"{}\" {}' -DisplayName 'zapret' -StartupType Automatic",
        bin.display(),
        final_args
    );

    if let Ok(s) = sh!("powershell", "-NoProfile", "-Command", &cmd) {
        if s.success() {
            let _ = sh!("sc", "start", "zapret");
            let _ = RegKey::predef(HKEY_LOCAL_MACHINE)
                .create_subkey(HKLM_PATH)
                .map(|(k, _)| k.set_value("zapret-discord-youtube", name));
            info(app, &format!("Запущено: {}", name));
        }
    }
}

pub fn stop_service(app: &AppHandle) {
    let _ = sh!("taskkill", "/F", "/IM", "winws.exe", "/T");
    for d in ["WinDivert", "WinDivert14", "zapret"] {
        info(app, &format!("Удаляю сервис {}...", d));
        let _ = sh!("net", "stop", d);
        let _ = sh!("sc", "delete", d);
    }
    info(app, "Сервисы ZAPRET-а очищены и удалены.");
}
