#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Timer, TimerMode};
use std::path::PathBuf;
use std::time::Duration;

slint::include_modules!();

mod backend;
#[cfg(target_os = "macos")]
mod mac_links;
mod notify;
mod scare;
mod single;
mod styles;

/// Настройки интерфейса v3. Общие с Zust 2 настройки (автозапуск, трей, GameFilter…)
/// хранит ядро в своём settings.json.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Config {
    /// None - ещё не выбирали, тогда "Классика"
    style: Option<i32>,
    accent: Option<u32>,
    light: bool,
    no_animations: bool,
    nav_right: bool,
    /// 0 - авто, 1 - GPU, 2 - без GPU
    renderer: i32,
    /// выбранная стратегия (имя файла .zapret)
    pub strategy: Option<String>,
    /// ipset для zapret: none / any / имя файла
    pub ipset: Option<String>,
    /// TOR: 0 - фоновый, 1 - системный прокси
    pub tor_mode: i32,
    /// уже спрашивали про ярлык в меню приложений
    pub launcher_asked: bool,
    /// последняя показанная новость (чтобы не открывать её при каждом запуске)
    pub news_seen: Option<String>,
    /// свой стиль поверх темы
    custom: styles::Custom,
}

/// Классика
const DEFAULT_STYLE: i32 = 3;

fn config_path() -> PathBuf {
    zust_core::paths::data_dir().join("ui-v3.json")
}

impl Config {
    fn load() -> Self {
        std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) {
        let _ = std::fs::write(
            config_path(),
            serde_json::to_string_pretty(self).unwrap_or_default(),
        );
    }
}

/// Выбор способа отрисовки. ZUST_RENDERER (femtovg/software) важнее настройки:
/// так "авто" откатывается на программный режим, если OpenGL не поднялся.
fn select_renderer(cfg: &Config) -> &'static str {
    let forced = std::env::var("ZUST_RENDERER").ok();
    let name = match (forced.as_deref(), cfg.renderer) {
        (Some("software"), _) | (_, 2) => "software",
        _ => "femtovg",
    };
    let ok = slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name(name.into())
        .select()
        .is_ok();
    if !ok && name != "software" {
        let _ = slint::BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("software".into())
            .select();
        return "software";
    }
    name
}

/// Резидентная память процесса (Linux). На других ОС - прочерк.
fn memory_mb() -> String {
    #[cfg(target_os = "linux")]
    if let Ok(s) = std::fs::read_to_string("/proc/self/statm")
        && let Some(pages) = s
            .split_whitespace()
            .nth(1)
            .and_then(|p| p.parse::<u64>().ok())
    {
        return format!("{} МБ", pages * 4096 / 1024 / 1024);
    }
    "-".into()
}

