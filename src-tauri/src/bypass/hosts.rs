use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{env, fs};
use tauri::AppHandle;

use crate::{sh, utils::*};

const MALW_URL: &str =
    "https://raw.githubusercontent.com/ImMALWARE/dns.malw.link/refs/heads/master/hosts";
const FLOWSEAL_URL: &str = "https://raw.githubusercontent.com/Flowseal/zapret-discord-youtube/refs/heads/main/.service/hosts";

pub struct Hosts;

impl Hosts {
    // малв, я надеюсь ты это менять не будешь. 🥹
    const MALW_START: &str = "### dns.malw.link: hosts file";
    const MALW_END: &str = "### dns.malw.link: end hosts file";

    const FLOW_START: &str = "### flowseal: hosts file";
    const FLOW_END: &str = "### flowseal: end hosts file";

    fn get_path() -> PathBuf {
        let sys_root = env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
        Path::new(&sys_root).join(r"System32\drivers\etc\hosts")
    }

    pub async fn fetch(app: &AppHandle) -> Result<String, String> {
        info(app, "Создаю запрос на получение актуальных файлов | HOSTS");
        let malw = match reqwest::get(MALW_URL).await {
            Ok(res) => res.text().await.ok(),
            Err(e) => {
                info(app, &format!("ERR_MALWARE_HOSTS_FETCH: {}", e));
                None
            }
        };
        let flow = match reqwest::get(FLOWSEAL_URL).await {
            Ok(res) => res.text().await.ok(),
            Err(e) => {
                info(app, &format!("ERR_FLOWSEAL_HOSTS_FETCH: {}", e));
                None
            }
        };

        let mut combined = String::new();
        if let Some(m) = malw {
            info(app, "MALWARE_HOSTS_QUEQE ADDED");
            combined.push_str(&format!(
                "{}\n{}\n{}\n",
                Self::MALW_START,
                m.trim(),
                Self::MALW_END
            ));
        }

        if let Some(f) = flow {
            info(app, "FLOWSEAL_HOSTS_QUEQE ADDED");
            combined.push_str(&format!(
                "{}\n# Discord & Telegram\n{}\n{}\n",
                Self::FLOW_START,
                f.trim(),
                Self::FLOW_END
            ));
        }

        if combined.is_empty() {
            return Err("NO_ONE_ADDED ERR".to_string());
        }

        Ok(combined)
    }

    pub fn clean(app: &AppHandle, content: &str, new_data: &str) -> String {
        let mut new_ips = HashSet::new();
        for line in new_data.lines() {
            if let Some(ip) = line.split_whitespace().next() {
                if !ip.starts_with('#') && (ip.contains('.') || ip.contains(':')) {
                    new_ips.insert(ip.to_string());
                }
            }
        }

        info(
            app,
            &format!("CONFLICT CHECKED -> {} IP-адресов", new_ips.len()),
        );

        let re_malw = Regex::new(&format!(
            r"(?s){}.*?{}",
            regex::escape(Self::MALW_START),
            regex::escape(Self::MALW_END)
        ))
        .unwrap();
        let re_flow = Regex::new(&format!(
            r"(?s){}.*?{}",
            regex::escape(Self::FLOW_START),
            regex::escape(Self::FLOW_END)
        ))
        .unwrap();

        let cleaned_malw = re_malw.replace_all(content, "");
        let no_blocks = re_flow.replace_all(&cleaned_malw, "");

        let final_lines: Vec<&str> = no_blocks
            .lines()
            .filter(|line| {
                if let Some(ip) = line.split_whitespace().next() {
                    return !new_ips.contains(ip);
                }
                true
            })
            .collect();

        final_lines.join("\n").trim().to_string()
    }

    pub fn write(app: &AppHandle, new_data: &str) -> Result<(), String> {
        let path = Self::get_path();
        let current_content = fs::read_to_string(&path).unwrap_or_default();

        info(app, "Очистка hosts от старых записей и дубликатов IP...");
        let base_content = Self::clean(app, &current_content, new_data);

        let final_content = format!("{}\n\n{}", new_data.trim(), base_content);

        match fs::write(&path, final_content) {
            Ok(_) => {
                info(app, "DNS был переопределён в файле /etc/hosts!");
                let _ = sh!("ipconfig", "/flushdns");
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Неизвестная ошибка записи: {}", e);
                info(app, &err_msg);
                Err(err_msg)
            }
        }
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
        let mut current_category = "Базовая".to_string();
        let mut inside_our_block = false;

        for line in content.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }

            if l == Self::MALW_START || l == Self::FLOW_START {
                inside_our_block = true;
                continue;
            }
            if l == Self::MALW_END || l == Self::FLOW_END {
                inside_our_block = false;
                continue;
            }

            if inside_our_block {
                if l.starts_with('#') {
                    let comment = l.trim_start_matches('#').trim();
                    if comment.contains("Последнее обновление") || comment.is_empty()
                    {
                        continue;
                    }
                    current_category = if comment.to_lowercase().contains("базов") {
                        "Базовая".to_string()
                    } else {
                        comment.to_string()
                    };
                } else {
                    map.entry(current_category.clone())
                        .or_default()
                        .push(l.to_string());
                }
            }
        }
        map
    }
}
