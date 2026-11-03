use crate::Hub;
use crate::bypass::zapret::{STRATEGY_EXT, Zapret};
use crate::logger::info;
use crate::paths::sanitize_file_name;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use url::Url;

#[derive(serde::Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum AddType {
    Strategy,
    Ipset,
    Hostlist,
}

#[derive(serde::Serialize)]
pub struct FileTree {
    pub name: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileTree>>,
    pub path: String,
}

pub fn file_tree() -> Vec<FileTree> {
    build_tree(&Zapret::path(""))
}

fn build_tree(path: &Path) -> Vec<FileTree> {
    let mut tree: Vec<FileTree> = fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            let low = name.to_lowercase();
            // служебные файлы и папки пользователю не показываем
            if low.contains("-hide") || name == "utils" {
                return None;
            }
            let p = entry.path();
            let is_dir = entry.file_type().ok()?.is_dir();
            if !is_dir && !(low.ends_with(".txt") || low.ends_with(STRATEGY_EXT)) {
                return None;
            }
            Some(FileTree {
                children: is_dir.then(|| build_tree(&p)),
                path: p.to_string_lossy().to_string(),
                name,
                is_dir,
            })
        })
        .collect();
    tree.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    tree
}

/// zust://action?param=... → событие "zust-deeplink" в интерфейс.
pub fn process_incoming_url(app: &Hub, url: &str) {
    info(app, &format!("Получена ссылка: {url}"));
    let parsed = match Url::parse(url) {
        Ok(u) if u.scheme() == "zust" => u,
        _ => {
            info(app, "Некорректная ссылка zust://");
            return;
        }
    };
    let params: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    let payload = serde_json::json!({
        "action": parsed.host_str().unwrap_or("unknown"),
        "params": params,
        "raw": url,
    });
    let _ = app.emit("zust-deeplink", payload);
}

/// Импорт стратегии / ipset / списка доменов по ссылке (из zust://add).
pub async fn handle_add_link(
    add_type: AddType,
    url: &str,
    custom_name: Option<String>,
) -> Result<String, String> {
    let parsed = Url::parse(url).map_err(|_| "Некорректная ссылка")?;
    if parsed.scheme() != "https" {
        return Err("Разрешены только https-ссылки".into());
    }
    let url_low = url.to_lowercase();
    let is_bat = url_low.ends_with(".bat");
    if add_type == AddType::Strategy && !is_bat && !url_low.ends_with(STRATEGY_EXT) {
        return Err("Стратегия должна быть файлом .bat или .zapret".into());
    }

    let bytes = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    // имя только из безопасных символов, без путей и расширения
    let name = custom_name
        .as_deref()
        .and_then(sanitize_file_name)
        .map(|n| {
            n.trim_end_matches(".bat")
                .trim_end_matches(STRATEGY_EXT)
                .trim_end_matches(".txt")
                .to_string()
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "imported_file".into());

    match add_type {
        AddType::Strategy => {
            let dir = Zapret::path("strategies");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            if is_bat {
                let tmp = std::env::temp_dir().join(format!("{name}.bat"));
                fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
                let res = Zapret::convert_bats(vec![tmp.to_string_lossy().into_owned()]);
                let _ = fs::remove_file(&tmp);
                match res? {
                    0 => Err("В .bat не найден запуск winws.exe".into()),
                    _ => Ok(format!(
                        "Стратегия \"{name}\" импортирована и сконвертирована"
                    )),
                }
            } else {
                let text = String::from_utf8(bytes.to_vec())
                    .map_err(|_| "Стратегия не является текстом".to_string())?;
                Zapret::validate_strategy(text.trim())?;
                fs::write(dir.join(format!("{name}{STRATEGY_EXT}")), &bytes)
                    .map_err(|e| e.to_string())?;
                Ok(format!("Стратегия \"{name}\" добавлена"))
            }
        }
        AddType::Ipset => {
            let dir = Zapret::path("ipset-configs");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            fs::write(dir.join(format!("{name}.txt")), &bytes).map_err(|e| e.to_string())?;
            Ok(format!("IP-сет \"{name}\" добавлен"))
        }
        AddType::Hostlist => {
            fs::write(Zapret::path("lists").join("list-general.txt"), &bytes)
                .map_err(|e| e.to_string())?;
            Ok("Список доменов list-general.txt обновлён".into())
        }
    }
}

/// Какие из указанных файлов отсутствуют: ["strategy"], ["ipset"].
pub fn missing_resources(strategy: Option<String>, ipset: Option<String>) -> Vec<String> {
    let mut missing = Vec::new();
    if let Some(s) = strategy.filter(|s| !s.is_empty() && s != "-") {
        let name = sanitize_file_name(&s).unwrap_or_default();
        let name = if name.ends_with(STRATEGY_EXT) {
            name
        } else {
            format!("{name}{STRATEGY_EXT}")
        };
        if !Zapret::path("strategies").join(name).exists() {
            missing.push("strategy".into());
        }
    }
    if let Some(i) = ipset
        .filter(|i| !i.is_empty() && !["-", "none", "any"].contains(&i.to_lowercase().as_str()))
    {
        let name = sanitize_file_name(&i).unwrap_or_default();
        let name = if name.ends_with(".txt") {
            name
        } else {
            format!("{name}.txt")
        };
        if !Zapret::path("ipset-configs").join(&name).exists()
            && !Zapret::path("lists").join(&name).exists()
        {
            missing.push("ipset".into());
        }
    }
    missing
}
