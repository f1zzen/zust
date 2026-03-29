use crate::sh;
use crate::utils::*;
use md5::{Digest, Md5};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri::Runtime;
use winreg::RegKey;
use winreg::enums::*;

const STUN_BIN_DATA: &[u8] = include_bytes!("../../zapret/bin/stun.bin");
const WINWS_EXE: &str =
    "https://github.com/bol-van/zapret-win-bundle/raw/refs/heads/master/zapret-winws/winws.exe";
const MAX_RU_BIN: &str = "https://github.com/Flowseal/zapret-discord-youtube/raw/refs/heads/main/bin/tls_clienthello_max_ru.bin";
const HKLM_PATH: &str = r"System\CurrentControlSet\Services\zapret";
const CONFIG_EXTENSION: &str = ".zapret";
const STRATEGIES_REPO: &str =
    "https://raw.githubusercontent.com/f1zzen/zust_strategies/refs/heads/main/";

pub struct Zapret;
pub struct ZExtensions;

impl ZExtensions {
    // набор "расширений" для будущей работы с застом. может быть здесь будет больше функционала..
    pub async fn repair_zapret4tor(app: AppHandle, strat_name: String) -> Result<(), String> {
        let list_path = Zapret::zapret_path(&app, "lists").join("list-general.txt");
        let domain = "torproject.org";
        if !Zapret::check_domain_in_list(&app, "list-general.txt", domain) {
            app.emit(
                "log-event",
                format!("[SYSTEM] Добавление {} в список...", domain),
            )
            .ok();

            let mut content = std::fs::read_to_string(&list_path).unwrap_or_default();
            if !content.is_empty() && !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(domain);
            content.push('\n');

            std::fs::write(&list_path, content).map_err(|e| e.to_string())?;
        }
        let all_strategies = Zapret::get_list_strategies(&app);
        let strat_index = all_strategies
            .iter()
            .position(|s| s == &strat_name || s.replace(".zapret", "") == strat_name)
            .map(|pos| (pos + 1) as i32)
            .unwrap_or(1);

        let current_ipset = Zapret::get_ipset_config();
        Zapret::start_service(&app, strat_index, Some(current_ipset));
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        if Zapret::get_status() {
            Ok(())
        } else {
            Err("Не удалось запустить службу обхода".into())
        }
    }
}

impl Zapret {
    pub fn get_ipset_config() -> String {
        RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey(HKLM_PATH)
            .and_then(|k| k.get_value("zapret-ipset-config"))
            .unwrap_or_else(|_| "ipset-all.txt".to_string())
    }

    pub async fn update_tls_bin(app: AppHandle) -> Result<String, String> {
        let target_path = Zapret::zapret_path(&app, "bin").join("tls_clienthello_max_ru.bin");
        let response = reqwest::get(MAX_RU_BIN)
            .await
            .map_err(|e| format!("Ошибка запроса! Хорош-ли твой интернет?: {}", e))?;

        if response.status().is_success() {
            let bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Неизвестная ошибка данных.. Всё нормально?: {}", e))?;

            fs::write(&target_path, bytes)
                .map_err(|e| format!("Неизвестная ошибка при записи файла. Всё хорошо?: {}", e))?;

            Ok("Файл успешно обновлён!".to_string())
        } else {
            Err(format!("Сервер вернул ошибку: {}", response.status()))
        }
    }

