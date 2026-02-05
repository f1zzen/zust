use chrono::Local;
use md5::{Digest, Md5};
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
const HOSTS_URL: &str =
    "https://raw.githubusercontent.com/ImMALWARE/dns.malw.link/refs/heads/master/hosts";
const WINWS_EXE: &str =
    "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/zapret-winws/winws.exe";
const HKLM_PATH: &str = r"System\CurrentControlSet\Services\zapret";
const CONFIG_EXTENSION: &str = ".zapret";

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

pub struct Zapret;

impl Zapret {
    // удаление старой папки _up_
    // для людей, которые хотят перейти на новую версию
    pub fn handle_up_folder(app: &AppHandle, force: bool) -> Result<(), String> {
        let res_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
        let up_path = res_dir.join("_up_");

        if up_path.exists() {
            info(app, "обнаружена папка _up_. начинаю процесс обработки");
            let current_strat_name = Self::get_strategy();
            let strat_list = Self::get_list_strategies(app);
            let strat_index = strat_list.iter().position(|s| s == &current_strat_name);
            Self::stop_service(app);
            std::thread::sleep(std::time::Duration::from_millis(500));
            if force {
                info(app, "перенос стратегий из _up_..");
                let old_str_path = up_path.join("zapret").join("strategies");
                let active_str_path = Self::zapret_path(app, "strategies");

                if old_str_path.exists() {
                    if let Ok(entries) = fs::read_dir(&old_str_path) {
                        for entry in entries.filter_map(|e| e.ok()) {
                            let path = entry.path();
                            if path.is_file() {
                                let file_name = path.file_name().unwrap();
                                let dest_path = active_str_path.join(file_name);
                                if !dest_path.exists() {
                                    let _ = fs::copy(&path, &dest_path);
                                }
                            }
                        }
                    }
                }
            }

            if let Err(e) = fs::remove_dir_all(&up_path) {
                let err_msg = format!("ошибка удаления _up_: {}", e);
                info(app, &err_msg);
                return Err(err_msg);
            }

            info(app, "папка _up_ удалена");
            if current_strat_name != "Отсутствует" {
                if let Some(idx) = strat_index {
                    info(
                        app,
                        &format!("перезапуск стратегии: {}", current_strat_name),
                    );
                    Self::start_service(app, (idx + 1) as i32, None);
                }
            }
        }
        Ok(())
    }

    // проверка обновлений winws
    // берётся напрямую с репозитория от bol-van
    pub async fn check_winws_update(app: AppHandle) -> Result<bool, String> {
        let bin_path = Self::zapret_path(&app, "bin/winws.exe");
        let local_hash = if bin_path.exists() {
            let content = fs::read(&bin_path).map_err(|e| e.to_string())?;
            format!("{:x}", Md5::digest(&content))
        } else {
            return Ok(true);
        };
        let response = reqwest::get(WINWS_EXE).await.map_err(|e| e.to_string())?;
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        let remote_hash = format!("{:x}", Md5::digest(&bytes));
        info(
            &app,
            &format!("hash diff {:?} n {:?}", remote_hash, local_hash),
        );
        if local_hash != remote_hash {
            info(&app, "new winws.exe, download");
            fs::write(&bin_path, &bytes).map_err(|e| e.to_string())?;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn get_custom_ipset_files(app: &AppHandle) -> Vec<String> {
        let path = Self::zapret_path(app, "ipset-configs");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        list_files(path, ".txt")
    }

    pub fn game_filter_toggle(enabled: bool, app: &AppHandle) -> Result<(), String> {
        let path = Self::zapret_path(app, "utils/game_filter.enabled");

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

    fn create_strategy_file(name: String, content: &str, app: &AppHandle) -> Result<(), String> {
        let path = Self::zapret_path(app, "strategies");
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
            let content = fs::read_to_string(path)
                .map_err(|e| format!("Ошибка чтения {}: {}", path_str, e))?;
            if let Some(start_idx) = content.find("winws.exe\"") {
                let args = &content[start_idx + 11..].trim();
                let file_name = path.file_stem().unwrap().to_str().unwrap();
                let new_file_name = format!("{}{CONFIG_EXTENSION}", file_name);
                Self::create_strategy_file(new_file_name, args, app)?;
            }
        }
        Ok(())
    }

    fn zapret_storage(app: &AppHandle, sub: &str) -> PathBuf {
        let path = {
            app.path()
                .resource_dir()
                .expect("failed to get resource dir")
                .join("zapret")
        };

        let final_path = path.join(sub);
        let path_str = final_path.to_string_lossy();
        // приколы раста \\?\C:\\Windows..
        PathBuf::from(path_str.trim_start_matches(r"\\?\").to_string())
    }

    pub fn zapret_path(app: &AppHandle, sub: &str) -> PathBuf {
        let mut path = app
            .path()
            .app_config_dir()
            .expect("failed to get config dir");
        path.push("zapret-winws");

        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path.join(sub)
    }

    // синхрониз. файлов с билда до appdata
    pub fn sync_zapret_files(app: &AppHandle) -> Result<(), String> {
        let source = Self::zapret_storage(app, "");
        let target = Self::zapret_path(app, "");
        if target.join("bin/winws.exe").exists() {
            return Ok(());
        }
        info(app, "синхронизация файлов с хранилища в %APPDATA%...");
        copy_dir_all(source, target).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_list_strategies(app: &AppHandle) -> Vec<String> {
        list_files(Self::zapret_path(app, "strategies"), CONFIG_EXTENSION)
    }

    pub fn get_strategy() -> String {
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(HKLM_PATH)
            .and_then(|k| k.get_value("zapret-discord-youtube"))
            .unwrap_or_else(|_| "Отсутствует".to_string())
    }

    pub fn get_files_strategies(app: &AppHandle) -> Vec<String> {
        fs::read_dir(Self::zapret_path(app, "lists"))
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
        let lists_dir = Self::zapret_path(app, "lists");
        let game_filter_enabled = Self::zapret_path(app, "utils/game_filter.enabled").exists();
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

        let mut args = raw.replace("%GameFilter%", filter).replace(
            "%BIN%",
            &format!("{}\\", Self::zapret_path(app, "bin").display()),
        );

        let ipset_path = match custom_ipset.as_deref() {
            Some("none") => lists_dir.join("ipset-none-hide.txt"),
            Some("any") => lists_dir.join("ipset-any-hide.txt"),
            Some(file) => {
                let custom = Self::zapret_path(app, "ipset-configs").join(file);
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
                .filter(|n| {
                    n.starts_with("list-") && n.ends_with(".txt") && !n.contains("excluded")
                })
                .map(|n| format!(" --hostlist=\"{}\"", lists_dir.join(n).display()))
                .collect();
            args.push_str(&hosts);
        }
        args.trim().to_string()
    }

    pub fn start_service(app: &AppHandle, index: i32, ipset_config: Option<String>) {
        Self::stop_service(app);
        let list = Self::get_list_strategies(app);
        let name = match list.get((index - 1).max(0) as usize) {
            Some(n) => n,
            None => {
                info(app, "Ошибка: Стратегия не найдена.");
                return;
            }
        };

        let strategy_raw =
            fs::read_to_string(Self::zapret_path(app, "strategies").join(name)).unwrap_or_default();
        let final_args = Self::build_full_args(app, strategy_raw.trim(), ipset_config);
        let bin = Self::zapret_path(app, "bin/winws.exe");

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

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
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
