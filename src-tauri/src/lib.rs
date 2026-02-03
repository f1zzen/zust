use serde::Deserialize;
use settings::Settings;
use std::{fs, path::PathBuf};
mod settings;
mod utils;
use crate::utils::Hosts;
use std::collections::HashMap;
use std::process::Command;
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
};

#[derive(serde::Serialize)]
pub struct HostsData {
    pub date: String,
    pub categories: HashMap<String, Vec<String>>,
}

#[derive(Deserialize)]
pub struct StartServiceArgs {
    pub index: i32,
    pub ipset_config: Option<String>,
}

#[tauri::command]
fn get_strategy() -> String {
    utils::get_strategy()
}

#[tauri::command]
fn get_list_strategies(app: tauri::AppHandle) -> Vec<String> {
    utils::get_list_strategies(&app)
}

#[tauri::command]
fn open_ipset_dir(app: tauri::AppHandle) {
    let path = utils::zapret_path(&app, "ipset-configs");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    let _ = Command::new("explorer").arg(path).spawn();
}

#[tauri::command]
fn open_strats_dir(app: tauri::AppHandle) {
    let path = utils::zapret_path(&app, "strategies");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    let _ = Command::new("explorer").arg(path).spawn();
}

#[tauri::command]
async fn start_service(app: tauri::AppHandle, args: StartServiceArgs) -> Result<(), String> {
    utils::start_service(&app, args.index, args.ipset_config);
    Ok(())
}

#[tauri::command]
async fn stop_service(app: tauri::AppHandle) -> Result<(), String> {
    utils::stop_service(&app);
    Ok(())
}

#[tauri::command]
fn game_filter_toggle(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    utils::game_filter_toggle(enabled, &app)
}

#[tauri::command]
fn log(text: &str, app: tauri::AppHandle) {
    utils::info(&app, text);
}

#[tauri::command]
fn get_list_files(app: tauri::AppHandle) -> Vec<String> {
    utils::get_files_strategies(&app)
}

#[tauri::command]
fn get_custom_configs(app: tauri::AppHandle) -> Vec<String> {
    utils::get_custom_ipset_files(&app)
}

#[tauri::command]
fn read_file(app: tauri::AppHandle, name: String) -> Result<String, String> {
    fs::read_to_string(utils::zapret_path(&app, "lists").join(name)).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_file(app: tauri::AppHandle, name: String, content: String) -> Result<(), String> {
    fs::write(utils::zapret_path(&app, "lists").join(name), content).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_config_path() -> PathBuf {
    settings::get_config_path()
}

#[tauri::command]
fn save_settings(settings: Settings) -> Result<(), String> {
    settings::save_settings(settings)
}

#[tauri::command]
fn load_settings() -> Settings {
    settings::load_settings()
}

#[tauri::command]
async fn convert_multiple_bats(paths: Vec<String>, app: tauri::AppHandle) -> Result<(), String> {
    utils::convert_multiple_bats(&app, paths).await
}

#[tauri::command]
async fn get_hosts_data(app: tauri::AppHandle) -> Result<HostsData, String> {
    let content = Hosts::fetch(&app).await?;
    let date = Hosts::get_update_date(&content);
    let categories = Hosts::get_categories(&content);
    Ok(HostsData { date, categories })
}

#[tauri::command]
async fn check_winws_update(app: tauri::AppHandle) -> Result<bool, String> {
    utils::check_winws_update(app).await
}

#[tauri::command]
async fn save_hosts_selection(
    app: tauri::AppHandle,
    selected_lines: Vec<String>,
) -> Result<(), String> {
    let data = selected_lines.join("\n");
    Hosts::write(&app, &data)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app.get_webview_window("main").map(|w| {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            });
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--silent"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_strategy,
            start_service,
            stop_service,
            get_list_files,
            read_file,
            save_file,
            get_list_strategies,
            get_config_path,
            save_settings,
            load_settings,
            game_filter_toggle,
            log,
            convert_multiple_bats,
            get_custom_configs,
            open_ipset_dir,
            get_hosts_data,
            save_hosts_selection,
            check_winws_update,
            open_strats_dir
        ])
        .setup(|app| {
            if let Ok(path) = app.path().executable_dir() {
                let mut log_path = path;
                log_path.push("latest.log");
                let _ = fs::remove_file(log_path);
            }
            let quit_i = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;

            TrayIconBuilder::with_id("zust_tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if crate::settings::load_settings().minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
