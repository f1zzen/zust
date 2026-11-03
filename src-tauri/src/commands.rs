//! Все команды, которые вызывает интерфейс через `invoke(...)`.

use std::collections::HashMap;
use std::path::Path;
use zust_core::bypass::hosts::Hosts;
use zust_core::bypass::hotspot::HotSpot;
use zust_core::bypass::proxies::{Proxies, Proxy};
use zust_core::bypass::tor::Tor;
use zust_core::bypass::zapret::Zapret;
use zust_core::platform;
use zust_core::settings::{self, Settings};
use zust_core::utils::{self, AddType, FileTree};
use zust_core::{hub, logger::info};

type Res<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())
}

// ---------- приложение ----------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: &'static str,
    pub commit: &'static str,
    pub build_date: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub debug: bool,
}

pub fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        commit: env!("ZUST_COMMIT"),
        build_date: env!("ZUST_BUILD_DATE"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        debug: cfg!(debug_assertions),
    }
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    app_info()
}

#[tauri::command]
pub fn log(text: &str) {
    let app = hub();
    info(&app, text);
}

/// Открывает ссылку в браузере/Telegram. Схемы ограничены, чтобы новость
/// или deeplink не могли открыть file:// или запустить программу.
#[tauri::command]
pub fn open_link(url: String) -> Res<()> {
    let ok = ["https://", "http://", "tg://", "mailto:"]
        .iter()
        .any(|p| url.to_lowercase().starts_with(p));
    if !ok {
        return Err("Неподдерживаемая ссылка".into());
    }
    tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|e| e.to_string())
}

/// Открывает папку zapret: strategies | ipset-configs | lists | "" (корень).
#[tauri::command]
pub fn open_folder(kind: String) -> Res<()> {
    let sub = match kind.as_str() {
        "strategies" | "ipset-configs" | "lists" | "" => kind.as_str(),
        _ => return Err("Неизвестная папка".into()),
    };
    let path = Zapret::path(sub);
    let _ = std::fs::create_dir_all(&path);
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

/// Открывает файл из папки zapret в редакторе по умолчанию.
#[tauri::command]
pub fn open_in_editor(path: String) -> Res<()> {
    let p = Path::new(&path);
    let root = Zapret::path("");
    let inside = p
        .canonicalize()
        .ok()
        .zip(root.canonicalize().ok())
        .is_some_and(|(p, r)| p.starts_with(r));
    if !inside {
        return Err("Файл вне папки zapret".into());
    }
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|e| e.to_string())
}

// ---------- настройки ----------

#[tauri::command]
pub fn load_settings() -> Settings {
    settings::load()
}

/// Сохраняет настройки и применяет автозапуск, если он изменился.
#[tauri::command]
pub async fn save_settings(settings: Settings) -> Res<()> {
    let old = settings::load();
    settings::save(&settings)?;
    if old.auto_start != settings.auto_start {
        let enabled = settings.auto_start;
        blocking(move || platform::set_autostart(enabled)).await??;
    }
    Ok(())
}

// ---------- zapret ----------

#[tauri::command]
pub fn get_list_strategies() -> Vec<String> {
    Zapret::list_strategies()
}

#[tauri::command]
pub fn get_custom_configs() -> Vec<String> {
    Zapret::custom_ipset_files()
}

/// Имя запущенной стратегии или null.
#[tauri::command]
pub async fn get_strategy() -> Res<Option<String>> {
    blocking(Zapret::current_strategy).await
}

#[tauri::command]
pub async fn is_active() -> Res<bool> {
    blocking(Zapret::is_running).await
}

#[tauri::command]
pub async fn start_service(strategy: String, ipset: Option<String>) -> Res<bool> {
    let app = hub();
    Zapret::start(&app, &strategy, ipset).await
}

#[tauri::command]
pub async fn stop_service() -> Res<()> {
    let app = hub();
    Zapret::stop(&app).await
}

#[tauri::command]
pub async fn sync_zapret_files() -> Res<()> {
    let app = hub();
    blocking(move || Zapret::sync_files(&app)).await?
}

#[tauri::command]
pub async fn restore_zapret_files() -> Res<String> {
    let app = hub();
    Zapret::restore_files(&app).await
}

