mod commands;

pub use zust_core::{AUTOSTART_ARG, bypass, logger, paths, platform, settings, utils};

use bypass::hotspot::HotSpot;
use bypass::tor::Tor;
use logger::info;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        let _ = w.emit("window-visible", true);
    }
}

/// Полный выход: TOR и точка доступа останавливаются, zapret (служба) продолжает работать.
fn quit(app: &AppHandle) {
    Tor::stop();
    HotSpot::stop();
    app.exit(0);
}

pub fn run() {
    platform::prepare_environment();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            show_main(app);
            if let Some(url) = args.iter().find(|a| a.starts_with("zust://")) {
                utils::process_incoming_url(&zust_core::hub(), url);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::log,
            commands::open_link,
            commands::open_folder,
            commands::open_in_editor,
            commands::load_settings,
            commands::save_settings,
            commands::get_list_strategies,
            commands::get_custom_configs,
            commands::get_strategy,
            commands::is_active,
            commands::start_service,
            commands::stop_service,
            commands::sync_zapret_files,
            commands::restore_zapret_files,
            commands::convert_multiple_bats,
            commands::check_strategy_updates,
            commands::apply_strategy_update,
            commands::update_tls_bin,
            commands::zapret_4_tor,
            commands::get_file_tree,
            commands::check_resources_exist,
            commands::handle_add_link,
            commands::get_hosts_data,
            commands::save_hosts_selection,
            commands::get_installed_categories,
            commands::flush_dns,
            commands::get_proxy_list,
            commands::check_proxy_ping,
            commands::start_tor,
            commands::stop_tor,
            commands::check_tor_status,
            commands::enable_system_proxy,
            commands::disable_system_proxy,
            commands::start_hotspot,
            commands::stop_hotspot,
            commands::get_hotspot_status,
        ])
        .setup(|app| {
            let tauri_handle = app.handle().clone();
            // ядро сообщает о событиях через Hub — пересылаем их в webview
            let emitter = tauri_handle.clone();
            zust_core::set_hub(zust_core::Hub::new(move |event, payload| {
                let _ = emitter.emit(event, payload);
            }));
            let handle = zust_core::hub();
            paths::init(
                app.path()
                    .app_config_dir()
                    .unwrap_or_else(|_| std::env::temp_dir().join("zust")),
                app.path().resource_dir().unwrap_or_default(),
            );
            logger::init();
            settings::migrate_legacy();

            let about = commands::app_info();
            info(
                &handle,
                &format!(
                    "Zust v{} ({}, {}) — {} {}",
                    about.version, about.commit, about.build_date, about.os, about.arch
                ),
            );

            if let Err(e) = platform::register_url_protocol() {
                info(
                    &handle,
                    &format!("Не удалось зарегистрировать zust://: {e}"),
                );
            }

            let args: Vec<String> = std::env::args().collect();
            let autostarted = args.iter().any(|a| a == AUTOSTART_ARG || a == "--silent");
            if autostarted {
                // без заставки: главное окно остаётся скрытым в трее
                if let Some(splash) = app.get_webview_window("splash") {
                    let _ = splash.close();
                }
            }
            if let Some(url) = args.iter().find(|a| a.starts_with("zust://")) {
                utils::process_incoming_url(&handle, url);
            }

            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                let s = settings::load();
                // синхронизируем автозапуск с настройкой (переход со старого способа)
                let auto = s.auto_start;
                let _ = tauri::async_runtime::spawn_blocking(move || platform::set_autostart(auto))
                    .await;
                if s.auto_tor
                    && !Tor::is_running()
                    && let Err(e) = Tor::start(h.clone()).await
                {
                    info(&h, &format!("[TOR] Автозапуск не удался: {e}"));
                }
            });

            let show_i = MenuItem::with_id(app, "show", "Открыть Zust", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            TrayIconBuilder::with_id("zust_tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip(format!("Zust v{}", about.version))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => quit(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
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
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if settings::load().minimize_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                        let _ = window.emit("window-visible", false);
                    } else {
                        quit(window.app_handle());
                    }
                }
                tauri::WindowEvent::Resized(_) => {
                    if let Ok(minimized) = window.is_minimized() {
                        let _ = window.emit("window-visible", !minimized);
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
