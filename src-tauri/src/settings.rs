use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub auto_start: bool,
    pub notifications: bool,
    pub minimize_to_tray: bool,
    pub animation_enabled: bool,
    pub devtools: bool,
    pub game_filter: bool
}
pub fn get_config_path() -> PathBuf {
    let mut path = std::env::current_exe().unwrap_or_default();
    path.pop();
    path.push("settings.json");
    path
}

pub fn save_settings(settings: Settings) -> Result<(), String> {
    let path = get_config_path();
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_settings() -> Settings {
    let path = get_config_path();
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(settings) = serde_json::from_str(&data) {
            return settings;
        }
    }
    Settings {
        auto_start: false,
        notifications: true,
        minimize_to_tray: true,
        animation_enabled: true,
        devtools: false,
        game_filter: false
    }
}
