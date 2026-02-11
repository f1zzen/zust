use crate::sh;
use crate::utils::*;
use hickory_resolver::AsyncResolver;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use md5::{Digest, Md5};
use std::fs;
use std::io::Write;
use std::net::IpAddr;
use std::path::Path;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;
use winreg::RegKey;
use winreg::enums::*;

const WINWS_EXE: &str =
    "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/zapret-winws/winws.exe";
const HKLM_PATH: &str = r"System\CurrentControlSet\Services\zapret";
const CONFIG_EXTENSION: &str = ".zapret";
const FLOWSEAL_REPO: &str =
    "https://raw.githubusercontent.com/Flowseal/zapret-discord-youtube/refs/heads/main/";
const REPO_STRAT_NAMES: &[&str] = &[
    "general (ALT).bat",
    "general (ALT2).bat",
    "general (ALT3).bat",
    "general (ALT4).bat",
    "general (ALT5).bat",
    "general (ALT6).bat",
    "general (ALT7).bat",
    "general (ALT8).bat",
    "general (ALT9).bat",
    "general (ALT10).bat",
    "general (ALT11).bat",
    "general.bat",
    "general (FAKE TLS AUTO ALT).bat",
    "general (FAKE TLS AUTO ALT2).bat",
    "general (FAKE TLS AUTO ALT3).bat",
    "general (FAKE TLS AUTO).bat",
    "general (SIMPLE FAKE ALT).bat",
    "general (SIMPLE FAKE ALT2).bat",
    "general (SIMPLE FAKE).bat",
];

pub struct Zapret;

