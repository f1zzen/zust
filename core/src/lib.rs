//! Ядро Z.UST без интерфейса: zapret, TOR, hosts, прокси, точка доступа, настройки.

pub mod bypass;
pub mod info;
pub mod logger;
pub mod paths;
pub mod platform;
pub mod settings;
pub mod utils;

use std::sync::{Arc, OnceLock};

/// Аргумент, с которым Zust запускается при входе в систему: стартуем свёрнутыми в трей.
pub const AUTOSTART_ARG: &str = "--autostart";

type Sink = dyn Fn(&str, serde_json::Value) + Send + Sync;

/// События ядра для интерфейса: log-event, tor-progress, zapret_repair, zust-deeplink.
#[derive(Clone)]
pub struct Hub {
    sink: Arc<Sink>,
}

impl Hub {
    pub fn new(sink: impl Fn(&str, serde_json::Value) + Send + Sync + 'static) -> Self {
        Self {
            sink: Arc::new(sink),
        }
    }

    /// Канал, который никуда не отправляет (тесты, консольные утилиты).
    pub fn silent() -> Self {
        Self::new(|_, _| {})
    }

    /// Отправить событие. Сигнатура как у `tauri::Emitter::emit`, чтобы код ядра не менялся.
    pub fn emit<S: serde::Serialize>(&self, event: &str, payload: S) -> Result<(), String> {
        let value = serde_json::to_value(payload).map_err(|e| e.to_string())?;
        (self.sink)(event, value);
        Ok(())
    }
}

static HUB: OnceLock<Hub> = OnceLock::new();

/// Задать общий канал событий (один раз при старте приложения).
pub fn set_hub(hub: Hub) {
    let _ = HUB.set(hub);
}

/// Общий канал событий; до `set_hub` - "немой".
pub fn hub() -> Hub {
    HUB.get().cloned().unwrap_or_else(Hub::silent)
}
