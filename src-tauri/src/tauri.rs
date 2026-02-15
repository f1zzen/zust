use crate::bypass::hosts::Hosts;
use crate::bypass::proxies::{Proxies, Proxy};
use crate::bypass::zapret::Zapret;
use crate::settings::{self, Settings};
use crate::utils;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;
use std::{fs, path::PathBuf};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(serde::Serialize)]
pub struct HostsData {
    pub date: String,
    pub categories: HashMap<String, Vec<String>>,
}

#[tauri::command]
pub fn open_link(url: String) -> Result<(), String> {
    tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct StartServiceArgs {
    pub index: i32,
    pub ipset_config: Option<String>,
}

#[tauri::command]
pub fn get_strategy() -> String {
    Zapret::get_strategy()
}

#[tauri::command]
pub fn get_list_strategies(app: tauri::AppHandle) -> Vec<String> {
    Zapret::get_list_strategies(&app)
}

#[tauri::command]
pub fn open_ipset_dir(app: tauri::AppHandle) {
    let path = Zapret::zapret_path(&app, "ipset-configs");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    let _ = Command::new("explorer").arg(path).spawn();
}

#[tauri::command]
pub async fn apply_strategy_update(app: tauri::AppHandle, file_name: String) -> Result<(), String> {
    Zapret::apply_strategy_update(app, file_name).await
}

#[tauri::command]
pub async fn check_strategy_updates(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    Zapret::check_strategy_updates(app).await
}

#[tauri::command]
pub fn open_strats_dir(app: tauri::AppHandle) {
    let path = Zapret::zapret_path(&app, "strategies");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    let _ = Command::new("explorer").arg(path).spawn();
}

#[tauri::command]
pub async fn start_service(app: tauri::AppHandle, args: StartServiceArgs) -> Result<(), String> {
    Zapret::start_service(&app, args.index, args.ipset_config);
    Ok(())
}

#[tauri::command]
pub async fn stop_service(app: tauri::AppHandle) -> Result<(), String> {
    Zapret::stop_service(&app);
    Ok(())
}

#[tauri::command]
pub fn game_filter_toggle(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    Zapret::game_filter_toggle(enabled, &app)
}

#[tauri::command]
pub fn log(text: &str, app: tauri::AppHandle) {
    utils::info(&app, text);
}

#[tauri::command]
pub fn get_list_files(app: tauri::AppHandle) -> Vec<String> {
    Zapret::get_files_lists(&app)
}

#[tauri::command]
pub fn get_custom_configs(app: tauri::AppHandle) -> Vec<String> {
    Zapret::get_custom_ipset_files(&app)
}

#[tauri::command]
pub fn read_file(app: tauri::AppHandle, name: String) -> Result<String, String> {
    fs::read_to_string(Zapret::zapret_path(&app, "lists").join(name)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_file(app: tauri::AppHandle, name: String, content: String) -> Result<(), String> {
    fs::write(Zapret::zapret_path(&app, "lists").join(name), content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_config_path() -> PathBuf {
    settings::get_config_path()
}

#[tauri::command]
pub fn save_settings(settings: Settings) -> Result<(), String> {
    settings::save_settings(settings)
}

#[tauri::command]
pub fn load_settings() -> Settings {
    settings::load_settings()
}

#[tauri::command]
pub async fn convert_multiple_bats(
    paths: Vec<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    Zapret::convert_multiple_bats(&app, paths).await
}

#[tauri::command]
pub async fn get_hosts_data(app: tauri::AppHandle) -> Result<HostsData, String> {
    let content = Hosts::fetch(&app).await?;
    let date = Hosts::get_update_date(&content);
    let categories = Hosts::get_categories(&content);
    Ok(HostsData { date, categories })
}

#[tauri::command]
pub async fn check_winws_update(app: tauri::AppHandle) -> Result<bool, String> {
    Zapret::check_winws_update(app).await
}

#[tauri::command]
pub async fn save_hosts_selection(
    app: tauri::AppHandle,
    selected_lines: Vec<String>,
) -> Result<(), String> {
    let data = selected_lines.join("\n");
    Hosts::write(&app, &data)
}

#[tauri::command]
pub fn check_legacy_folder(app: tauri::AppHandle) -> bool {
    let res_dir = app.path().resource_dir().unwrap_or_default();
    let up_path = res_dir.join("_up_");
    up_path.exists() && up_path.is_dir()
}

#[tauri::command]
pub async fn run_cleanup(app: tauri::AppHandle) -> Result<(), String> {
    Zapret::handle_up_folder(&app, true)
}

#[tauri::command]
pub async fn sync_zapret_files(app: tauri::AppHandle) -> Result<(), String> {
    Zapret::sync_zapret_files(&app)?;
    Ok(())
}

#[tauri::command]
pub async fn add_ip(app: tauri::AppHandle, file_name: String, ip: String) -> Result<(), String> {
    Zapret::add_ip(app, file_name, ip).await
}

#[tauri::command]
pub async fn resolve_host(host: String) -> Result<String, String> {
    Zapret::resolve_host(host).await
}

#[tauri::command]
pub async fn get_proxy_list() -> Result<Vec<String>, String> {
    Proxies::get_proxy_list().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_proxy_ping(
    app: tauri::AppHandle,
    address: &str,
) -> Result<Option<Proxy>, String> {
    let status = Proxies::check_proxy_ping(&address, app).await;
    Ok(status)
}

#[tauri::command]
pub fn main_window_init(app: tauri::AppHandle) {
    let _ = WebviewWindowBuilder::new(&app, "main", WebviewUrl::App("index.html".into()))
        .title("Zust")
        .visible(false)
        .build();
}

#[tauri::command]
pub async fn update_tls_bin(app: tauri::AppHandle) -> Result<String, String> {
    Zapret::update_tls_bin(app).await
}
