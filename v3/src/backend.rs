//! Связь интерфейса с ядром: долгое - на tokio, результат - в главный поток через `upgrade_in_event_loop`.

use crate::notify::{self, Kind};
use crate::{AppWindow, Backend, Config, HostsCategory, LinkRow, Person, ProxyItem, TestItem, Ui};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel, Weak};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use zust_core::bypass::comss::Comss;
use zust_core::bypass::hosts::Hosts;
use zust_core::bypass::hotspot::HotSpot;
use zust_core::bypass::proxies::Proxies;
use zust_core::bypass::tor::Tor;
use zust_core::bypass::zapret::{TestRow, UserList, Zapret};
use zust_core::{Hub, hub, info, logger, platform, settings};

const TG_LOCAL: &str = "tg://socks?server=127.0.0.1&port=9050";
const FRP_SERVER: &str = "frp.freefrp.net";
const MAX_LOG_LINES: usize = 1000;
/// Значок для ярлыка в меню приложений (Linux).
pub const ICON_PNG: &[u8] = include_bytes!("../../src-tauri/icons/128x128@2x.png");
/// Как часто сверяемся с системой: zapret могут остановить извне (служба, другая программа).
const POLL: Duration = Duration::from_secs(3);

// ---------- инфраструктура ----------

static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
/// Идёт запуск/остановка zapret - периодическая проверка не должна перебивать состояние.
static ZAPRET_BUSY: AtomicBool = AtomicBool::new(false);

fn rt() -> &'static tokio::runtime::Runtime {
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

pub fn spawn<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    rt().spawn(f);
}

/// Выполнить блокирующее действие в фоне, а результат применить к интерфейсу.
fn blocking<T: Send + 'static>(
    weak: &Weak<AppWindow>,
    work: impl FnOnce() -> T + Send + 'static,
    then: impl FnOnce(AppWindow, T) + Send + 'static,
) {
    let weak = weak.clone();
    rt().spawn_blocking(move || {
        let r = work();
        let _ = weak.upgrade_in_event_loop(move |ui| then(ui, r));
    });
}

fn log(text: impl AsRef<str>) {
    logger::info(&hub(), text.as_ref());
}

/// Ссылки открываются только в браузере или Telegram - как в Zust 2.
fn open_link(url: &str) {
    let ok = ["https://", "http://", "tg://"]
        .iter()
        .any(|p| url.to_lowercase().starts_with(p));
    if !ok {
        log(format!("Ссылка не открыта (неподдерживаемая схема): {url}"));
        return;
    }
    if let Err(e) = open::that_detached(url) {
        log(format!("Не удалось открыть ссылку: {e}"));
    }
}

// ---------- состояние интерфейса (только главный поток) ----------

#[derive(Default)]
struct State {
    cfg: Config,
    /// Файлы стратегий (`general.zapret`), в том же порядке, что и Backend.strategies.
    files: Vec<String>,
    /// Ссылки на прокси (tg://proxy?...), в том же порядке, что и Backend.proxies.
    proxy_links: Vec<String>,
    started_at: Option<Instant>,
    core: settings::Settings,
    /// Значения ipset (none / any / ipset-all.txt / свои файлы), как Backend.ipsets.
    ipsets: Vec<String>,
    /// Файлы стратегий, для которых есть обновление в репозитории.
    updates: Vec<String>,
    /// Категории hosts из последней загрузки: название → строки.
    hosts_map: std::collections::HashMap<String, Vec<String>>,
    /// Что ждёт подтверждения в окне ссылки zust://.
    pending: Option<LinkAction>,
    /// id новости в окне "Новости" - запоминается как прочитанная при закрытии
    news_id: Option<String>,
    /// Флаг остановки идущего теста стратегий.
    test_cancel: Option<Arc<AtomicBool>>,
    test_rows: Option<Rc<VecModel<TestItem>>>,
}

/// Действие из ссылки zust://, которое выполнится после подтверждения.
#[derive(Clone)]
enum LinkAction {
    Add {
        kind: zust_core::utils::AddType,
        url: String,
        name: Option<String>,
    },
    Configure {
        strategy: String,
        ipset: Option<String>,
        game_filter: Option<bool>,
    },
    Restore,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn display_name(file: &str) -> SharedString {
    file.strip_suffix(".zapret").unwrap_or(file).into()
}

/// Папка данных - та же, что у Zust 2 (Tauri): настройки, рабочие файлы zapret и лог общие.
pub fn data_dir() -> std::path::PathBuf {
    use std::path::PathBuf;
    const ID: &str = "com.f1zzen.zust";
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base =
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"));
    #[cfg(target_os = "linux")]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.map(|b| b.join(ID))
        .unwrap_or_else(|| std::env::temp_dir().join("zust"))
}

/// Ресурсы (zapret, tor, frp): ZUST_RESOURCES → рядом с exe → src-tauri при сборке из исходников.
pub fn resource_dir() -> std::path::PathBuf {
    if let Some(p) = std::env::var_os("ZUST_RESOURCES") {
        return p.into();
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|p| p.to_path_buf()))
        && dir.join("zapret").is_dir()
    {
        return dir;
    }
    // Z.UST.app/Contents/MacOS/zust → Contents/Resources
    #[cfg(target_os = "macos")]
    if let Some(res) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent()?.parent().map(|c| c.join("Resources")))
        && res.join("zapret").is_dir()
    {
        return res;
    }
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri")
}

// ---------- запуск ----------

