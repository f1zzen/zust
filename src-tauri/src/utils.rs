use chrono::Local;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri::Url;
use winreg::RegKey;
use winreg::enums::HKEY_CLASSES_ROOT;

use crate::bypass::zapret::Zapret;

#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AddType {
    Strategy,
    OldStrategy,
    IpsetConfig,
    Hostlist,
}

#[derive(serde::Serialize)]
pub struct FileTree {
    pub name: String,
    pub is_dir: bool,
    pub children: Option<Vec<FileTree>>,
    pub path: String,
}

pub fn get_file_tree(app: AppHandle) -> Vec<FileTree> {
    let root_path = Zapret::zapret_path(&app, "");
    build_tree(&root_path)
}

fn build_tree(path: &std::path::Path) -> Vec<FileTree> {
    let mut tree = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let name_low = name.to_lowercase(); //
            let p = entry.path();
            if name_low.starts_with("-hide") || name == "utils" {
                continue;
            }

            let meta = entry.metadata().unwrap();
            if meta.is_dir() {
                tree.push(FileTree {
                    name,
                    is_dir: true,
                    children: Some(build_tree(&p)),
                    path: p.to_string_lossy().to_string(),
                });
            } else {
                if name_low.ends_with(".txt") || name_low.ends_with(".zapret") {
                    tree.push(FileTree {
                        name,
                        is_dir: false,
                        children: None,
                        path: p.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }
    tree.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.cmp(&b.name)));
    tree
}

pub async fn process_incoming_url(app: AppHandle, url_str: String) {
    info(&app, &format!("zust:// url detect ->>> {}", url_str));

    let parsed_url = match Url::parse(&url_str) {
        Ok(u) if u.scheme() == "zust" => u,
        _ => {
            info(&app, "invalid zust://");
            return;
        }
    };
    let params: HashMap<String, String> = parsed_url.query_pairs().into_owned().collect();
    let action = parsed_url.host_str().unwrap_or("unknown").to_string();
    let payload = serde_json::json!({
      "action": action,
      "params": params,
      "raw": url_str
    });
    if let Err(e) = app.emit("zust-deeplink", payload) {
        info(&app, &format!("FR собщение не получил {}", e));
    }
}

pub async fn flush_dns() -> Result<String, String> {
    {
        let output = Command::new("ipconfig")
            .arg("/flushdns")
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() {
            Ok("ДНС кэш вычищен".into())
        } else {
            Err("ДНС кэш не вычищен. Неизвестная ошибка..".into())
        }
    }
}

pub async fn handle_add_link(
    app: AppHandle,
    add_type: AddType,
    url: String,
    custom_name: Option<String>,
) -> Result<String, String> {
    if add_type == AddType::Strategy || add_type == AddType::OldStrategy {
        let url_low = url.to_lowercase();
        if !url_low.ends_with(".bat") && !url_low.ends_with(".zapret") {
            return Err("ERR: STRATEGY_NOT_RIGHT_FORMAT".to_string());
        }
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    let clean_name = custom_name
        .map(|n| {
            n.replace(".bat", "")
                .replace(".zapret", "")
                .replace(".txt", "")
                .trim()
                .to_string()
        })
        .unwrap_or_else(|| "imported_file".to_string());

    match add_type {
        AddType::Strategy | AddType::OldStrategy => {
            let strats_dir = Zapret::zapret_path(&app, "strategies");

            if url.to_lowercase().ends_with(".bat") {
                let temp_path = strats_dir.join(format!("{}.bat", clean_name));
                fs::write(&temp_path, bytes).map_err(|e| e.to_string())?;
                Zapret::convert_multiple_bats(&app, vec![temp_path.to_string_lossy().into_owned()])
                    .await?;
                let _ = fs::remove_file(temp_path);

                Ok(format!(
                    "Стратегия '{}' импортирована и сконвертирована",
                    clean_name
                ))
            } else {
                let final_path = strats_dir.join(format!("{}.zapret", clean_name));
                fs::write(final_path, bytes).map_err(|e| e.to_string())?;
                Ok(format!("Стратегия '{}' добавлена", clean_name))
            }
        }
        AddType::IpsetConfig => {
            let path =
                Zapret::zapret_path(&app, "ipset-configs").join(format!("{}.txt", clean_name));
            fs::write(path, bytes).map_err(|e| e.to_string())?;
            Ok(format!("IP-сет '{}' добавлен", clean_name))
        }
        AddType::Hostlist => {
            let path = Zapret::zapret_path(&app, "lists").join("list-general.txt");
            fs::write(path, bytes).map_err(|e| e.to_string())?;
            Ok("Список доменов 'list-general.txt' обновлен".to_string())
        }
    }
}

pub fn register_zust_protocol() -> Result<(), String> {
    let exe_path =
        std::env::current_exe().map_err(|e| format!("Не удалось получить путь exe: {}", e))?;
    let exe_str = exe_path.to_string_lossy();
    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let (key, _) = hkcr.create_subkey("zust").map_err(|e| e.to_string())?;
    key.set_value("", &"URL:Zust Protocol")
        .map_err(|e| e.to_string())?;
    key.set_value("URL Protocol", &"")
        .map_err(|e| e.to_string())?;
    let (shell_key, _) = key
        .create_subkey(r"shell\open\command")
        .map_err(|e| e.to_string())?;

    let command = format!("\"{}\" \"%1\"", exe_str);
    shell_key
        .set_value("", &command)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[macro_export]
macro_rules! sh {
    ($cmd:expr, $($arg:expr),*) => {{
        use std::os::windows::process::CommandExt;
        std::process::Command::new($cmd)
            .args([$($arg),*])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .status()
    }};
}
fn write_to_log(app: &AppHandle, text: &str) {
    let mut log_path = app.path().executable_dir().unwrap_or_else(|_| {
        let mut p = env::current_exe().unwrap_or_else(|_| env::current_dir().unwrap());
        p.pop();
        p
    });
    log_path.push("latest.log");
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "{}", text);
    }
}

pub async fn check_resources_exist(
    app: tauri::AppHandle,
    strategy: Option<String>,
    ipset: Option<String>,
) -> Result<Vec<String>, String> {
    let mut missing = Vec::new();
    if let Some(s) = strategy {
        if !s.is_empty() && s != "—" {
            let name = if s.ends_with(".zapret") {
                s
            } else {
                format!("{}.zapret", s)
            };
            if !Zapret::zapret_path(&app, "strategies").join(name).exists() {
                missing.push("strategy".to_string());
            }
        }
    }
    if let Some(i) = ipset {
        if !i.is_empty() && i != "—" && i != "none" && i != "any" {
            let name = if i.ends_with(".txt") {
                i
            } else {
                format!("{}.txt", i)
            };
            let path = Zapret::zapret_path(&app, "ipset-configs").join(&name);
            if !path.exists() {
                missing.push("ipset".to_string());
            }
        }
    }

    Ok(missing)
}

pub fn info(app: &AppHandle, text: &str) {
    let now = Local::now();
    let time_str = now.format("%d.%m %H:%M:%S").to_string();
    let log = format!("[{}] {}", time_str, text);
    write_to_log(app, &log);
    let _ = app.emit("log-event", &log);
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn list_files(path: PathBuf, ext: &str) -> Vec<String> {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(ext))
                .collect()
        })
        .unwrap_or_default()
}
