//! Уведомления: всплывашки внизу окна, а когда окно спрятано в трей или свёрнуто - системные.
//! Выключенная настройка "Уведомления" глушит всё, кроме ошибок.

use crate::{AppWindow, Backend, Toast};
use slint::{ComponentHandle, Model, VecModel};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Info = 0,
    Ok = 1,
    Error = 2,
}

const MAX_TOASTS: usize = 3;
const FADE: Duration = Duration::from_millis(320);

thread_local! {
    static TOASTS: RefCell<Option<Rc<VecModel<Toast>>>> = const { RefCell::new(None) };
    static NEXT_ID: Cell<i32> = const { Cell::new(0) };
}

pub fn init(ui: &AppWindow) {
    let model = Rc::new(VecModel::<Toast>::default());
    let be = ui.global::<Backend>();
    be.set_toasts(model.clone().into());
    be.on_toast_close(dismiss);
    TOASTS.with(|t| *t.borrow_mut() = Some(model));
}

fn enabled(ui: &AppWindow, kind: Kind) -> bool {
    kind == Kind::Error || ui.global::<Backend>().get_notifications()
}

fn window_hidden(ui: &AppWindow) -> bool {
    let w = ui.window();
    !w.is_visible() || w.is_minimized()
}

/// Окно открыто - всплывашка, иначе системное уведомление.
pub fn show(ui: &AppWindow, kind: Kind, text: impl Into<String>) {
    if !enabled(ui, kind) {
        return;
    }
    let text = text.into();
    if window_hidden(ui) {
        system(kind, text);
    } else {
        push(kind, text);
    }
}

/// Только когда окна не видно: в открытом окне это и так видно по кнопкам.
pub fn background(ui: &AppWindow, kind: Kind, text: impl Into<String>) {
    if enabled(ui, kind) && window_hidden(ui) {
        system(kind, text.into());
    }
}

/// Только всплывашка в окне - для того, что неважно, пока окно закрыто.
pub fn toast(ui: &AppWindow, kind: Kind, text: impl Into<String>) {
    if enabled(ui, kind) {
        push(kind, text.into());
    }
}

fn push(kind: Kind, text: String) {
    let Some(model) = TOASTS.with(|t| t.borrow().clone()) else {
        return;
    };
    let id = NEXT_ID.with(|n| {
        n.set(n.get() + 1);
        n.get()
    });
    // повтор того же текста не плодит копии
    if let Some(old) = model.iter().find(|t| t.text == text.as_str() && t.shown) {
        dismiss(old.id);
    }
    model.push(Toast {
        id,
        text: text.into(),
        kind: kind as i32,
        shown: false,
    });
    // показываем на следующем кадре, чтобы сработала анимация появления
    let m = model.clone();
    slint::Timer::single_shot(Duration::from_millis(16), move || {
        set_shown(&m, id, true);
    });
    let live: Vec<i32> = model
        .iter()
        .filter(|t| t.shown || t.id == id)
        .map(|t| t.id)
        .collect();
    for old in &live[..live.len().saturating_sub(MAX_TOASTS)] {
        dismiss(*old);
    }
    let secs = if kind == Kind::Error { 8 } else { 4 };
    slint::Timer::single_shot(Duration::from_secs(secs), move || dismiss(id));
}

/// Плавно убирает всплывашку: сначала прозрачность, потом из списка.
fn dismiss(id: i32) {
    let Some(model) = TOASTS.with(|t| t.borrow().clone()) else {
        return;
    };
    if !set_shown(&model, id, false) {
        return;
    }
    slint::Timer::single_shot(FADE, move || {
        if let Some(i) = model.iter().position(|t| t.id == id) {
            model.remove(i);
        }
    });
}

/// false - такой всплывашки нет или она уже в нужном состоянии
fn set_shown(model: &VecModel<Toast>, id: i32, shown: bool) -> bool {
    let Some(i) = model.iter().position(|t| t.id == id) else {
        return false;
    };
    let mut t = model.row_data(i).unwrap();
    if t.shown == shown {
        return false;
    }
    t.shown = shown;
    model.set_row_data(i, t);
    true
}

fn system(kind: Kind, text: String) {
    // показ может подождать D-Bus / WinRT - не держим интерфейс
    std::thread::spawn(move || {
        let mut n = notify_rust::Notification::new();
        n.appname("Z.UST")
            .summary(if kind == Kind::Error {
                "Z.UST - ошибка"
            } else {
                "Z.UST"
            })
            .body(&text);
        #[cfg(target_os = "linux")]
        if let Some(icon) = icon_path() {
            n.icon(&icon);
        }
        let _ = n.show();
    });
}

#[cfg(target_os = "linux")]
fn icon_path() -> Option<String> {
    let path = zust_core::paths::data_dir().join("notify-icon.png");
    if !path.exists() {
        std::fs::write(&path, crate::backend::ICON_PNG).ok()?;
    }
    Some(path.to_string_lossy().into_owned())
}