/// Подключает интерфейс к ядру. Вызывать один раз после создания окна.
/// Возвращает таймеры, которые должны жить, пока работает окно.
pub fn connect(ui: &AppWindow, cfg: Config) -> Vec<Timer> {
    let core = settings::load();
    let be = ui.global::<Backend>();
    notify::init(ui);
    be.set_autostart(core.auto_start);
    be.set_tray(core.minimize_to_tray);
    be.set_notifications(core.notifications);
    be.set_game_filter(core.game_filter);
    be.set_tor_mode(cfg.tor_mode.clamp(0, 1));
    with_state(|s| {
        s.cfg = cfg;
        s.core = core;
    });

    // ---- события ядра → интерфейс ----
    let logs = Rc::new(VecModel::<SharedString>::default());
    be.set_logs(ModelRc::from(logs.clone()));
    let weak = ui.as_weak();
    zust_core::set_hub(Hub::new(move |event, payload| {
        let weak = weak.clone();
        match event {
            "log-event" => {
                let Some(line) = payload.as_str().map(str::to_owned) else {
                    return;
                };
                let _ = weak.upgrade_in_event_loop(move |ui| push_log(&ui, line));
            }
            "tor-progress" => {
                let p = payload.as_i64().unwrap_or(0) as i32;
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let b = ui.global::<Backend>();
                    if b.get_tor_state() == 1 {
                        b.set_tor_progress(p.clamp(0, 100));
                    }
                });
            }
            "zapret_repair" => {
                let _ = weak.upgrade_in_event_loop(|ui| {
                    log("[TOR] Мосты не получены. Возможно, zapret мешает TOR - попробуйте другую стратегию или выключите zapret на время подключения.");
                    notify::toast(&ui, Kind::Info, "Мосты TOR не получены. Возможно, мешает zapret: смените стратегию или выключите его на время подключения");
                });
            }
            _ => {}
        }
    }));

    log(format!(
        "Z.UST {} - {} {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    ));

    connect_zapret(ui);
    connect_test(ui);
    crate::scare::connect(ui);
    connect_extras(ui);
    connect_hosts(ui);
    connect_lists(ui);
    connect_about(ui);
    connect_links(ui);
    connect_tor(ui);
    connect_proxies(ui);
    startup(ui);

    // ---- таймеры ----
    let poll = Timer::default();
    {
        let weak = ui.as_weak();
        poll.start(TimerMode::Repeated, POLL, move || refresh(&weak));
    }
    let uptime = Timer::default();
    {
        let weak = ui.as_weak();
        uptime.start(TimerMode::Repeated, Duration::from_secs(5), move || {
            if let Some(ui) = weak.upgrade() {
                update_uptime(&ui);
            }
        });
    }
    vec![poll, uptime]
}

/// При закрытии окна: TOR и точка доступа останавливаются, zapret продолжает работать.
pub fn shutdown() {
    Tor::stop();
    HotSpot::stop();
}

fn push_log(ui: &AppWindow, line: String) {
    let b = ui.global::<Backend>();
    if let Some(rest) = line.strip_prefix("[TOR] ") {
        b.set_tor_log(rest.into());
    }
    let model = b.get_logs();
    if let Some(vm) = model.as_any().downcast_ref::<VecModel<SharedString>>() {
        if vm.row_count() >= MAX_LOG_LINES {
            vm.remove(0);
        }
        vm.push(line.into());
    }
}

/// Подготовка при старте: файлы zapret, список стратегий, текущее состояние, bin-файлы.
fn startup(ui: &AppWindow) {
    blocking(
        &ui.as_weak(),
        || {
            if let Err(e) = Zapret::sync_files(&hub()) {
                log(format!("Не удалось подготовить файлы zapret: {e}"));
            }
            let files = Zapret::list_strategies();
            let running = Zapret::current_strategy();
            let tor = Tor::is_running();
            if tor {
                Tor::replay_logs(&hub());
            }
            let ipset = Zapret::current_ipset().filter(|_| running.is_some());
            (
                files,
                running,
                tor,
                HotSpot::status(),
                ipset,
                Zapret::custom_ipset_files(),
            )
        },
        |ui, (files, running, tor, hotspot, ipset, custom)| {
            let b = ui.global::<Backend>();
            set_strategies(&ui, files, running.as_deref());
            if let Some(ipset) = ipset {
                with_state(|s| s.cfg.ipset = Some(ipset));
            }
            set_ipsets(&ui, custom);
            if running.is_some() {
                set_zapret_state(&ui, 2);
            }
            if tor {
                b.set_tor_state(2);
                b.set_tor_progress(100);
                b.set_tor(true);
            }
            // "Запускать TOR вместе с Zust" (настройка из Zust 2)
            if !tor && with_state(|s| s.core.auto_tor) {
                b.invoke_tor_toggle();
            }
            if let Some(port) = hotspot {
                b.set_hotspot(true);
                b.set_hotspot_address(format!("{FRP_SERVER}:{port}").into());
            }
            refresh(&ui.as_weak());
        },
    );
    check_updates(ui);
    check_app_update(ui);
    // обновление .bin-файлов для fake-tls/quic - в фоне, как в Zust 2
    spawn(async {
        match Zapret::update_tls_bin(&hub()).await {
            Ok(msg) => log(msg),
            Err(e) => log(format!("Обновление bin-файлов: {e}")),
        }
    });
}

fn set_strategies(ui: &AppWindow, files: Vec<String>, running: Option<&str>) {
    let b = ui.global::<Backend>();
    let names: Vec<SharedString> = files.iter().map(|f| display_name(f)).collect();
    // какую выбрать: запущенную → сохранённую → первую
    let wanted = running
        .map(str::to_owned)
        .or_else(|| with_state(|s| s.cfg.strategy.clone()));
    let idx = wanted
        .and_then(|w| {
            let w = w.strip_suffix(".zapret").unwrap_or(&w).to_lowercase();
            files
                .iter()
                .position(|f| f.strip_suffix(".zapret").unwrap_or(f).to_lowercase() == w)
        })
        .unwrap_or(0);
    b.set_strategies(ModelRc::new(VecModel::from(names)));
    b.set_strategy(idx as i32);
    with_state(|s| s.files = files);
}

// ---------- ipset, обновления стратегий, импорт, ярлык ----------

fn set_ipsets(ui: &AppWindow, custom: Vec<String>) {
    let mut values: Vec<String> = vec!["none".into(), "any".into(), "ipset-all.txt".into()];
    let mut names: Vec<SharedString> = vec![
        "Только домены".into(),
        "Все адреса".into(),
        "ipset-all".into(),
    ];
    for f in custom {
        if values.iter().any(|v| v.eq_ignore_ascii_case(&f)) {
            continue;
        }
        names.push(
            f.strip_suffix(".txt")
                .unwrap_or(&f)
                .replace('_', " ")
                .into(),
        );
        values.push(f);
    }
    let current = with_state(|s| s.cfg.ipset.clone()).unwrap_or_else(|| "none".into());
    let idx = values
        .iter()
        .position(|v| v.eq_ignore_ascii_case(&current))
        .unwrap_or(0);
    let b = ui.global::<Backend>();
    b.set_ipsets(ModelRc::new(VecModel::from(names)));
    b.set_ipset(idx as i32);
    with_state(|s| s.ipsets = values);
}

fn check_updates(ui: &AppWindow) {
    let weak = ui.as_weak();
    spawn(async move {
        let r = Zapret::check_strategy_updates().await;
        let _ = weak.upgrade_in_event_loop(move |ui| match r {
            Ok(list) => {
                if !list.is_empty() {
                    log(format!(
                        "Доступны обновления стратегий: {}",
                        list.join(", ")
                    ));
                    notify::toast(
                        &ui,
                        Kind::Info,
                        format!("Есть обновления стратегий: {}. Их можно скачать на вкладке \"Стратегии\"", list.len()),
                    );
                }
                let names: Vec<SharedString> = list.iter().map(|f| display_name(f)).collect();
                ui.global::<Backend>()
                    .set_updates(ModelRc::new(VecModel::from(names)));
                with_state(|s| s.updates = list);
            }
            Err(e) => log(format!("Проверка обновлений стратегий: {e}")),
        });
    });
}

/// Перечитать список стратегий с диска (после обновления или импорта).
fn reload_strategies(ui: &AppWindow) {
    let current = with_state(|s| {
        let idx = ui.global::<Backend>().get_strategy().max(0) as usize;
        s.files.get(idx).cloned()
    });
    blocking(&ui.as_weak(), Zapret::list_strategies, move |ui, files| {
        set_strategies(&ui, files, current.as_deref());
    });
}

fn connect_extras(ui: &AppWindow) {
    let be = ui.global::<Backend>();

    be.on_select_ipset({
        let weak = ui.as_weak();
        move |i| {
            let Some(ui) = weak.upgrade() else { return };
            let Some(value) = with_state(|s| s.ipsets.get(i.max(0) as usize).cloned()) else {
                return;
            };
            ui.global::<Backend>().set_ipset(i);
            with_state(|s| {
                s.cfg.ipset = Some(value.clone());
                s.cfg.save();
            });
            // ipset - часть аргументов движка: работающий zapret перезапускаем
            if ui.global::<Backend>().get_state() == 2 {
                log(format!("Фильтр по IP: {value} - перезапуск zapret"));
                start_zapret(&ui);
            }
        }
    });

    be.on_open_folder(|kind| {
        let path = match kind.as_str() {
            "strategies" | "ipset-configs" | "lists" => Zapret::path(&kind),
            "themes" => crate::styles::dir(),
            _ => return,
        };
        let _ = std::fs::create_dir_all(&path);
        if let Err(e) = open::that_detached(&path) {
            log(format!("Не удалось открыть папку: {e}"));
        }
    });

    be.on_apply_updates({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let list = with_state(|s| s.updates.clone());
            if list.is_empty() {
                return;
            }
            ui.global::<Backend>().set_updating(true);
            let weak = weak.clone();
            spawn(async move {
                let mut done = Vec::new();
                for name in &list {
                    match Zapret::apply_strategy_update(name).await {
                        Ok(()) => done.push(name.clone()),
                        Err(e) => log(format!("Стратегия {name} не обновлена: {e}")),
                    }
                }
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let b = ui.global::<Backend>();
                    b.set_updating(false);
                    log(format!("Обновлено стратегий: {}", done.len()));
                    if done.len() == list.len() {
                        notify::toast(
                            &ui,
                            Kind::Ok,
                            format!("Обновлено стратегий: {}", done.len()),
                        );
                    } else {
                        notify::toast(
                            &ui,
                            Kind::Error,
                            format!(
                                "Обновлено стратегий: {} из {}. Подробности на вкладке \"Логи\"",
                                done.len(),
                                list.len()
                            ),
                        );
                    }
                    let left: Vec<String> =
                        list.into_iter().filter(|n| !done.contains(n)).collect();
                    let names: Vec<SharedString> = left.iter().map(|f| display_name(f)).collect();
                    b.set_updates(ModelRc::new(VecModel::from(names)));
                    // если обновилась работающая стратегия - перезапускаем на новой версии
                    let idx = b.get_strategy().max(0) as usize;
                    let current = with_state(|s| {
                        s.updates = left;
                        s.files.get(idx).cloned()
                    });
                    let restart =
                        b.get_state() == 2 && current.as_ref().is_some_and(|c| done.contains(c));
                    reload_strategies(&ui);
                    if restart {
                        log("Работающая стратегия обновилась - перезапуск zapret");
                        start_zapret(&ui);
                    }
                });
            });
        }
    });

    be.on_import_bat({
        let weak = ui.as_weak();
        move || {
            let weak = weak.clone();
            spawn(async move {
                let Some(files) = rfd::AsyncFileDialog::new()
                    .set_title("Стратегии zapret (.bat)")
                    .add_filter("Стратегии", &["bat", "cmd"])
                    .pick_files()
                    .await
                else {
                    return;
                };
                let paths: Vec<String> = files
                    .iter()
                    .map(|f| f.path().to_string_lossy().into_owned())
                    .collect();
                let r = tokio::task::spawn_blocking(move || Zapret::convert_bats(paths))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r);
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    match r {
                        Ok(n) => {
                            log(format!("Импортировано стратегий: {n}"));
                            if n > 0 {
                                notify::toast(
                                    &ui,
                                    Kind::Ok,
                                    format!("Импортировано стратегий: {n}"),
                                );
                            } else {
                                notify::toast(
                                    &ui,
                                    Kind::Error,
                                    "В файлах не нашлось запуска winws.exe",
                                );
                            }
                        }
                        Err(e) => {
                            log(format!("Импорт не удался: {e}"));
                            notify::toast(&ui, Kind::Error, format!("Импорт не удался: {e}"));
                        }
                    }
                    reload_strategies(&ui);
                });
            });
        }
    });

    // ---- ярлык в меню приложений ----
    let supported = platform::launcher_supported();
    let installed = supported && platform::launcher_installed();
    if installed {
        platform::refresh_launcher(ICON_PNG);
    }
    be.set_launcher_supported(supported);
    be.set_launcher_installed(installed);
    be.set_launcher_offer(supported && !installed && !with_state(|s| s.cfg.launcher_asked));
    be.on_launcher_set({
        let weak = ui.as_weak();
        move |enable| {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            let r = if enable {
                platform::install_launcher(ICON_PNG)
            } else {
                platform::remove_launcher()
            };
            match r {
                Ok(()) => log(if enable {
                    "Ярлык Z.UST добавлен в меню приложений"
                } else {
                    "Ярлык Z.UST убран из меню приложений"
                }),
                Err(e) => {
                    log(format!("Ярлык: {e}"));
                    notify::toast(&ui, Kind::Error, format!("Ярлык: {e}"));
                }
            }
            b.set_launcher_installed(platform::launcher_installed());
            b.set_launcher_offer(false);
            update_cfg(|c| c.launcher_asked = true);
        }
    });
    be.on_launcher_dismiss({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Backend>().set_launcher_offer(false);
            }
            update_cfg(|c| c.launcher_asked = true);
        }
    });
}

