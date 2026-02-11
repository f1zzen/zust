use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};
use tauri::AppHandle;

use crate::{sh, utils::*};

const HOSTS_URL: &str =
    "https://raw.githubusercontent.com/ImMALWARE/dns.malw.link/refs/heads/master/hosts";

pub struct Hosts;

impl Hosts {
    // малв, я надеюсь ты это менять не будешь. 🥹
    const HOSTS_START_MARKER: &str = "### dns.malw.link: hosts file";
    const HOSTS_MARKER_END: &str = "### dns.malw.link: end hosts file";

    fn get_path() -> PathBuf {
        let sys_root = env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
        Path::new(&sys_root).join(r"System32\drivers\etc\hosts")
    }

    pub async fn fetch(app: &AppHandle) -> Result<String, String> {
        let response = reqwest::get(HOSTS_URL).await.map_err(|e| {
            let err_msg = format!("err internet: {}", e);
            info(app, &err_msg);
            err_msg
        })?;

        let text = response.text().await.map_err(|e| {
            let err_msg = format!("err read: {}", e);
            info(app, &err_msg);
            err_msg
        })?;

        Ok(text)
    }

    pub fn clean(content: &str) -> String {
        let pattern = format!(
            r"(?s){}.*?{}",
            regex::escape(Self::HOSTS_START_MARKER),
            regex::escape(Self::HOSTS_MARKER_END)
        );
        let re = Regex::new(&pattern).unwrap();
        re.replace_all(content, "").trim().to_string()
    }

    pub fn write(app: &AppHandle, new_data: &str) -> Result<(), String> {
        let path = Self::get_path();
        let current_content = fs::read_to_string(&path).unwrap_or_default();
        if current_content.contains(Self::HOSTS_START_MARKER) {
            info(app, "hosts block already exists, updating...");
        } else {
            info(app, "hosts block not found, creating new...");
        }
        let cleaned_base = Self::clean(&current_content);
        let final_content = format!(
            "{}\n{}\n{}\n\n{}",
            Self::HOSTS_START_MARKER,
            new_data.trim(),
            Self::HOSTS_MARKER_END,
            cleaned_base
        );
        fs::write(&path, final_content).map_err(|e| format!("err write: {}", e))?;

        let result = sh!("ipconfig", "/flushdns");
        info(app, &format!("hosts success {:?}", result));
        Ok(())
    }

    pub fn get_update_date(content: &str) -> String {
        content
            .lines()
            .find(|line| line.contains("Последнее обновление"))
            .map(|line| {
                line.trim_start_matches('#')
                    .replace("Последнее обновление:", "")
                    .trim()
                    .to_string()
            })
            .unwrap_or_else(|| "Неизвестно".to_string())
    }

    pub fn get_categories(content: &str) -> HashMap<String, Vec<String>> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        let mut inside_block = false;
        let mut current_category = "Базовая".to_string();

        for line in content.lines() {
            let line = line.trim();
            if line == Self::HOSTS_START_MARKER {
                inside_block = true;
                continue;
            }
            if line == Self::HOSTS_MARKER_END {
                break;
            }

            if inside_block && !line.is_empty() {
                if line.starts_with('#') {
                    let comment = line.trim_start_matches('#').trim();
                    if comment.contains("Последнее обновление") || comment.is_empty()
                    {
                        continue;
                    }

                    current_category = if comment.to_lowercase().starts_with("базов") {
                        "Базовая".to_string()
                    } else {
                        comment.to_string()
                    };
                    continue;
                }
                map.entry(current_category.clone())
                    .or_default()
                    .push(line.to_string());
            }
        }
        map
    }
}
