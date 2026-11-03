//! Пасхалка из Zust 2: 20 нажатий на "Главную" подряд (не дольше 5 с между ними)
//! превращают знак Z.UST в скример, нажатие на него проигрывает звук.

use crate::{AppWindow, Backend, Ui};
use slint::ComponentHandle;
use std::cell::Cell;
use std::time::{Duration, Instant, SystemTime};

const CLICKS: u32 = 20;
const PICTURES: i32 = 12;
const SOUND: &[u8] = include_bytes!("../ui/scare/litvin.wav");

thread_local! {
    static COUNT: Cell<(u32, Option<Instant>)> = const { Cell::new((0, None)) };
}

pub fn connect(ui: &AppWindow) {
    let be = ui.global::<Backend>();
    be.on_nav_click({
        let weak = ui.as_weak();
        move |page| {
            let Some(ui) = weak.upgrade() else { return };
            let (mut n, last) = COUNT.get();
            if page != 0 || last.is_some_and(|t| t.elapsed() > Duration::from_secs(5)) {
                n = 0;
            }
            if page == 0 {
                n += 1;
            }
            if n >= CLICKS {
                n = 0;
                let g = ui.global::<Ui>();
                let nanos = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.subsec_nanos())
                    .unwrap_or(0);
                // каждый раз другая картинка
                let mut pic = (nanos % PICTURES as u32) as i32 + 1;
                if pic == g.get_scare() {
                    pic = pic % PICTURES + 1;
                }
                g.set_scare(pic);
            }
            COUNT.set((n, Some(Instant::now())));
        }
    });
    be.on_scare_sound(play);
}

/// Через системный проигрыватель: отдельная библиотека для звука ради пасхалки не нужна.
fn play() {
    let path = std::env::temp_dir().join("zust-litvin.wav");
    if std::fs::metadata(&path).map(|m| m.len()).ok() != Some(SOUND.len() as u64)
        && std::fs::write(&path, SOUND).is_err()
    {
        return;
    }
    #[cfg(windows)]
    let cmd = {
        let p = path.to_string_lossy().replace('\'', "''");
        let mut c = zust_core::platform::command("powershell");
        c.args([
            "-NoProfile",
            "-Command",
            &format!("(New-Object Media.SoundPlayer '{p}').PlaySync()"),
        ]);
        Some(c)
    };
    #[cfg(target_os = "macos")]
    let cmd = {
        let mut c = std::process::Command::new("/usr/bin/afplay");
        c.arg(&path);
        Some(c)
    };
    #[cfg(target_os = "linux")]
    let cmd = ["paplay", "pw-play", "aplay"]
        .into_iter()
        .find(|p| zust_core::platform::find_in_path(p).is_some())
        .map(|p| {
            let mut c = std::process::Command::new(p);
            c.arg(&path);
            c
        });
    if let Some(mut c) = cmd {
        let _ = c.spawn();
    }
}