// ---------- zapret ----------

fn set_zapret_state(ui: &AppWindow, state: i32) {
    let b = ui.global::<Backend>();
    with_state(|s| match state {
        2 if s.started_at.is_none() => s.started_at = Some(Instant::now()),
        0 | 3 => s.started_at = None,
        _ => {}
    });
    b.set_state(state);
    update_uptime(ui);
}

fn update_uptime(ui: &AppWindow) {
    let text = with_state(|s| s.started_at).map(|t| match t.elapsed().as_secs() / 60 {
        0 => "· только что".to_string(),
        m if m < 60 => format!("· {m} мин"),
        m => format!("· {} ч {} мин", m / 60, m % 60),
    });
    ui.global::<Backend>()
        .set_uptime(text.unwrap_or_default().into());
}

/// Сверяет состояние интерфейса с системой (zapret, TOR, hosts).
fn refresh(weak: &Weak<AppWindow>) {
    if ZAPRET_BUSY.load(Ordering::SeqCst) {
        return;
    }
    blocking(
        weak,
        || {
            let zapret = Zapret::is_running();
            let tor = Tor::is_running();
            let hosts = !Hosts::get_installed_categories().is_empty();
            (zapret, tor, hosts)
        },
        |ui, (zapret, tor, hosts)| {
            let b = ui.global::<Backend>();
            if !ZAPRET_BUSY.load(Ordering::SeqCst) {
                let st = b.get_state();
                if zapret && st != 2 {
                    set_zapret_state(&ui, 2);
                } else if !zapret && st == 2 {
                    log("zapret остановлен извне");
                    notify::show(&ui, Kind::Error, "zapret перестал работать, обход выключен");
                    set_zapret_state(&ui, 0);
                } else if !zapret && st == 3 {
                    // ошибка показана - возвращаемся в "выключен"
                    set_zapret_state(&ui, 0);
                }
            }
            b.set_hosts(hosts);
            b.set_tor(tor);
            // TOR умер сам (или его остановили) - интерфейс не должен показывать "работает"
            if !tor && b.get_tor_state() == 2 {
                notify::show(&ui, Kind::Error, "TOR отключился");
                b.set_tor_state(0);
                b.set_tor_progress(0);
                b.set_hotspot(false);
            }
        },
    );
}

