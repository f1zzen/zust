//! Новости, спонсоры и сообщество. Данные лежат в гисте, поэтому обновляются без релиза
//! и одинаковы у v2, v3 и телефона. Если гист недоступен - встроенные списки.

use serde::{Deserialize, Serialize};
use std::time::Duration;

const GIST: &str =
    "https://gist.githubusercontent.com/f1zzen/92a5742eb95e6e1922b268e9dd586f45/raw/";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct News {
    pub title: String,
    pub body: String,
}

impl News {
    /// По нему запоминаем, что новость уже показали.
    pub fn id(&self) -> String {
        format!("{}\n{}", self.title, self.body)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct People {
    #[serde(default)]
    pub sponsors: Vec<Person>,
    #[serde(default)]
    pub community: Vec<Person>,
}

impl Default for People {
    fn default() -> Self {
        let p = |name: &str, note: &str| Person {
            name: name.into(),
            note: note.into(),
        };
        Self {
            sponsors: vec![p("neton4ik", "400₽"), p("Рустам", "1063₽")],
            community: vec![
                p("neton4ik", "Первый донат на бусти"),
                p("Анархист", "Первый обзор на Z.UST"),
                p("радостева", "Z.UST-тян"),
                p("ks", "Z.UST-тян"),
                p("Исчадие", "Z.UST-тян"),
                p("П.Ф", "Z.UST-тян"),
                p("T0ko", "Z.UST-тян"),
            ],
        }
    }
}

async fn get(file: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;
    let res = client.get(format!("{GIST}{file}")).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    res.text().await.ok()
}

/// Первая строка - заголовок (`# ...`), дальше текст. Пусто или `#` - новостей нет.
pub fn parse_news(text: &str) -> Option<News> {
    let text = text.trim();
    if text.is_empty() || text == "#" {
        return None;
    }
    let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
    Some(News {
        title: first.trim_start_matches('#').trim().to_string(),
        body: plain_markdown(rest.trim()),
    })
}

/// Markdown → обычный текст: без **, `, # и со ссылками в виде "текст (адрес)".
fn plain_markdown(s: &str) -> String {
    let mut out = String::new();
    for line in s.lines() {
        let mut l = line.trim_start_matches('#').trim_start().to_string();
        if let Some(rest) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
            l = format!("• {rest}");
        }
        l = l.replace("**", "").replace("__", "").replace('`', "");
        while let Some(start) = l.find('[') {
            let Some(mid) = l[start..].find("](").map(|i| start + i) else {
                break;
            };
            let Some(end) = l[mid..].find(')').map(|i| mid + i) else {
                break;
            };
            let (text, url) = (&l[start + 1..mid], &l[mid + 2..end]);
            l = format!("{}{text} ({url}){}", &l[..start], &l[end + 1..]);
        }
        out.push_str(&l);
        out.push('\n');
    }
    out.trim().to_string()
}

pub async fn news() -> Option<News> {
    parse_news(&get("warning").await?)
}

pub async fn people() -> People {
    match get("community.json").await {
        Some(t) => serde_json::from_str(&t).unwrap_or_default(),
        None => People::default(),
    }
}

/// Новый релиз Z.UST на GitHub: версия без "v" и ссылка на страницу загрузки.
#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    #[serde(rename = "tag_name")]
    pub version: String,
    #[serde(rename = "html_url")]
    pub url: String,
}

/// Последний опубликованный релиз (черновики GitHub не отдаёт), если он новее `current`.
pub async fn newer_release(current: &str) -> Option<Release> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        // без User-Agent GitHub API отвечает 403
        .user_agent("Z.UST")
        .build()
        .ok()?;
    let res = client
        .get("https://api.github.com/repos/f1zzen/zust/releases/latest")
        .send()
        .await
        .ok()?;
    let mut r: Release = res.error_for_status().ok()?.json().await.ok()?;
    r.version = r.version.trim_start_matches('v').to_string();
    is_newer(&r.version, current).then_some(r)
}

/// 3.0.10 > 3.0.9; "3.1.0-beta" сравнивается как 3.1.0.
fn is_newer(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split(['-', '+'])
            .next()
            .unwrap_or("")
            .split('.')
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parse(a) > parse(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(is_newer("3.0.1", "3.0.0"));
        assert!(is_newer("3.0.10", "3.0.9"));
        assert!(is_newer("3.1", "3.0.5"));
        assert!(!is_newer("3.0.0", "3.0.0"));
        assert!(!is_newer("2.0.1", "3.0.0"));
    }

    #[test]
    fn news_parsing() {
        assert_eq!(parse_news("  #  "), None);
        let n = parse_news("# Обновление\n**Важно**: см. [релиз](https://x.y)\n- раз").unwrap();
        assert_eq!(n.title, "Обновление");
        assert_eq!(n.body, "Важно: см. релиз (https://x.y)\n• раз");
    }
}
