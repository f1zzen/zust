// ВАЖНОЕ ПРИМЕЧАНИЕ!!
// Если захотели изменить код, то учтите:
// ::tauri - библиотека tauri
// tauri - компонент tauri.rs

pub mod bypass;
pub mod settings;
pub mod utils;

use ::tauri::Emitter;
use ::tauri::Manager;
use ::tauri::menu::Menu;
use ::tauri::menu::MenuItem;
use ::tauri::tray::MouseButton;
use ::tauri::tray::TrayIconBuilder;
use ::tauri::tray::TrayIconEvent;
use std::fs;

mod tauri;

use crate::bypass::tor::Tor;
use crate::tauri::*;
use crate::utils::info;
use crate::utils::process_incoming_url;
use crate::utils::register_zust_protocol;

pub fn run() {
    unsafe {
        std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", "--no-sandbox");
        std::env::set_var("WEBVIEW2_IGNORE_WEBVIEW_VERSION_MISMATCH", "1");
    }

    ::tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let _ = app.get_webview_window("main").map(|w| {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
                let _ = w.emit("window-visible", true);
            });
            if let Some(url) = args.iter().find(|a| a.starts_with("zust://")) {
                let handle = app.clone();
                let url_to_process = url.clone();
                ::tauri::async_runtime::spawn(async move {
                    process_incoming_url(handle, url_to_process).await;
                });
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--silent"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(::tauri::generate_handler![
            open_link,
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
            open_strats_dir,
            check_legacy_folder,
            sync_zapret_files,
            apply_strategy_update,
            check_strategy_updates,
            resolve_host,
            add_ip,
            get_proxy_list,
            check_proxy_ping,
            update_tls_bin,
            start_tor,
            stop_tor,
            enable_system_proxy,
            disable_system_proxy,
            is_active,
            handle_add_link,
            check_resources_exist,
            flush_dns,
            restore_zapret_files,
            check_tor_status,
            manage_autostart,
            get_file_tree,
            open_in_editor,
            get_installed_categories,
            start_hotspot,
            stop_hotspot,
            get_hotspot_status,
            zapret_4_tor
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let _ = register_zust_protocol();
            let handle_for_settings = handle.clone();
            let handle_info = handle.clone();

            ::tauri::async_runtime::spawn(async move {
                let settings = crate::settings::load_settings();
                if settings.auto_tor {
                    if !Tor::check_existing_tor(handle_for_settings.clone()) {
                        if let Err(e) = Tor::start(handle_for_settings.clone()).await {
                            info(&handle_info, &e);
                        }
                    }
                }
                let _ = manage_autostart(settings.auto_start).await;
            });

            let args: Vec<String> = std::env::args().collect();
            if let Some(u) = args.iter().find(|a| a.starts_with("zust://")) {
                let url = u.clone();
                let handle_for_url = handle.clone();
                ::tauri::async_runtime::spawn(async move {
                    process_incoming_url(handle_for_url, url).await;
                });
            }

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
                        let _ = Tor::stop(app.clone());
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
                            let _ = w.emit("window-visible", true);
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            match event {
                ::tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                    let _ = window.emit("window-visible", false);
                }
                ::tauri::WindowEvent::Resized(_) => {
                    if let Ok(minimized) = window.is_minimized() {
                        let _ = window.emit("window-visible", !minimized);
                    }
                }
                _ => {}
            }
        })
        .run(::tauri::generate_context!())
        .expect("error while running tauri application")
}