fn start_zapret(ui: &AppWindow) {
    let b = ui.global::<Backend>();
    let idx = b.get_strategy().max(0) as usize;
    let Some(file) = with_state(|s| s.files.get(idx).cloned()) else {
        log("Нет стратегий - добавьте файл .zapret в папку стратегий");
        return;
    };
    let ipset = with_state(|s| s.cfg.ipset.clone()).unwrap_or_else(|| "none".into());
    ZAPRET_BUSY.store(true, Ordering::SeqCst);
    set_zapret_state(ui, 1);
    with_state(|s| s.started_at = None);
    let weak = ui.as_weak();
    spawn(async move {
        let name = display_name(&file);
        let r = Zapret::start(&hub(), &file, Some(ipset)).await;
        let running = tokio::task::spawn_blocking(Zapret::is_running)
            .await
            .unwrap_or(false);
        let _ = weak.upgrade_in_event_loop(move |ui| {
            ZAPRET_BUSY.store(false, Ordering::SeqCst);
            match r {
                Ok(_) if running => {
                    set_zapret_state(&ui, 2);
                    notify::background(&ui, Kind::Ok, format!("zapret включён: {name}"));
                }
                Ok(_) => {
                    log("zapret не запустился - подробности выше в логе");
                    notify::show(
                        &ui,
                        Kind::Error,
                        "zapret не запустился. Подробности на вкладке \"Логи\"",
                    );
                    set_zapret_state(&ui, 3);
                }
                Err(e) => {
                    log(format!("Ошибка запуска zapret: {e}"));
                    notify::show(&ui, Kind::Error, format!("zapret не запустился: {e}"));
                    set_zapret_state(&ui, 3);
                }
            }
        });
    });
}

fn stop_zapret(ui: &AppWindow) {
    ZAPRET_BUSY.store(true, Ordering::SeqCst);
    set_zapret_state(ui, 1);
    let weak = ui.as_weak();
    spawn(async move {
        let r = Zapret::stop(&hub()).await;
        let running = tokio::task::spawn_blocking(Zapret::is_running)
            .await
            .unwrap_or(false);
        let _ = weak.upgrade_in_event_loop(move |ui| {
            ZAPRET_BUSY.store(false, Ordering::SeqCst);
            if let Err(e) = r {
                log(format!("Ошибка при остановке zapret: {e}"));
                notify::show(&ui, Kind::Error, format!("zapret не остановился: {e}"));
            } else {
                log("zapret остановлен");
                notify::background(&ui, Kind::Info, "zapret выключен");
            }
            set_zapret_state(&ui, if running { 2 } else { 0 });
        });
    });
}

fn connect_zapret(ui: &AppWindow) {
    let be = ui.global::<Backend>();
    be.on_toggle({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            match ui.global::<Backend>().get_state() {
                1 => {}
                2 => stop_zapret(&ui),
                _ => start_zapret(&ui),
            }
        }
    });
    be.on_restart({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade()
                && ui.global::<Backend>().get_state() != 1
            {
                start_zapret(&ui);
            }
        }
    });
    be.on_select_strategy({
        let weak = ui.as_weak();
        move |i| {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            let n = with_state(|s| s.files.len()) as i32;
            if n == 0 {
                return;
            }
            let i = i.rem_euclid(n);
            b.set_strategy(i);
            with_state(|s| {
                s.cfg.strategy = s.files.get(i as usize).cloned();
                s.cfg.save();
            });
            // если zapret работает - сразу перезапускаем на новой стратегии
            if b.get_state() == 2 {
                start_zapret(&ui);
            }
        }
    });
}

// ---------- тест стратегий ----------

fn test_item(r: &TestRow, targets: usize, tls13: bool, best: bool) -> TestItem {
    let n = targets;
    let detail = match &r.error {
        Some(e) => format!("не запустилась: {e}"),
        None => {
            let mut d = format!("HTTP {}/{n} · TLS 1.2 {}/{n}", r.http, r.tls12);
            if tls13 {
                d.push_str(&format!(" · TLS 1.3 {}/{n}", r.tls13));
            }
            if r.avg_ms > 0 {
                d.push_str(&format!(" · {} мс", r.avg_ms));
            }
            d
        }
    };
    let total = n * if tls13 { 3 } else { 2 };
    TestItem {
        name: r.strategy.clone().into(),
        score: format!("{}/{total}", r.score()).into(),
        detail: detail.into(),
        best,
        failed: r.error.is_some(),
    }
}

fn start_test(ui: &AppWindow) {
    let b = ui.global::<Backend>();
    if b.get_test_running() || b.get_state() == 1 {
        return;
    }
    let files = with_state(|s| s.files.clone());
    if files.is_empty() {
        return;
    }
    let ipset = with_state(|s| s.cfg.ipset.clone()).unwrap_or_else(|| "none".into());
    let cancel = Arc::new(AtomicBool::new(false));
    let rows = Rc::new(VecModel::<TestItem>::default());
    b.set_test_rows(ModelRc::from(rows.clone()));
    with_state(|s| {
        s.test_cancel = Some(cancel.clone());
        s.test_rows = Some(rows);
    });
    b.set_test_running(true);
    b.set_test_progress(0.0);
    b.set_test_status("Подготовка…".into());
    ZAPRET_BUSY.store(true, Ordering::SeqCst);
    set_zapret_state(ui, 1);
    log(format!("Тест стратегий: {} шт.", files.len()));

    let weak = ui.as_weak();
    spawn(async move {
        let progress = {
            let weak = weak.clone();
            move |p: zust_core::bypass::zapret::Progress| {
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let b = ui.global::<Backend>();
                    if !b.get_test_running() {
                        return;
                    }
                    b.set_test_progress(p.done as f32 / p.total.max(1) as f32);
                    b.set_test_status(
                        if p.row.is_some() || p.targets == 0 {
                            format!("{} ({}/{})", p.current, p.done, p.total)
                        } else {
                            format!("Проверка: {} ({}/{})", p.current, p.done + 1, p.total)
                        }
                        .into(),
                    );
                    if let Some(row) = p.row {
                        let item = test_item(&row, p.targets, true, false);
                        with_state(|s| {
                            if let Some(m) = &s.test_rows {
                                m.push(item);
                            }
                        });
                    }
                });
            }
        };
        let r = Zapret::test_strategies(&hub(), files, ipset, cancel, progress).await;
        let running = tokio::task::spawn_blocking(Zapret::is_running)
            .await
            .unwrap_or(false);
        let _ = weak.upgrade_in_event_loop(move |ui| {
            let b = ui.global::<Backend>();
            ZAPRET_BUSY.store(false, Ordering::SeqCst);
            set_zapret_state(&ui, if running { 2 } else { 0 });
            b.set_test_running(false);
            with_state(|s| s.test_cancel = None);
            match r {
                Ok(rep) => {
                    let n = rep.targets as usize;
                    let best = rep.rows.first().filter(|r| r.score() > 0);
                    let items: Vec<TestItem> = rep
                        .rows
                        .iter()
                        .enumerate()
                        .map(|(i, r)| test_item(r, n, rep.tls13, i == 0 && best.is_some()))
                        .collect();
                    if let Some(m) = with_state(|s| s.test_rows.clone()) {
                        m.set_vec(items);
                    }
                    let status = match best {
                        _ if rep.cancelled => "Тест остановлен. Результаты - по проверенным стратегиям, нажмите на любую, чтобы выбрать её.".to_string(),
                        Some(r) => format!(
                            "Лучшая: {} ({} из {}). Нажмите на стратегию, чтобы выбрать её.",
                            r.strategy,
                            r.score(),
                            rep.total()
                        ),
                        None => "Ни одна стратегия не открыла Discord и YouTube. Попробуйте другой фильтр по IP или TOR.".into(),
                    };
                    log(format!("Тест стратегий: {status}"));
                    b.set_test_status(status.into());
                    notify::background(&ui, Kind::Ok, "Тест стратегий завершён");
                }
                Err(e) => {
                    log(format!("Тест стратегий: {e}"));
                    b.set_test_status(e.clone().into());
                    notify::show(&ui, Kind::Error, format!("Тест не выполнен: {e}"));
                }
            }
        });
    });
}