    pub fn get_status() -> bool {
        let output = Command::new("tasklist")
            .args(["/NH", "/FI", "IMAGENAME eq winws.exe"])
            .creation_flags(0x08000000)
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            return stdout.contains("winws.exe");
        }
        false
    }

    pub fn check_domain_in_list(app: &AppHandle, list_name: &str, domain: &str) -> bool {
        let list_path = Self::zapret_path(app, "lists").join(list_name);
        if !list_path.exists() {
            return false;
        }

        if let Ok(content) = fs::read_to_string(list_path) {
            return content.lines().any(|line| line.trim() == domain);
        }
        false
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
            &format!(
                "Сравнение хэшов программ: {:?}.НОВ n {:?}.ТЕК",
                remote_hash, local_hash
            ),
        );
        if local_hash != remote_hash {
            info(&app, "Обнаружен новый бинарник запрета, скачиваю");
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

    pub async fn check_strategy_updates(app: AppHandle) -> Result<Vec<String>, String> {
        let client = reqwest::Client::new();
        let strats_dir = Self::zapret_path(&app, "strategies");
        let mut to_update = Vec::new();

        let list_url = format!("{}list", STRATEGIES_REPO);
        let list_res = client
            .get(&list_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !list_res.status().is_success() {
            return Err(format!(
                "Не удалось получить список стратегий: {}",
                list_res.status()
            ));
        }

        let list_content = list_res.text().await.map_err(|e| e.to_string())?;
        let repo_strat_names: Vec<&str> = list_content.lines().filter(|l| !l.is_empty()).collect();

        for file_name in repo_strat_names {
            let url = format!("{}{}", STRATEGIES_REPO, file_name);
            let res = client.get(&url).send().await.map_err(|e| e.to_string())?;

            if res.status().is_success() {
                let remote_content = res.text().await.map_err(|e| e.to_string())?;
                let zapret_path = strats_dir.join(file_name);

                let mut needs_update = true;
                if zapret_path.exists() {
                    let local_content = fs::read_to_string(&zapret_path).unwrap_or_default();
                    if local_content.trim() == remote_content.trim() {
                        needs_update = false;
                    }
                }

                if needs_update {
                    to_update.push(file_name.to_string());
                    println!("ТРЕБУЕТСЯ ОБНОВЛЕНИЕ {}", file_name.to_string())
                }
            }
        }
        Ok(to_update)
    }

    pub async fn apply_strategy_update(app: AppHandle, file_name: String) -> Result<(), String> {
        let client = reqwest::Client::new();
        let strats_dir = Self::zapret_path(&app, "strategies");

        let url = format!("{}{}", STRATEGIES_REPO, file_name);
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
            Err(format!(
                "При подключении к github.com произошла ошибка {}",
                res.status()
            ))
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

    fn add_only_missing_files(app: &AppHandle, src: &Path, dst: &Path) -> std::io::Result<()> {
        if !src.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let name = entry.file_name();
            let src_path = entry.path();
            let dst_path = dst.join(&name);

            if file_type.is_dir() {
                if !dst_path.exists() {
                    fs::create_dir_all(&dst_path)?;
                }
                Self::add_only_missing_files(app, &src_path, &dst_path)?;
            } else {
                if !dst_path.exists() {
                    info(app, &format!("Добавление нового компонента: {:?}", name));
                    fs::copy(&src_path, &dst_path)?;
                }
            }
        }
        Ok(())
    }

    pub fn sync_zapret_files(app: &AppHandle) -> Result<(), String> {
        let source_root = Self::zapret_storage(app, "");
        let target_root = Self::zapret_path(app, "");

        let bin_path = target_root.join("bin").join("winws.exe");
        let strat_dir = target_root.join("strategies");

        let mut needs_initial_sync = !bin_path.exists();

        if !needs_initial_sync {
            let has_strategies = fs::read_dir(&strat_dir)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .any(|e| e.path().extension().map_or(false, |ext| ext == "zapret"))
                })
                .unwrap_or(false);
            if !has_strategies {
                needs_initial_sync = true;
            }
        }
        if needs_initial_sync {
            info(app, "Первичная настройка ресурсов ZAPRET...");
            Self::copy_dir_all(&source_root, &target_root).map_err(|e| e.to_string())?;
        } else {
            if let Err(e) = Self::add_only_missing_files(app, &source_root, &target_root) {
                info(app, &format!("Предупреждение при синхронизации: {}", e));
            }
        }

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

    pub fn bin_files_exist(app: &AppHandle) -> Result<(), String> {
        let bin_dir = Self::zapret_path(app, "bin");
        if !bin_dir.exists() {
            fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;
        }
        let stun_path = bin_dir.join("stun.bin");
        if !stun_path.exists() {
            fs::write(&stun_path, STUN_BIN_DATA).map_err(|e| format!("stun.bin: {}", e))?;
            info(app, "Файл stun.bin был успешно восстановлен.");
        }

        Ok(())
    }

    pub fn get_files_lists(app: &AppHandle) -> Vec<String> {
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

    pub async fn restore_zapret_files<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
        let app_wry = (&app as &dyn std::any::Any)
            .downcast_ref::<AppHandle<tauri::Wry>>()
            .ok_or("downcast failed")?;

        Self::stop_service(app_wry);
        let storage_dir = Zapret::zapret_storage(app_wry, "");
        let working_dir = Zapret::zapret_path(app_wry, "");
        if !storage_dir.exists() {
            return Err(format!(
                "Ошибка: Хранилище не найдено по пути {:?}",
                storage_dir
            ));
        }
        if working_dir.exists() {
            fs::remove_dir_all(&working_dir).map_err(|e| {
                format!(
                    "Не удалось очистить рабочую папку. Возможно, процесс запущен: {}",
                    e
                )
            })?;
        }
        fs::create_dir_all(&working_dir).map_err(|e| e.to_string())?;
        copy_dir_all(&storage_dir, &working_dir)
            .map_err(|e| format!("Ошибка при копировании из хранилища: {}", e))?;

        Ok("Система успешно восстановлена из хранилища".into())
    }

    pub fn build_full_args(app: &AppHandle, raw: &str, custom_ipset: Option<String>) -> String {
        let lists_dir = Self::zapret_path(app, "lists");
        let game_filter_enabled = Self::zapret_path(app, "utils/game_filter.enabled").exists();
        info(
            app,
            &format!(
                "gameFilter: {}",
                if game_filter_enabled { "on" } else { "off" }
            ),
        );
        let filter = if game_filter_enabled {
            "1024-65535"
        } else {
            "12"
        };

        let mut args = raw
            .replace("%GameFilter%", filter)
            .replace(
                "%BIN%",
                &format!("{}\\", Self::zapret_path(app, "bin").display()),
            )
            .replace("%GameFilterTCP%", filter)
            .replace("%GameFilterUDP%", filter)
            .replace("-user", "");
        args = args
            .replace("^", "")
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

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
        args = args.replace(
            "--ipset=\"%LISTS%ipset-all.txt\"",
            &format!("--ipset=\"{}\"", ipset_path.display()),
        );

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

        args = args.trim().to_string();

        info(&app, &args);
        args
    }

    pub fn start_service(app: &AppHandle, index: i32, ipset_config: Option<String>) {
        Self::stop_service(app);
        let _ = Self::bin_files_exist(app);
        let list = Self::get_list_strategies(app);
        let name = match list.get((index - 1).max(0) as usize) {
            Some(n) => n,
            None => return,
        };

        let strategy_raw =
            fs::read_to_string(Self::zapret_path(app, "strategies").join(name)).unwrap_or_default();
        let final_args = Self::build_full_args(app, strategy_raw.trim(), ipset_config.clone());
        let bin = Self::zapret_path(app, "bin/winws.exe");

        let _ = sh!(
            "netsh",
            "interface",
            "tcp",
            "set",
            "global",
            "timestamps=enabled"
        );

        let log_path = Self::zapret_path(app, "latest.log");
        let binary_path = format!(
            "cmd.exe /c \"\"{}\" {} > \"{}\" 2>&1\"",
            bin.display(),
            final_args,
            log_path.display()
        );

        let cmd = format!(
            "New-Service -Name 'zapret' -BinaryPathName '{}' -DisplayName 'zapret' -StartupType Automatic",
            binary_path
        );

        if let Ok(s) = sh!("powershell", "-NoProfile", "-Command", &cmd) {
            if s.success() {
                let _ = sh!("sc", "start", "zapret");
                let _ = RegKey::predef(HKEY_LOCAL_MACHINE)
                    .create_subkey(HKLM_PATH)
                    .map(|(k, _)| {
                        let _ = k.set_value("zapret-discord-youtube", name);
                        let _ = k.set_value(
                            "zapret-ipset-config",
                            &ipset_config.unwrap_or_else(|| "ipset-all.txt".to_string()),
                        );
                    });
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

    pub fn is_active() -> bool {
        let output = Command::new("sc")
            .args(["query", "zapret"])
            .creation_flags(0x08000000)
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            return stdout.contains("RUNNING");
        }
        false
    }
}
