use crate::Hub;
use crate::logger::info;
use crate::platform;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;

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

    pub async fn fetch(app: &Hub) -> Result<String, String> {
        info(app, "Создаю запрос на получение актуальных файлов | HOSTS");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        let (malw_res, flow_res) =
            tokio::join!(client.get(MALW_URL).send(), client.get(FLOWSEAL_URL).send());
        let malw = match malw_res {
            Ok(res) => res.text().await.ok(),
            Err(e) => {
                info(
                    app,
                    &format!("Не удалось получить хосты от MALW, подробнее: {}", e),
                );
                None
            }
        };
        let flow = match flow_res {
            Ok(res) => res.text().await.ok(),
            Err(e) => {
                info(
                    app,
                    &format!("Не удалось получить хосты от FLOWSEAL, подробнее: {}", e),
                );
                None
            }
        };

        let mut combined = String::new();
        if let Some(m) = malw {
            info(app, "Списки от MALW получены, и отправлены на обработку.");
            combined.push_str(&format!(
                "{}\n{}\n{}\n",
                Self::MALW_START,
                m.trim(),
                Self::MALW_END
            ));
        }

        if let Some(f) = flow {
            info(
                app,
                "Списки от FLOWSEAL получены, и отправлены на обработку.",
            );
            combined.push_str(&format!(
                "{}\n# Discord & Telegram\n{}\n{}\n",
                Self::FLOW_START,
                f.trim(),
                Self::FLOW_END
            ));
        }

        if combined.is_empty() {
            return Err("По неизвестной нам с тобой причине, список пуст. Убедитесь, что ничего лишнего не нажимали и попробуйте снова.".to_string());
        }

        Ok(combined)
    }

    pub fn clean(app: &Hub, content: &str, new_data: &str) -> String {
        let mut new_ips = HashSet::new();
        for line in new_data.lines() {
            if let Some(ip) = line.split_whitespace().next()
                && !ip.starts_with('#')
                && (ip.contains('.') || ip.contains(':'))
            {
                new_ips.insert(ip.to_string());
            }
        }

        info(
            app,
            &format!(
                "В процессе чистки лишних IP-адресов, были найдены как минмум {}.",
                new_ips.len()
            ),
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

    pub fn write(app: &Hub, new_data: &str) -> Result<(), String> {
        let path = platform::hosts_path();
        let current_content = fs::read_to_string(&path).unwrap_or_default();

        info(
            app,
            "Очистка hosts-файла от старых записей, дубликатов и подобного..",
        );
        let base_content = Self::clean(app, &current_content, new_data);

        let final_content = format!("{}\n\n{}", new_data.trim(), base_content);

        match platform::write_hosts(&final_content) {
            Ok(_) => {
                info(app, &format!("DNS переопределён в {}", path.display()));
                let _ = platform::flush_dns();
                Ok(())
            }
            Err(e) => {
                let err_msg = format!("Не удалось записать hosts: {e}");
                info(app, &err_msg);
                Err(err_msg)
            }
        }
    }

    pub fn build_selection(
        categories: &HashMap<String, Vec<String>>,
        selected: &[String],
    ) -> String {
        let (mut malw, mut flow) = (String::new(), String::new());
        for cat in selected {
            let Some(lines) = categories.get(cat) else {
                continue;
            };
            let block = format!("# {cat}\n{}\n", lines.join("\n"));
            if cat == "Discord & Telegram" {
                flow.push_str(&block);
            } else {
                malw.push_str(&block);
            }
        }
        let mut body = String::new();
        if !malw.is_empty() {
            body.push_str(&format!("{}\n{malw}{}\n", Self::MALW_START, Self::MALW_END));
        }
        if !flow.is_empty() {
            body.push_str(&format!("{}\n{flow}{}\n", Self::FLOW_START, Self::FLOW_END));
        }
        body.trim().to_string()
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

    pub fn get_installed_categories() -> Vec<String> {
        let path = platform::hosts_path();
        let content = fs::read_to_string(&path).unwrap_or_default();
        let mut installed = Vec::new();
        let mut inside_our_block = false;
        for line in content.lines() {
            let l = line.trim();
            if l == Self::MALW_START || l == Self::FLOW_START {
                inside_our_block = true;
                continue;
            }
            if l == Self::MALW_END || l == Self::FLOW_END {
                inside_our_block = false;
                continue;
            }
            if inside_our_block && l.starts_with("# ") {
                let cat_name = l.trim_start_matches("# ").trim().to_string();
                if !cat_name.is_empty() && !cat_name.contains("Последнее обновление")
                {
                    installed.push(cat_name);
                }
            }
        }
        installed
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