fn connect_test(ui: &AppWindow) {
    let be = ui.global::<Backend>();
    be.on_test_open({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.global::<Ui>().set_dialog(5);
            }
        }
    });
    be.on_test_start({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                start_test(&ui);
            }
        }
    });
    be.on_test_stop({
        let weak = ui.as_weak();
        move || {
            if let Some(c) = with_state(|s| s.test_cancel.clone()) {
                c.store(true, Ordering::SeqCst);
            }
            if let Some(ui) = weak.upgrade() {
                ui.global::<Backend>()
                    .set_test_status("Останавливаем после текущей стратегии…".into());
            }
        }
    });
    be.on_test_apply({
        let weak = ui.as_weak();
        move |name| {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            let Some(i) = with_state(|s| s.files.iter().position(|f| display_name(f) == name))
            else {
                return;
            };
            b.invoke_select_strategy(i as i32);
            // выключенный zapret включаем: стратегию выбирают, чтобы ей пользоваться
            if matches!(b.get_state(), 0 | 3) {
                start_zapret(&ui);
            }
            ui.global::<Ui>().set_dialog(0);
            notify::toast(&ui, Kind::Ok, format!("Выбрана стратегия {name}"));
        }
    });
}

// ---------- TOR ----------

fn connect_tor(ui: &AppWindow) {
    let be = ui.global::<Backend>();
    be.on_tor_toggle({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            if b.get_tor_state() != 0 {
                b.set_tor_state(0);
                b.set_tor_progress(0);
                b.set_tor(false);
                b.set_hotspot(false);
                b.set_tor_log("Остановка…".into());
                blocking(
                    &weak,
                    || {
                        HotSpot::stop();
                        Tor::stop();
                    },
                    |ui, ()| {
                        ui.global::<Backend>().set_tor_log("Готов к работе".into());
                        log("[TOR] остановлен");
                        notify::background(&ui, Kind::Info, "TOR выключен");
                    },
                );
                return;
            }
            b.set_tor_state(1);
            b.set_tor_progress(0);
            b.set_tor_log("Получение мостов…".into());
            let weak = weak.clone();
            spawn(async move {
                let r = Tor::start(hub()).await;
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let b = ui.global::<Backend>();
                    // пользователь уже нажал "стоп" - результат запуска не нужен
                    if b.get_tor_state() != 1 {
                        return;
                    }
                    match r {
                        Ok(()) => {
                            b.set_tor_state(2);
                            b.set_tor_progress(100);
                            b.set_tor(true);
                            notify::show(&ui, Kind::Ok, "TOR подключён");
                            if b.get_tor_mode() == 1 {
                                set_system_proxy(&ui, true);
                            }
                        }
                        Err(e) => {
                            b.set_tor_state(0);
                            b.set_tor_progress(0);
                            if e != "interrupted" {
                                b.set_tor_log(format!("Ошибка: {e}").into());
                                log(format!("[TOR] не запустился: {e}"));
                                notify::show(&ui, Kind::Error, format!("TOR не подключился: {e}"));
                                rt().spawn_blocking(Tor::stop);
                            }
                        }
                    }
                });
            });
        }
    });
    be.on_tor_mode_changed({
        let weak = ui.as_weak();
        move |m| {
            let Some(ui) = weak.upgrade() else { return };
            with_state(|s| {
                s.cfg.tor_mode = m;
                s.cfg.save();
            });
            if ui.global::<Backend>().get_tor_state() == 2 {
                set_system_proxy(&ui, m == 1);
            }
        }
    });
    be.on_tg_connect_local(|| open_link(TG_LOCAL));
    be.on_hotspot_toggle({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            // Switch уже поменял значение (двусторонняя привязка)
            if b.get_hotspot() {
                b.set_hotspot_address("запуск…".into());
                blocking(
                    &weak,
                    || HotSpot::run(&hub(), None),
                    |ui, r| {
                        let b = ui.global::<Backend>();
                        match r {
                            Ok(port) => {
                                b.set_hotspot_address(format!("{FRP_SERVER}:{port}").into());
                                notify::toast(&ui, Kind::Ok, "Точка доступа запущена");
                            }
                            Err(e) => {
                                b.set_hotspot(false);
                                log(format!("[FRP] Точка доступа не запустилась: {e}"));
                                notify::show(
                                    &ui,
                                    Kind::Error,
                                    format!("Точка доступа не запустилась: {e}"),
                                );
                            }
                        }
                    },
                );
            } else {
                rt().spawn_blocking(HotSpot::stop);
                log("[FRP] точка доступа выключена");
            }
        }
    });
}

fn set_system_proxy(ui: &AppWindow, enable: bool) {
    blocking(
        &ui.as_weak(),
        move || platform::set_system_proxy(enable),
        move |ui, r| {
            if let Err(e) = r {
                log(format!("[TOR] системный прокси: {e}"));
                notify::show(
                    &ui,
                    Kind::Error,
                    format!("Системный прокси не включился: {e}"),
                );
                if enable {
                    ui.global::<Backend>().set_tor_mode(0);
                }
            }
        },
    );
}

// ---------- прокси для Telegram ----------

/// tg://proxy?server=host&port=443&secret=… → host:443
fn proxy_address(link: &str) -> String {
    url::Url::parse(link)
        .ok()
        .and_then(|u| {
            let q: std::collections::HashMap<_, _> = u.query_pairs().into_owned().collect();
            let server = q.get("server")?.clone();
            let port = q.get("port").cloned().unwrap_or_else(|| "443".into());
            Some(format!("{server}:{port}"))
        })
        .unwrap_or_else(|| link.to_string())
}

fn connect_proxies(ui: &AppWindow) {
    let be = ui.global::<Backend>();
    let model = Rc::new(VecModel::<ProxyItem>::default());
    be.set_proxies(ModelRc::from(model.clone()));

    // список прокси - из gist, как в Zust 2; после загрузки сразу проверяем пинг
    let weak = ui.as_weak();
    spawn(async move {
        let list = Proxies::get_proxy_list().await.map_err(|e| e.to_string());
        let _ = weak.upgrade_in_event_loop(move |ui| match list {
            Ok(links) => {
                let items: Vec<ProxyItem> = links
                    .iter()
                    .map(|l| ProxyItem {
                        country: "··".into(),
                        address: proxy_address(l).into(),
                        ping: 0,
                    })
                    .collect();
                with_state(|s| s.proxy_links = links);
                ui.global::<Backend>()
                    .set_proxies(ModelRc::new(VecModel::from(items)));
                ui.global::<Backend>().invoke_proxies_check();
            }
            Err(e) => log(format!("Список прокси не загружен: {e}")),
        });
    });

    be.on_proxies_check({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let links = with_state(|s| s.proxy_links.clone());
            if links.is_empty() {
                return;
            }
            ui.global::<Backend>().set_proxies_checking(true);
            let weak = weak.clone();
            spawn(async move {
                let checks = links.iter().map(|l| Proxies::check_proxy_ping(l, hub()));
                let results = futures::future::join_all(checks).await;
                let results: Vec<Option<(i32, String)>> = results
                    .into_iter()
                    .map(|r| r.map(|p| (p.ping.min(9999) as i32, p.country_code)))
                    .collect();
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let b = ui.global::<Backend>();
                    let model = b.get_proxies();
                    for (i, r) in results.into_iter().enumerate() {
                        let Some(mut item) = model.row_data(i) else {
                            continue;
                        };
                        match r {
                            Some((ping, country)) => {
                                item.ping = ping.max(1);
                                item.country = country.into();
                            }
                            None => item.ping = -1,
                        }
                        model.set_row_data(i, item);
                    }
                    b.set_proxies_checking(false);
                });
            });
        }
    });
    be.on_proxy_connect(|i| {
        if let Some(link) = with_state(|s| s.proxy_links.get(i as usize).cloned()) {
            open_link(&link);
        }
    });
}

