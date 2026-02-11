use std::{
    error::Error,
    time::{Duration, Instant},
};

use tauri::Url;
use tokio::net::{TcpStream, lookup_host};
use tokio::time::timeout;

use crate::utils::info;

pub struct Proxies;

const PROXIES_URL: &str =
    "https://gist.githubusercontent.com/f1zzen/565db71a26205f007f28f98c6c2c6b61/raw/proxies";

#[derive(serde::Serialize)]
pub struct Proxy {
    pub ping: u64,
    pub country_code: String,
}

impl Proxies {
    pub async fn get_proxy_list() -> Result<Vec<String>, Box<dyn Error>> {
        let response = reqwest::get(PROXIES_URL).await?.text().await?;
        let proxies: Vec<String> = response
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(proxies)
    }

    pub async fn check_proxy_ping(address: &str, app: tauri::AppHandle) -> Option<Proxy> {
        let clean_address = address.trim();
        let target = if clean_address.contains("server=") && clean_address.contains("port=") {
            let parsed = match Url::parse(clean_address) {
                Ok(p) => p,
                Err(_) => {
                    info(&app, &format!("url parse err: {}", clean_address));
                    return None;
                }
            };
            let query: std::collections::HashMap<_, _> =
                parsed.query_pairs().into_owned().collect();

            let server = match query.get("server") {
                Some(s) => s,
                None => return None,
            };

            let port = query
                .get("port")
                .cloned()
                .unwrap_or_else(|| "443".to_string());

            format!("{}:{}", server, port)
        } else {
            clean_address.to_string()
        };
        let addrs = lookup_host(&target).await.ok()?;
        let addr = addrs.into_iter().next()?;
        let ip_str = addr.ip().to_string();

        let start = Instant::now();
        match timeout(Duration::from_secs(3), TcpStream::connect(&addr)).await {
            Ok(Ok(_)) => {
                let ms = start.elapsed().as_millis() as u64;
                let url = format!("http://ip-api.com/json/{}?fields=countryCode", ip_str);

                let country_code = match timeout(Duration::from_secs(2), reqwest::get(url)).await {
                    Ok(Ok(response)) => {
                        let json: serde_json::Value = response.json().await.unwrap_or_default();
                        json["countryCode"].as_str().unwrap_or("??").to_string()
                    }
                    _ => "??".to_string(),
                };

                Some(Proxy {
                    ping: ms,
                    country_code,
                })
            }
            _ => None,
        }
    }
}
