use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Настройки приложения. `#[serde(default)]` - чтобы добавление нового поля
/// не сбрасывало все настройки у пользователей со старым settings.json.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub auto_start: bool,
    pub notifications: bool,
    pub minimize_to_tray: bool,
    pub game_filter: bool,
    pub refresh_bridges: bool,
    pub auto_tor: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_start: false,
            notifications: true,
            minimize_to_tray: true,
            game_filter: false,
            refresh_bridges: false,
            auto_tor: false,
        }
    }
}

pub fn config_path() -> PathBuf {
    crate::paths::data_dir().join("settings.json")
}

/// До 2.1.0 настройки лежали рядом с exe. Переносим их в папку данных один раз.
pub fn migrate_legacy() {
    let new_path = config_path();
    if new_path.exists() {
        return;
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let old = dir.join("settings.json");
        if old.exists() {
            let _ = fs::copy(&old, &new_path);
        }
    }
}

pub fn save(settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(config_path(), json).map_err(|e| e.to_string())
}

pub fn load() -> Settings {
    fs::read_to_string(config_path())
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}