// ---------- настройки ----------

/// Изменить и сохранить настройки интерфейса (тема, акцент, отрисовка, выбор стратегии).
pub fn update_cfg(f: impl FnOnce(&mut Config)) {
    with_state(|s| {
        f(&mut s.cfg);
        s.cfg.save();
    });
}

/// Сохраняет настройки, общие с Zust 2 (settings.json ядра), и применяет то, что изменилось.
pub fn apply_core_settings(ui: &AppWindow) {
    let b = ui.global::<Backend>();
    let new = settings::Settings {
        auto_start: b.get_autostart(),
        minimize_to_tray: b.get_tray(),
        notifications: b.get_notifications(),
        game_filter: b.get_game_filter(),
        ..with_state(|s| s.core.clone())
    };
    let old = with_state(|s| std::mem::replace(&mut s.core, new.clone()));
    if let Err(e) = settings::save(&new) {
        log(format!("Настройки не сохранены: {e}"));
        notify::toast(ui, Kind::Error, format!("Настройки не сохранены: {e}"));
        return;
    }
    if old.auto_start != new.auto_start {
        let enabled = new.auto_start;
        blocking(
            &ui.as_weak(),
            move || platform::set_autostart(enabled),
            move |ui, r| {
                if let Err(e) = r {
                    log(format!("Автозапуск: {e}"));
                    notify::toast(&ui, Kind::Error, format!("Автозапуск не изменён: {e}"));
                    ui.global::<Backend>().set_autostart(!enabled);
                }
            },
        );
    }
    // GameFilter меняет аргументы движка - перезапускаем zapret, если он работает
    if old.game_filter != new.game_filter && b.get_state() == 2 {
        log("GameFilter изменён - перезапуск zapret");
        notify::toast(ui, Kind::Info, "GameFilter изменён, zapret перезапускается");
        start_zapret(ui);
    }
}

// ---------- hosts (DNS Override) ----------

fn show_list(ui: &AppWindow, list: UserList) {
    let entries: Vec<SharedString> = list.read().into_iter().map(Into::into).collect();
    ui.global::<Backend>()
        .set_lists_entries(ModelRc::new(VecModel::from(entries)));
}

fn current_list(ui: &AppWindow) -> UserList {
    UserList::ALL[ui.global::<Backend>().get_lists_tab().clamp(0, 2) as usize]
}

fn people_model(list: Vec<info::Person>) -> ModelRc<Person> {
    let items: Vec<Person> = list
        .into_iter()
        .map(|p| Person {
            name: p.name.into(),
            note: p.note.into(),
        })
        .collect();
    ModelRc::new(VecModel::from(items))
}

/// Новости, спонсоры, сообщество: грузим при старте, новую новость показываем один раз.
fn connect_about(ui: &AppWindow) {
    let be = ui.global::<Backend>();
    let local = info::People::default();
    be.set_sponsors(people_model(local.sponsors));
    be.set_community(people_model(local.community));
    be.on_open_link(|url| open_link(&url));
    be.on_about_open({
        let weak = ui.as_weak();
        move |tab| {
            let Some(ui) = weak.upgrade() else { return };
            ui.global::<Backend>().set_about_tab(tab);
            ui.global::<Ui>().set_dialog(4);
        }
    });

    let weak = ui.as_weak();
    spawn(async move {
        let (news, people) = tokio::join!(info::news(), info::people());
        let _ = weak.upgrade_in_event_loop(move |ui| {
            let b = ui.global::<Backend>();
            b.set_sponsors(people_model(people.sponsors));
            b.set_community(people_model(people.community));
            let Some(news) = news else { return };
            b.set_news_title(news.title.clone().into());
            b.set_news_body(news.body.clone().into());
            let id = news.id();
            let fresh = with_state(|s| {
                s.news_id = Some(id.clone());
                s.cfg.news_seen.as_deref() != Some(id.as_str())
            });
            if fresh && ui.global::<Ui>().get_dialog() == 0 {
                b.set_about_tab(0);
                ui.global::<Ui>().set_dialog(4);
            }
        });
    });
}

fn check_app_update(ui: &AppWindow) {
    let weak = ui.as_weak();
    spawn(async move {
        let Some(r) = info::newer_release(env!("CARGO_PKG_VERSION")).await else {
            return;
        };
        let _ = weak.upgrade_in_event_loop(move |ui| {
            log(format!("Доступна новая версия Z.UST: v{}", r.version));
            notify::show(
                &ui,
                Kind::Info,
                format!(
                    "Вышла новая версия Z.UST: v{}. Скачать можно в Настройках",
                    r.version
                ),
            );
            let b = ui.global::<Backend>();
            b.set_update_version(r.version.into());
            b.on_update_open(move || open_link(&r.url));
        });
    });
}

fn connect_lists(ui: &AppWindow) {
    let be = ui.global::<Backend>();
    be.on_lists_open({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            b.set_lists_error("".into());
            b.set_lists_changed(false);
            show_list(&ui, current_list(&ui));
            ui.global::<Ui>().set_dialog(3);
        }
    });
    be.on_lists_select({
        let weak = ui.as_weak();
        move |i| {
            let Some(ui) = weak.upgrade() else { return };
            ui.global::<Backend>().set_lists_tab(i);
            ui.global::<Backend>().set_lists_error("".into());
            show_list(&ui, current_list(&ui));
        }
    });
    be.on_lists_add({
        let weak = ui.as_weak();
        move |text| {
            let Some(ui) = weak.upgrade() else {
                return false;
            };
            let b = ui.global::<Backend>();
            let list = current_list(&ui);
            let entry = match list.normalize(&text) {
                Ok(e) => e,
                Err(e) => {
                    b.set_lists_error(e.into());
                    return false;
                }
            };
            let mut entries = list.read();
            if entries.contains(&entry) {
                b.set_lists_error(format!("{entry} уже в списке").into());
                return false;
            }
            entries.push(entry);
            if let Err(e) = list.write(&entries) {
                b.set_lists_error(e.into());
                return false;
            }
            b.set_lists_error("".into());
            b.set_lists_changed(true);
            show_list(&ui, list);
            true
        }
    });
    be.on_lists_remove({
        let weak = ui.as_weak();
        move |i| {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            let list = current_list(&ui);
            let mut entries = list.read();
            if i < 0 || i as usize >= entries.len() {
                return;
            }
            entries.remove(i as usize);
            match list.write(&entries) {
                Ok(()) => b.set_lists_changed(true),
                Err(e) => b.set_lists_error(e.into()),
            }
            show_list(&ui, list);
        }
    });
    be.on_lists_restart({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            ui.global::<Backend>().set_lists_changed(false);
            ui.global::<Ui>().set_dialog(0);
            restart_zapret(&ui);
        }
    });
}

