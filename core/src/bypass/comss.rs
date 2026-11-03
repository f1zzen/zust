//! Comss.one DNS (dns.comss.one): открывает сервисы, которые не работают из России
//! (ChatGPT, Claude, Copilot и др.), и блокирует рекламу. Ставится системным DNS.

use crate::{paths, platform};

pub const SERVERS: [&str; 2] = ["212.109.221.20", "212.109.195.93"];
pub const DOH: &str = "https://dns.comss.one/dns-query";

pub struct Comss;

impl Comss {
    fn backup() -> std::path::PathBuf {
        paths::data_dir().join("dns-backup.txt")
    }

    pub fn is_enabled() -> bool {
        platform::dns_uses(SERVERS[0])
    }

    pub fn set(enable: bool) -> Result<(), String> {
        match (enable, Self::is_enabled()) {
            // повторное включение затёрло бы сохранённые прежние DNS
            (true, true) | (false, false) => Ok(()),
            (true, false) => platform::set_dns(&SERVERS, DOH, &Self::backup()),
            (false, true) => platform::restore_dns(&Self::backup(), SERVERS[0]),
        }
    }
}