fn main() -> Result<(), slint::PlatformError> {
    // Windows: zapret (WinDivert), hosts и службы требуют прав администратора
    zust_core::platform::ensure_admin();
    // Zust уже запущен - он покажет окно (и обработает ссылку zust://), а мы выходим
    let instance = single::acquire();
    if matches!(instance, single::Instance::Secondary) {
        return Ok(());
    }
    zust_core::platform::prepare_environment();
    zust_core::paths::init(backend::data_dir(), backend::resource_dir());
    zust_core::logger::init();
    zust_core::settings::migrate_legacy();
    // ссылки zust:// ведут на этот exe (на Windows - и поверх записи Zust 2)
    std::thread::spawn(|| {
        if let Err(e) = zust_core::platform::register_url_protocol() {
            eprintln!("zust:// не зарегистрирован: {e}");
        }
    });

    let cfg = Config::load();
    let renderer = select_renderer(&cfg);
    // app id окна на Wayland/X11 - совпадает с ярлыком zust.desktop (значок, группировка)
    let _ = slint::set_xdg_app_id("zust");

    let ui = match AppWindow::new() {
        Ok(ui) => ui,
        // OpenGL есть, но окно не создалось (старый драйвер и т.п.) - перезапуск без GPU
        Err(e) if renderer != "software" && std::env::var_os("ZUST_RENDERER").is_none() => {
            eprintln!("GPU-отрисовка недоступна ({e}), перезапуск в программном режиме");
            drop(instance); // порт нужен новому процессу
            let exe = std::env::current_exe().expect("current_exe");
            let status = std::process::Command::new(exe)
                .args(std::env::args_os().skip(1))
                .env("ZUST_RENDERER", "software")
                .status();
            std::process::exit(status.map(|s| s.code().unwrap_or(1)).unwrap_or(1));
        }
        Err(e) => return Err(e),
    };

    // ---- применяем настройки ----
    let theme = ui.global::<Theme>();
    theme.set_style(cfg.style.unwrap_or(DEFAULT_STYLE).clamp(0, 4));
    theme.set_dark(!cfg.light);
    theme.set_animations(!cfg.no_animations);
    theme.set_nav_side(cfg.nav_right as i32);
    if let Some(a) = cfg.accent {
        theme.set_accent(slint::Color::from_argb_encoded(a));
    }

    styles::connect(&ui, &cfg.custom);
    let be = ui.global::<Backend>();
    be.set_renderer(
        match renderer {
            "software" => "программная отрисовка",
            _ => "OpenGL (видеокарта)",
        }
        .into(),
    );
    be.set_renderer_choice(cfg.renderer.clamp(0, 2));
    be.set_version(concat!("v", env!("CARGO_PKG_VERSION")).into());

    // ---- настоящий бэкенд: zapret, TOR, прокси, точка доступа ----
    let _timers = backend::connect(&ui, cfg);
    zust_core::logger::info(&zust_core::hub(), &format!("Отрисовка: {renderer}"));

    // ---- сохранение настроек ----
    let weak = ui.as_weak();
    be.on_settings_changed(move || {
        let Some(ui) = weak.upgrade() else { return };
        let t = ui.global::<Theme>();
        let b = ui.global::<Backend>();
        backend::update_cfg(|c| {
            // пока тему не меняли, не запоминаем: иначе застынет текущая тема по умолчанию
            if c.style.is_some() || t.get_style() != DEFAULT_STYLE {
                c.style = Some(t.get_style());
            }
            c.accent = Some(t.get_accent().as_argb_encoded());
            c.light = !t.get_dark();
            c.no_animations = !t.get_animations();
            c.nav_right = t.get_nav_side() == 1;
            c.renderer = b.get_renderer_choice();
            c.custom = styles::read(&ui);
        });
        backend::apply_core_settings(&ui);
    });

    // ---- память ----
    be.set_memory(memory_mb().into());
    let mem_timer = Timer::default();
    {
        let weak = ui.as_weak();
        mem_timer.start(TimerMode::Repeated, Duration::from_secs(2), move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Backend>().set_memory(memory_mb().into());
            }
        });
    }

    // ---- трей ----
    let tray = match ZustTray::new() {
        Ok(t) => Some(t),
        Err(e) => {
            zust_core::logger::info(&zust_core::hub(), &format!("Трей недоступен: {e}"));
            None
        }
    };
    let tray_timer = Timer::default();
    if let Some(tray) = &tray {
        tray.set_mac(cfg!(target_os = "macos"));
        tray.on_open({
            let weak = ui.as_weak();
            move || {
                if let Some(ui) = weak.upgrade() {
                    show_window(&ui);
                }
            }
        });
        tray.on_toggle_zapret({
            let weak = ui.as_weak();
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.global::<Backend>().invoke_toggle();
                }
            }
        });
        tray.on_toggle_tor({
            let weak = ui.as_weak();
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.global::<Backend>().invoke_tor_toggle();
                }
            }
        });
        tray.on_quit(|| {
            let _ = slint::quit_event_loop();
        });
        // состояние в меню трея - из того же Backend, что и у окна
        let (weak, tray_weak) = (ui.as_weak(), tray.as_weak());
        tray_timer.start(TimerMode::Repeated, Duration::from_millis(700), move || {
            let (Some(ui), Some(tray)) = (weak.upgrade(), tray_weak.upgrade()) else {
                return;
            };
            let b = ui.global::<Backend>();
            tray.set_zapret_on(b.get_state() == 2);
            tray.set_tor_on(b.get_tor_state() != 0);
            let name =
                slint::Model::row_data(&b.get_strategies(), b.get_strategy().max(0) as usize);
            tray.set_strategy(name.unwrap_or_default());
        });
    }

    // ---- закрытие окна: в трей или выход (настройка "Сворачивать в трей") ----
    let has_tray = tray.is_some();
    ui.window().on_close_requested({
        let weak = ui.as_weak();
        move || {
            let to_tray = has_tray
                && weak
                    .upgrade()
                    .is_some_and(|ui| ui.global::<Backend>().get_tray());
            if !to_tray {
                let _ = slint::quit_event_loop();
            }
            slint::CloseRequestResponse::HideWindow
        }
    });

    // ---- повторный запуск и ссылки zust:// ----
    if let single::Instance::Primary(listener) = instance {
        let weak = ui.as_weak();
        single::listen(listener, move |args| {
            let _ = weak.upgrade_in_event_loop(move |ui| {
                show_window(&ui);
                handle_link(&ui, &args);
            });
        });
    }
    #[cfg(target_os = "macos")]
    mac_links::listen({
        let weak = ui.as_weak();
        move |url| {
            if let Some(ui) = weak.upgrade() {
                show_window(&ui);
                handle_link(&ui, &[url]);
            }
        }
    });
    let args: Vec<String> = std::env::args().skip(1).collect();
    handle_link(&ui, &args);

    // при входе в систему стартуем свёрнутыми в трей (если трей есть)
    let autostarted = args
        .iter()
        .any(|a| a == zust_core::AUTOSTART_ARG || a == "--silent");
    if !(autostarted && has_tray) {
        ui.show()?;
    }
    let r = slint::run_event_loop();
    drop(tray);
    backend::shutdown();
    r
}

/// Ссылка zust:// из аргументов - окно подтверждения (backend::handle_link).
fn handle_link(ui: &AppWindow, args: &[String]) {
    if let Some(url) = args.iter().find(|a| a.starts_with("zust://")) {
        backend::handle_link(ui, url);
    }
}

fn show_window(ui: &AppWindow) {
    let _ = ui.show();
    ui.window().set_minimized(false);
}