fn restart_zapret(ui: &AppWindow) {
    ZAPRET_BUSY.store(true, Ordering::SeqCst);
    set_zapret_state(ui, 1);
    let weak = ui.as_weak();
    spawn(async move {
        let r = Zapret::stop(&hub()).await;
        let _ = weak.upgrade_in_event_loop(move |ui| {
            ZAPRET_BUSY.store(false, Ordering::SeqCst);
            match r {
                Ok(()) => start_zapret(&ui),
                Err(e) => {
                    log(format!("Ошибка при остановке zapret: {e}"));
                    set_zapret_state(&ui, 2);
                }
            }
        });
    });
}

fn connect_hosts(ui: &AppWindow) {
    let be = ui.global::<Backend>();
    be.on_comss_toggle({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            if b.get_comss_busy() {
                return;
            }
            // переключатель уже сменил значение
            let enable = b.get_comss();
            b.set_comss_busy(true);
            blocking(
                &weak,
                move || (Comss::set(enable), Comss::is_enabled()),
                move |ui, (r, on)| {
                    let b = ui.global::<Backend>();
                    b.set_comss_busy(false);
                    b.set_comss(on);
                    match r {
                        Ok(()) if on == enable => {
                            let text = if on {
                                "Comss.one DNS включён"
                            } else {
                                "Прежние DNS возвращены"
                            };
                            log(text);
                            notify::toast(&ui, Kind::Ok, text);
                        }
                        Ok(()) => {
                            log("Comss.one DNS: настройки не применились");
                            notify::show(
                                &ui,
                                Kind::Error,
                                "DNS не изменился. Подробности на вкладке \"Логи\"",
                            );
                        }
                        Err(e) => {
                            log(format!("Comss.one DNS: {e}"));
                            notify::show(&ui, Kind::Error, format!("Не удалось изменить DNS: {e}"));
                        }
                    }
                },
            );
        }
    });
    be.on_hosts_open({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            ui.global::<Ui>().set_dialog(1);
            b.set_hosts_error("".into());
            b.set_hosts_loading(true);
            blocking(&weak, Comss::is_enabled, |ui, on| {
                ui.global::<Backend>().set_comss(on);
            });
            let weak = weak.clone();
            spawn(async move {
                let r = Hosts::fetch(&hub()).await;
                let installed = tokio::task::spawn_blocking(Hosts::get_installed_categories)
                    .await
                    .unwrap_or_default();
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    let b = ui.global::<Backend>();
                    b.set_hosts_loading(false);
                    let content = match r {
                        Ok(c) => c,
                        Err(e) => {
                            b.set_hosts_error(format!("Списки не загрузились: {e}").into());
                            return;
                        }
                    };
                    let map = Hosts::get_categories(&content);
                    let mut names: Vec<&String> = map.keys().collect();
                    // "Базовая" и "Discord & Telegram" - первыми, остальное по алфавиту
                    names.sort_by_key(|n| {
                        let rank = match n.as_str() {
                            "Базовая" => 0,
                            "Discord & Telegram" => 1,
                            _ => 2,
                        };
                        (rank, n.to_lowercase())
                    });
                    let items: Vec<HostsCategory> = names
                        .iter()
                        .map(|n| HostsCategory {
                            name: n.as_str().into(),
                            count: map[*n].len() as i32,
                            selected: installed.contains(n),
                        })
                        .collect();
                    b.set_hosts_categories(ModelRc::new(VecModel::from(items)));
                    b.set_hosts_date(Hosts::get_update_date(&content).into());
                    with_state(|s| s.hosts_map = map);
                });
            });
        }
    });
    be.on_hosts_toggle({
        let weak = ui.as_weak();
        move |i| {
            let Some(ui) = weak.upgrade() else { return };
            let model = ui.global::<Backend>().get_hosts_categories();
            if let Some(mut c) = model.row_data(i as usize) {
                c.selected = !c.selected;
                model.set_row_data(i as usize, c);
            }
        }
    });
    be.on_hosts_apply({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let b = ui.global::<Backend>();
            let selected: Vec<String> = b
                .get_hosts_categories()
                .iter()
                .filter(|c| c.selected)
                .map(|c| c.name.to_string())
                .collect();
            let body = with_state(|s| Hosts::build_selection(&s.hosts_map, &selected));
            b.set_hosts_saving(true);
            b.set_hosts_error("".into());
            // запись hosts требует прав: на Linux спросит пароль (pkexec)
            blocking(
                &weak,
                move || Hosts::write(&hub(), &body),
                move |ui, r| {
                    let b = ui.global::<Backend>();
                    b.set_hosts_saving(false);
                    match r {
                        Ok(()) => {
                            let msg = if selected.is_empty() {
                                "Записи Z.UST убраны из hosts".to_string()
                            } else {
                                format!("hosts обновлён: {}", selected.join(", "))
                            };
                            log(&msg);
                            notify::toast(&ui, Kind::Ok, msg);
                            b.set_hosts(!selected.is_empty());
                            ui.global::<Ui>().set_dialog(0);
                        }
                        Err(e) => b.set_hosts_error(e.into()),
                    }
                },
            );
        }
    });
    be.on_flush_dns({
        let weak = ui.as_weak();
        move || {
            blocking(&weak, platform::flush_dns, |ui, r| match r {
                Ok(()) => {
                    log("Кэш DNS очищен");
                    notify::toast(&ui, Kind::Ok, "Кэш DNS очищен");
                }
                Err(e) => {
                    log(format!("Не удалось очистить кэш DNS: {e}"));
                    notify::toast(&ui, Kind::Error, format!("Кэш DNS не очищен: {e}"));
                }
            });
        }
    });
}

// ---------- ссылки zust:// ----------

fn row(label: &str, value: impl Into<SharedString>, ok: bool) -> LinkRow {
    LinkRow {
        label: label.into(),
        value: value.into(),
        ok,
    }
}