impl Zapret {
    // удаление старой папки _up_
    // для людей, которые хотят перейти на новую версию
    pub fn handle_up_folder(app: &AppHandle, force: bool) -> Result<(), String> {
        let res_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
        let up_path = res_dir.join("_up_");

        if up_path.exists() {
            info(app, "detect legacy folder, moving");
            let current_strat_name = Self::get_strategy();
            let strat_list = Self::get_list_strategies(app);
            let strat_index = strat_list.iter().position(|s| s == &current_strat_name);
            Self::stop_service(app);
            std::thread::sleep(std::time::Duration::from_millis(500));
            if force {
                info(app, "moving strategies from legacy folder ");
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
                let err_msg = format!("error on deleting _up_ folder: {}", e);
                info(app, &err_msg);
                return Err(err_msg);
            }

            info(app, "folder _up_ deleted");
            if current_strat_name != "Отсутствует" {
                if let Some(idx) = strat_index {
                    info(app, &format!("strategy {} restart", current_strat_name));
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

    pub async fn add_ip(app: AppHandle, file_name: String, ip: String) -> Result<(), String> {
        let lists_dir = Self::zapret_path(&app, "ipset-configs");
        let mut file_path = lists_dir.join(&file_name);

        if !file_path.exists() {
            if &file_name != "ipset-all.txt" {
                return Err(format!("Файл {} не найден", file_name));
            } else {
                file_path = Self::zapret_path(&app, "lists").join("ipset-all.txt");
            }
        }
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return Err("Некорректный формат IP. Ожидалось x.x.x.x".to_string());
        }
        let formatted_ip = format!("{}.{}.{}.0/24", parts[0], parts[1], parts[2]);

        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&file_path)
            .map_err(|e| e.to_string())?;
        writeln!(file, "{}", formatted_ip).map_err(|e| e.to_string())?;

        Ok(())
    }

    pub async fn resolve_host(host: String) -> Result<String, String> {
        let resolver = AsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
        let srv_name = format!("_minecraft._tcp.{}", host);
        let target_host = if let Ok(srv_lookup) = resolver.srv_lookup(srv_name).await {
            srv_lookup
                .iter()
                .next()
                .map(|srv| srv.target().to_string().trim_end_matches('.').to_string())
                .unwrap_or_else(|| host.clone())
        } else {
            host.clone()
        };
        let ip_lookup = resolver
            .lookup_ip(target_host)
            .await
            .map_err(|e| format!("Ошибка резолва: {}", e))?;

        let ip = ip_lookup
            .iter()
            .next()
            .ok_or_else(|| "IP-адрес не найден".to_string())?;
        match ip {
            IpAddr::V4(v4) => {
                let o = v4.octets();
                Ok(format!("{}.{}.{}.0/24", o[0], o[1], o[2]))
            }
            IpAddr::V6(v6) => Ok(v6.to_string()),
        }
    }

    pub fn get_custom_ipset_files(app: &AppHandle) -> Vec<String> {
        let path = Self::zapret_path(app, "ipset-configs");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        list_files(path, ".txt")
    }

    fn extract_args_from_bat(content: &str) -> Option<String> {
        content
            .find("winws.exe\"")
            .map(|start_idx| content[start_idx + 11..].trim().to_string())
    }

    pub async fn check_strategy_updates(app: AppHandle) -> Result<Vec<String>, String> {
        let client = reqwest::Client::new();
        let strats_dir = Self::zapret_path(&app, "strategies");
        let mut to_update = Vec::new();

        for file_name in REPO_STRAT_NAMES {
            let url = format!("{}{}", FLOWSEAL_REPO, file_name);
            let res = client.get(&url).send().await.map_err(|e| e.to_string())?;

            if res.status().is_success() {
                let remote_bat_content = res.text().await.map_err(|e| e.to_string())?;

                if let Some(remote_args) = Self::extract_args_from_bat(&remote_bat_content) {
                    let zapret_name = file_name.replace(".bat", CONFIG_EXTENSION);
                    let zapret_path = strats_dir.join(&zapret_name);

                    let mut needs_update = true;
                    if zapret_path.exists() {
                        let local_zapret_content =
                            fs::read_to_string(&zapret_path).unwrap_or_default();
                        if local_zapret_content.trim() == remote_args.trim() {
                            needs_update = false;
                        }
                    }

                    if needs_update {
                        to_update.push(file_name.to_string());
                    }
                }
            }
        }
        Ok(to_update)
    }

    pub async fn apply_strategy_update(app: AppHandle, file_name: String) -> Result<(), String> {
        let client = reqwest::Client::new();
        let strats_dir = Self::zapret_path(&app, "strategies");

        let url = format!("{}{}", FLOWSEAL_REPO, file_name);
        let res = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if res.status().is_success() {
            let bytes = res.bytes().await.map_err(|e| e.to_string())?;
            let bat_path = strats_dir.join(&file_name);
            fs::write(&bat_path, &bytes).map_err(|e| e.to_string())?;
            Self::convert_multiple_bats(&app, vec![bat_path.to_string_lossy().into_owned()])
                .await?;
            let _ = fs::remove_file(bat_path);
            Ok(())
        } else {
            Err(format!("err response from github {}", res.status()))
        }
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
        info(app, "начало синхронизации");
        if let Err(e) = Self::copy_dir_all(&source, &target) {
            let err_msg = format!("ошибка синхронизации: {}", e);
            info(app, &err_msg);
            return Err(err_msg);
        }
        info(app, "синхронизация завершена (занятые файлы пропущены).");
        Ok(())
    }

    pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
        fs::create_dir_all(&dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let dst_path = dst.as_ref().join(entry.file_name());
            if ty.is_dir() {
                let _ = copy_dir_all(entry.path(), dst_path);
            } else {
                match fs::copy(entry.path(), &dst_path) {
                    Ok(_) => {}
                    Err(e) if e.raw_os_error() == Some(32) => {
                        eprintln!("файл занят, пропускаем {:?}", entry.path());
                    }
                    Err(e) => return Err(e),
                }
            }
        }
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
            let s: std::process::ExitStatus = s;
            if s.success() {
                let _ = sh!("sc", "start", "zapret");
                let _ = RegKey::predef(HKEY_LOCAL_MACHINE)
                    .create_subkey(HKLM_PATH)
                    .map(|(k, _)| k.set_value("zapret-discord-youtube", name));
                info(app, &format!("запущено: {}", name));
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
