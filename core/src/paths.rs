//! Общие пути приложения. Инициализируются один раз в `setup`, после чего доступны без Hub.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Вызвать один раз при старте: data - настройки, resources - zapret/tor/frp рядом с программой.
pub fn init(data: PathBuf, resources: PathBuf) {
    let _ = std::fs::create_dir_all(&data);
    let _ = DATA_DIR.set(data);
    let _ = RESOURCE_DIR.set(strip_verbatim(&resources));
}

/// Папка с данными: %APPDATA%\com.f1zzen.zust на Windows, ~/.config/com.f1zzen.zust на Linux.
pub fn data_dir() -> PathBuf {
    DATA_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().join("zust"))
}

/// Папка с ресурсами, которые поставляются вместе с приложением (zapret, tor, frp).
pub fn resource_dir() -> PathBuf {
    RESOURCE_DIR.get().cloned().unwrap_or_default()
}

/// Убирает префикс `\\?\`, который Windows добавляет к канонизированным путям
/// (winws и cmd его не понимают).
pub fn strip_verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    PathBuf::from(s.trim_start_matches(r"\\?\").to_string())
}

/// Оставляет от строки только имя файла: защищает от `../` в путях, пришедших извне.
pub fn sanitize_file_name(name: &str) -> Option<String> {
    // разделители обеих систем, чтобы `..\` отсекался и на Linux
    let base = name.trim().rsplit(['/', '\\']).next()?;
    let clean: String = base
        .chars()
        .filter(|c| {
            !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') && !c.is_control()
        })
        .collect();
    let clean = clean.trim().trim_matches('.').to_string();
    (!clean.is_empty()).then_some(clean)
}

#[cfg(test)]
mod tests {
    use super::sanitize_file_name;

    #[test]
    fn sanitize() {
        assert_eq!(
            sanitize_file_name("general (ALT).zapret").as_deref(),
            Some("general (ALT).zapret")
        );
        assert_eq!(
            sanitize_file_name("../../etc/passwd").as_deref(),
            Some("passwd")
        );
        assert_eq!(
            sanitize_file_name(r"..\..\Windows\evil.bat").as_deref(),
            Some("evil.bat")
        );
        assert_eq!(sanitize_file_name("..").as_deref(), None);
        assert_eq!(sanitize_file_name("").as_deref(), None);
    }
}