/// Ссылка zust://действие?параметры. Всё, что меняет файлы или настройки,
/// показывается в окне и выполняется только после подтверждения.
pub fn handle_link(ui: &AppWindow, link: &str) {
    log(format!("Получена ссылка: {link}"));
    let Ok(url) = url::Url::parse(link) else {
        log("Некорректная ссылка zust://");
        return;
    };
    if url.scheme() != "zust" {
        return;
    }
    let action = url.host_str().unwrap_or_default().to_lowercase();
    let params: std::collections::HashMap<String, String> =
        url.query_pairs().into_owned().collect();
    let get = |k: &str| params.get(k).cloned().filter(|v| !v.is_empty());
    let b = ui.global::<Backend>();

    let (title, text, confirm, danger, rows, pending) = match action.as_str() {
        "flush_dns" => {
            b.invoke_flush_dns();
            return;
        }
        // пасхалки из Zust 2
        "milana_hametova" => return open_link("https://i.imgur.com/CGqXjZB.png"),
        "net" => return open_link("https://i.imgur.com/GuuLZta.png"),
        "restore_zapret" => (
            "Восстановление zapret",
            "Все стратегии и списки вернутся к исходным. Ваши собственные стратегии и списки будут удалены - сохраните их, если они нужны.",
            "Сбросить",
            true,
            vec![],
            LinkAction::Restore,
        ),
        "add" => {
            use zust_core::utils::AddType;
            let kind_s = get("type").unwrap_or_default().to_lowercase();
            let kind = match kind_s.as_str() {
                "strategy" => AddType::Strategy,
                "ipset" => AddType::Ipset,
                "hostlist" => AddType::Hostlist,
                _ => {
                    log(format!("zust://add: неизвестный тип \"{kind_s}\""));
                    return;
                }
            };
            let Some(src) = get("url") else {
                log("zust://add: нет адреса файла");
                return;
            };
            let name = if kind == AddType::Hostlist {
                Some("list-general.txt".to_string())
            } else {
                get("name")
            };
            let kind_label = match kind {
                AddType::Strategy => "стратегия",
                AddType::Ipset => "список IP (ipset)",
                AddType::Hostlist => "список доменов (list-general.txt)",
            };
            (
                "Добавить файл",
                "Файл скачается по ссылке и пройдёт проверку: стратегии с опасными параметрами не принимаются.",
                "Добавить",
                false,
                vec![
                    row("Тип", kind_label, true),
                    row(
                        "Имя",
                        name.clone().unwrap_or_else(|| "из ссылки".into()),
                        true,
                    ),
                    row("Источник", src.clone(), src.starts_with("https://")),
                ],
                LinkAction::Add {
                    kind,
                    url: src,
                    name,
                },
            )
        }
        "configure_zapret" => {
            let Some(strategy) = get("strategy") else {
                log("zust://configure_zapret: нет стратегии");
                return;
            };
            let ipset = get("ipset_config").map(|i| {
                let low = i.to_lowercase();
                if low == "any" || low == "none" || low.ends_with(".txt") {
                    i
                } else {
                    format!("{i}.txt")
                }
            });
            let game_filter = get("game_filter").map(|v| v == "true" || v == "1");
            let missing =
                zust_core::utils::missing_resources(Some(strategy.clone()), ipset.clone());
            let mut rows = vec![row(
                "Стратегия",
                strategy.clone(),
                !missing.iter().any(|m| m == "strategy"),
            )];
            if let Some(i) = &ipset {
                rows.push(row(
                    "Фильтр по IP",
                    i.clone(),
                    !missing.iter().any(|m| m == "ipset"),
                ));
            }
            if let Some(g) = game_filter {
                rows.push(row(
                    "GameFilter",
                    if g {
                        "включить"
                    } else {
                        "выключить"
                    },
                    true,
                ));
            }
            let ok = missing.is_empty();
            (
                "Настройка zapret",
                if ok {
                    "zapret запустится с этими параметрами."
                } else {
                    "Некоторых файлов нет - сначала добавьте их (или обновите стратегии)."
                },
                if ok { "Применить" } else { "" },
                false,
                rows,
                LinkAction::Configure {
                    strategy,
                    ipset,
                    game_filter,
                },
            )
        }
        other => {
            log(format!("Команда zust://{other} не распознана"));
            return;
        }
    };
    b.set_link_title(title.into());
    b.set_link_text(text.into());
    b.set_link_confirm_text(confirm.into());
    b.set_link_danger(danger);
    b.set_link_busy(false);
    b.set_link_rows(ModelRc::new(VecModel::from(rows)));
    with_state(|s| s.pending = Some(pending));
    ui.global::<Ui>().set_dialog(2);
}

fn connect_links(ui: &AppWindow) {
    let be = ui.global::<Backend>();
    be.on_dialog_close({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                let ui_g = ui.global::<Ui>();
                if ui_g.get_dialog() == 4 {
                    with_state(|s| {
                        if s.news_id.is_some() && s.cfg.news_seen != s.news_id {
                            s.cfg.news_seen = s.news_id.clone();
                            s.cfg.save();
                        }
                    });
                }
                ui_g.set_dialog(0);
            }
            with_state(|s| s.pending = None);
        }
    });
    be.on_link_confirm({
        let weak = ui.as_weak();
        move || {
            let Some(ui) = weak.upgrade() else { return };
            let Some(action) = with_state(|s| s.pending.clone()) else {
                return;
            };
            let b = ui.global::<Backend>();
            match action {
                LinkAction::Configure {
                    strategy,
                    ipset,
                    game_filter,
                } => {
                    if let Some(g) = game_filter {
                        b.set_game_filter(g);
                        apply_core_settings(&ui);
                    }
                    if let Some(i) = ipset {
                        update_cfg(|c| c.ipset = Some(i));
                        let custom = Zapret::custom_ipset_files();
                        set_ipsets(&ui, custom);
                    }
                    let wanted = strategy
                        .strip_suffix(".zapret")
                        .unwrap_or(&strategy)
                        .to_lowercase();
                    let idx = with_state(|s| {
                        s.files.iter().position(|f| {
                            f.strip_suffix(".zapret").unwrap_or(f).to_lowercase() == wanted
                        })
                    });
                    if let Some(i) = idx {
                        b.set_strategy(i as i32);
                        with_state(|s| {
                            s.cfg.strategy = s.files.get(i).cloned();
                            s.cfg.save();
                        });
                    }
                    log("Настройка из ссылки применена");
                    notify::toast(&ui, Kind::Ok, "Настройка из ссылки применена");
                    ui.global::<Ui>().set_dialog(0);
                    with_state(|s| s.pending = None);
                    start_zapret(&ui);
                }
                LinkAction::Add { kind, url, name } => {
                    b.set_link_busy(true);
                    let weak = weak.clone();
                    spawn(async move {
                        let r = zust_core::utils::handle_add_link(kind, &url, name).await;
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let b = ui.global::<Backend>();
                            b.set_link_busy(false);
                            match r {
                                Ok(msg) => {
                                    log(&msg);
                                    notify::toast(&ui, Kind::Ok, msg);
                                    ui.global::<Ui>().set_dialog(0);
                                    with_state(|s| s.pending = None);
                                    reload_strategies(&ui);
                                    set_ipsets(&ui, Zapret::custom_ipset_files());
                                    // новый список доменов работает только после перезапуска
                                    if kind == zust_core::utils::AddType::Hostlist
                                        && b.get_state() == 2
                                    {
                                        start_zapret(&ui);
                                    }
                                }
                                Err(e) => {
                                    b.set_link_text(format!("Не получилось: {e}").into());
                                    b.set_link_danger(true);
                                }
                            }
                        });
                    });
                }
                LinkAction::Restore => {
                    b.set_link_busy(true);
                    let weak = weak.clone();
                    spawn(async move {
                        let r = Zapret::restore_files(&hub()).await;
                        let _ = weak.upgrade_in_event_loop(move |ui| {
                            let b = ui.global::<Backend>();
                            b.set_link_busy(false);
                            match r {
                                Ok(msg) => {
                                    log(&msg);
                                    notify::toast(&ui, Kind::Ok, msg);
                                    update_cfg(|c| {
                                        c.strategy = None;
                                        c.ipset = None;
                                    });
                                    ui.global::<Ui>().set_dialog(0);
                                    with_state(|s| s.pending = None);
                                    reload_strategies(&ui);
                                    set_ipsets(&ui, Zapret::custom_ipset_files());
                                }
                                Err(e) => b.set_link_text(format!("Не получилось: {e}").into()),
                            }
                        });
                    });
                }
            }
        }
    });
}