#[tauri::command]
pub async fn convert_multiple_bats(paths: Vec<String>) -> Res<usize> {
    blocking(move || Zapret::convert_bats(paths)).await?
}

#[tauri::command]
pub async fn check_strategy_updates() -> Res<Vec<String>> {
    Zapret::check_strategy_updates().await
}

#[tauri::command]
pub async fn apply_strategy_update(file_name: String) -> Res<()> {
    Zapret::apply_strategy_update(&file_name).await
}

#[tauri::command]
pub async fn update_tls_bin() -> Res<String> {
    let app = hub();
    Zapret::update_tls_bin(&app).await
}

#[tauri::command]
pub async fn zapret_4_tor(strat_name: String) -> Res<()> {
    let app = hub();
    Zapret::repair_for_tor(&app, &strat_name).await
}

#[tauri::command]
pub fn get_file_tree() -> Vec<FileTree> {
    utils::file_tree()
}

#[tauri::command]
pub fn check_resources_exist(strategy: Option<String>, ipset: Option<String>) -> Vec<String> {
    utils::missing_resources(strategy, ipset)
}

#[tauri::command]
pub async fn handle_add_link(
    add_type: AddType,
    url: String,
    custom_name: Option<String>,
) -> Res<String> {
    utils::handle_add_link(add_type, &url, custom_name).await
}

// ---------- hosts / DNS ----------

#[derive(serde::Serialize)]
pub struct HostsData {
    pub date: String,
    pub categories: HashMap<String, Vec<String>>,
}

#[tauri::command]
pub async fn get_hosts_data() -> Res<HostsData> {
    let app = hub();
    let content = Hosts::fetch(&app).await?;
    Ok(HostsData {
        date: Hosts::get_update_date(&content),
        categories: Hosts::get_categories(&content),
    })
}

#[tauri::command]
pub async fn save_hosts_selection(selected_lines: String) -> Res<()> {
    let app = hub();
    blocking(move || Hosts::write(&app, &selected_lines)).await?
}

#[tauri::command]
pub fn get_installed_categories() -> Vec<String> {
    Hosts::get_installed_categories()
}

#[tauri::command]
pub async fn flush_dns() -> Res<String> {
    blocking(platform::flush_dns).await??;
    Ok("Кэш DNS очищен".into())
}

// ---------- прокси для Telegram ----------

#[tauri::command]
pub async fn get_proxy_list() -> Res<Vec<String>> {
    Proxies::get_proxy_list().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_proxy_ping(address: String) -> Res<Option<Proxy>> {
    let app = hub();
    Ok(Proxies::check_proxy_ping(&address, app).await)
}

// ---------- TOR ----------

#[tauri::command]
pub async fn start_tor() -> Res<()> {
    let app = hub();
    Tor::start(app).await
}

#[tauri::command]
pub async fn stop_tor() -> Res<()> {
    blocking(Tor::stop).await
}

#[derive(serde::Serialize)]
pub struct TorStatus {
    pub running: bool,
    pub progress: u32,
}

#[tauri::command]
pub async fn check_tor_status() -> Res<TorStatus> {
    let app = hub();
    let running = blocking(Tor::is_running).await?;
    if running {
        Tor::replay_logs(&app);
    }
    // если Zust перезапустили при работающем TOR, прогресс неизвестен — значит, он уже подключён
    let progress = match (running, Tor::progress()) {
        (false, _) => 0,
        (true, 0) => 100,
        (true, p) => p,
    };
    Ok(TorStatus { running, progress })
}

#[tauri::command]
pub async fn enable_system_proxy() -> Res<()> {
    blocking(|| platform::set_system_proxy(true)).await?
}

#[tauri::command]
pub async fn disable_system_proxy() -> Res<()> {
    blocking(|| platform::set_system_proxy(false)).await?
}

// ---------- точка доступа ----------

#[tauri::command]
pub async fn start_hotspot(port: Option<u16>) -> Res<u16> {
    let app = hub();
    blocking(move || HotSpot::run(&app, port)).await?
}

#[tauri::command]
pub async fn stop_hotspot() -> Res<()> {
    blocking(HotSpot::stop).await
}

#[tauri::command]
pub async fn get_hotspot_status() -> Res<Option<u16>> {
    blocking(HotSpot::status).await
}
