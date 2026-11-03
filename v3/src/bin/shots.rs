//! Снимки всех тем в PNG: cargo run --release --features shots --bin shots -- <папка>

use slint::Rgb8Pixel;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

slint::include_modules!();

thread_local! {
    static NOW: Cell<Duration> = const { Cell::new(Duration::ZERO) };
}

struct ShotPlatform {
    window: Rc<MinimalSoftwareWindow>,
}

impl Platform for ShotPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }
    fn duration_since_start(&self) -> Duration {
        NOW.with(Cell::get)
    }
}

fn advance(ms: u64) {
    // шагами, чтобы пружинные анимации честно дошли до конца
    for _ in 0..(ms / 16) {
        NOW.with(|n| n.set(n.get() + Duration::from_millis(16)));
        slint::platform::update_timers_and_animations();
    }
}

fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "shots".into()));
    std::fs::create_dir_all(&out).unwrap();
    let (w, h) = (960u32, 660u32);

    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    slint::platform::set_platform(Box::new(ShotPlatform {
        window: window.clone(),
    }))
    .unwrap();
    window.set_size(slint::PhysicalSize::new(w, h));

    let ui = AppWindow::new().unwrap();
    let be = ui.global::<Backend>();
    let names: Vec<SharedString> = [
        "general",
        "general (ALT)",
        "general (ALT2)",
        "general (ALT3)",
        "general (FAKE TLS AUTO)",
        "Zust Special 1",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    be.set_strategies(ModelRc::new(VecModel::from(names)));
    be.set_strategy(1);
    be.set_renderer("программная отрисовка".into());
    be.set_memory("29 МБ".into());
    be.set_version("v3.0.0".into());
    be.set_logs(ModelRc::new(VecModel::from(vec![
        SharedString::from("[01.11 02:10:01] Z.UST v3.0.0 - linux x86_64"),
        "[01.11 02:10:03] Запуск zapret: general (ALT)".into(),
        "[01.11 02:10:04] zapret запущен".into(),
    ])));
    be.set_proxies(ModelRc::new(VecModel::from(vec![
        ProxyItem {
            country: "DE".into(),
            address: "de1.example-proxy.net:443".into(),
            ping: 84,
        },
        ProxyItem {
            country: "NL".into(),
            address: "nl.example-proxy.net:8443".into(),
            ping: 233,
        },
        ProxyItem {
            country: "FI".into(),
            address: "fi2.example-proxy.net:443".into(),
            ping: 512,
        },
    ])));
    be.set_tor_state(1);
    be.set_tor_progress(45);
    be.set_tor_log(
        "Bootstrapped 45% (requesting_descriptors): Asking for relay descriptors".into(),
    );
    be.set_uptime("· 12 мин".into());
    be.set_ipsets(ModelRc::new(VecModel::from(vec![
        SharedString::from("Только домены"),
        "Все адреса".into(),
        "ipset-all".into(),
        "мой список".into(),
    ])));
    be.set_updates(ModelRc::new(VecModel::from(vec![
        SharedString::from("general (ALT)"),
        "general (ALT9)".into(),
    ])));
    be.set_launcher_supported(true);
    be.set_launcher_offer(false);
    be.set_hosts_categories(ModelRc::new(VecModel::from(vec![
        HostsCategory {
            name: "Базовая".into(),
            count: 12,
            selected: true,
        },
        HostsCategory {
            name: "Discord & Telegram".into(),
            count: 40,
            selected: false,
        },
        HostsCategory {
            name: "Gemini".into(),
            count: 9,
            selected: true,
        },
        HostsCategory {
            name: "ChatGPT".into(),
            count: 6,
            selected: false,
        },
        HostsCategory {
            name: "Spotify".into(),
            count: 14,
            selected: false,
        },
    ])));
    be.set_hosts_date("30.10.2026".into());
    be.set_link_title("Настройка zapret".into());
    be.set_link_text("Некоторых файлов нет - сначала добавьте их (или обновите стратегии).".into());
    be.set_link_confirm_text("Применить".into());
    be.set_link_rows(ModelRc::new(VecModel::from(vec![
        LinkRow {
            label: "Стратегия".into(),
            value: "general (ALT5)".into(),
            ok: true,
        },
        LinkRow {
            label: "Фильтр по IP".into(),
            value: "discord.txt".into(),
            ok: false,
        },
        LinkRow {
            label: "GameFilter".into(),
            value: "включить".into(),
            ok: true,
        },
    ])));
    be.set_lists_entries(ModelRc::new(VecModel::from(vec![
        slint::SharedString::from("rutracker.org"),
        "kinozal.tv".into(),
        "nnmclub.to".into(),
    ])));
    be.set_lists_changed(true);
    be.set_news_title("Z.UST 3.0 уже скоро".into());
    be.set_news_body("Новый интерфейс, пять тем и свои списки сайтов.\n\n• Классика по умолчанию\n• Анимированный знак\n\nСпасибо, что вы с нами!".into());
    let person = |n: &str, t: &str| Person {
        name: n.into(),
        note: t.into(),
    };
    be.set_sponsors(ModelRc::new(VecModel::from(vec![
        person("neton4ik", "400₽"),
        person("Рустам", "1063₽"),
    ])));
    be.set_community(ModelRc::new(VecModel::from(vec![
        person("neton4ik", "Первый донат на бусти"),
        person("Анархист", "Первый обзор на Z.UST"),
        person("радостева", "Z.UST-тян"),
        person("ks", "Z.UST-тян"),
        person("Исчадие", "Z.UST-тян"),
    ])));
    ui.show().unwrap();

    let styles = ["m3e", "warm", "mono", "classic", "lite"];
    // по умолчанию везде тёмная тема и фиолетовый; плюс тёплая светлая с терракотой для сравнения
    let variants: [(usize, &str, bool, u32); 10] = [
        (0, styles[0], true, 0xff8e6cef),
        (0, "m3e-light", false, 0xff8e6cef),
        (1, styles[1], true, 0xff8e6cef),
        (1, "warm-light", false, 0xffc96442),
        (2, styles[2], true, 0xff8e6cef),
        (3, styles[3], true, 0xff8e6cef),
        (4, styles[4], true, 0xff8e6cef),
        (0, "m3e-right", true, 0xff8e6cef),
        (1, "warm-right", true, 0xff8e6cef),
        (3, "classic-right", true, 0xff8e6cef),
    ];
    // (страница, состояние zapret, суффикс)
    let shots = [
        (0, 0, "home-off"),
        (0, 2, "home-on"),
        (1, 2, "configs"),
        (2, 2, "proxy"),
        (3, 0, "settings"),
        (10, 0, "hosts"),
        (20, 0, "link"),
        (31, 2, "lists"),
        (40, 0, "news"),
        (42, 0, "community"),
    ];

    let mut buf = vec![Rgb8Pixel::default(); (w * h) as usize];
    for (si, style, dark, accent) in variants {
        let t = ui.global::<Theme>();
        t.set_style(si as i32);
        t.set_accent(slint::Color::from_argb_encoded(accent));
        t.set_dark(dark);
        t.set_nav_side(style.ends_with("-right") as i32);
        for (page, state, suffix) in shots {
            // десятки - открытое окно поверх страницы
            ui.global::<Ui>().set_dialog(page / 10);
            ui.global::<Ui>().set_page(page % 10);
            // окно 4: единицы - вкладка, а не страница
            if page / 10 == 4 {
                be.set_about_tab(page % 10);
                ui.global::<Ui>().set_page(3);
            }
            be.set_state(state);
            advance(1500);
            window.request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut buf, w as usize);
            });
            let bytes: Vec<u8> = buf.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            let path = out.join(format!("{si}-{style}-{suffix}.png"));
            image::save_buffer(&path, &bytes, w, h, image::ExtendedColorType::Rgb8).unwrap();
            println!("{}", path.display());
        }
        let toast = |id, text: &str, kind| Toast {
            id,
            text: text.into(),
            kind,
            shown: true,
        };
        be.set_toasts(ModelRc::new(VecModel::from(vec![
            toast(
                1,
                "Есть обновления стратегий: 2. Их можно скачать на вкладке \"Стратегии\"",
                0,
            ),
            toast(2, "TOR подключён", 1),
            toast(
                3,
                "zapret не запустился. Подробности на вкладке \"Логи\"",
                2,
            ),
        ])));
        ui.global::<Ui>().set_dialog(0);
        ui.global::<Ui>().set_page(0);
        be.set_state(2);
        advance(1500);
        window.request_redraw();
        window.draw_if_needed(|r| {
            r.render(&mut buf, w as usize);
        });
        let bytes: Vec<u8> = buf.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
        let path = out.join(format!("{si}-{style}-toasts.png"));
        image::save_buffer(&path, &bytes, w, h, image::ExtendedColorType::Rgb8).unwrap();
        be.set_toasts(ModelRc::new(VecModel::<Toast>::default()));
        if style == "classic" {
            ui.global::<Ui>().set_page(0);
            ui.global::<Ui>().set_picker(true);
            advance(800);
            window.request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut buf, w as usize);
            });
            let bytes: Vec<u8> = buf.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            let path = out.join(format!("{si}-{style}-picker.png"));
            image::save_buffer(&path, &bytes, w, h, image::ExtendedColorType::Rgb8).unwrap();
            ui.global::<Ui>().set_picker(false);
            advance(800);
        }
    }
}
