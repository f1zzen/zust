use super::{USER_LISTS, Zapret};
use std::fs;
use std::net::IpAddr;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UserList {
    Domains,
    ExcludeDomains,
    ExcludeIps,
}

impl UserList {
    pub const ALL: [UserList; 3] = [Self::Domains, Self::ExcludeDomains, Self::ExcludeIps];

    pub fn file(self) -> &'static str {
        USER_LISTS[self as usize].0
    }

    fn placeholder(self) -> &'static str {
        USER_LISTS[self as usize].1.trim()
    }

    fn is_ip(self) -> bool {
        self == Self::ExcludeIps
    }

    pub fn read(self) -> Vec<String> {
        Zapret::ensure_user_lists();
        let text = fs::read_to_string(Zapret::path("lists").join(self.file())).unwrap_or_default();
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#') && *l != self.placeholder())
            .map(str::to_string)
            .collect()
    }

    /// Комментарии из файла сохраняются. Пустой список нельзя записать пустым файлом:
    /// для zapret это значит "подходит всё", поэтому пишем заглушку.
    pub fn write(self, entries: &[String]) -> Result<(), String> {
        let path = Zapret::path("lists").join(self.file());
        let old = fs::read_to_string(&path).unwrap_or_default();
        let mut out: Vec<&str> = old
            .lines()
            .filter(|l| l.trim_start().starts_with('#'))
            .collect();
        if entries.is_empty() {
            out.push(self.placeholder());
        } else {
            out.extend(entries.iter().map(String::as_str));
        }
        fs::write(&path, out.join("\n") + "\n")
            .map_err(|e| format!("Не удалось сохранить {}: {e}", self.file()))
    }

    /// Приводит ввод к виду, который понимает zapret: из ссылки берёт домен,
    /// для IP-списка принимает адрес или подсеть.
    pub fn normalize(self, input: &str) -> Result<String, String> {
        let s = input.trim();
        if s.is_empty() {
            return Err("Пустая строка".into());
        }
        if self.is_ip() {
            return normalize_ip(s).ok_or_else(|| {
                format!("\"{s}\" - не IP-адрес и не подсеть (пример: 1.2.3.4 или 1.2.3.0/24)")
            });
        }
        normalize_domain(s)
            .ok_or_else(|| format!("\"{s}\" - не похоже на домен (пример: example.com)"))
    }
}

fn normalize_domain(s: &str) -> Option<String> {
    let s = s.to_lowercase();
    let s = s.split("://").last()?;
    let host = s.split(['/', '?', '#']).next()?;
    let host = host.rsplit('@').next()?;
    let host = host.split(':').next()?.trim_end_matches('.');
    // zapret сам захватывает поддомены, так что www. только сузил бы правило
    let host = host.strip_prefix("*.").unwrap_or(host);
    let host = host.strip_prefix("www.").unwrap_or(host);
    let labels: Vec<&str> = host.split('.').collect();
    let ok = labels.len() >= 2
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && !labels.last()?.chars().all(|c| c.is_ascii_digit());
    ok.then(|| host.to_string())
}

fn normalize_ip(s: &str) -> Option<String> {
    let (addr, prefix) = match s.split_once('/') {
        Some((a, p)) => (a, Some(p.parse::<u8>().ok()?)),
        None => (s, None),
    };
    let ip: IpAddr = addr.parse().ok()?;
    let max = if ip.is_ipv4() { 32 } else { 128 };
    match prefix {
        Some(p) if p > max => None,
        Some(p) => Some(format!("{ip}/{p}")),
        None => Some(ip.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::UserList::*;

    #[test]
    fn domains() {
        assert_eq!(
            Domains
                .normalize("https://www.YouTube.com/watch?v=1")
                .unwrap(),
            "youtube.com"
        );
        assert_eq!(Domains.normalize("discord.gg:443").unwrap(), "discord.gg");
        assert_eq!(Domains.normalize("*.example.com").unwrap(), "example.com");
        assert!(Domains.normalize("localhost").is_err());
        assert!(Domains.normalize("1.2.3.4").is_err());
        assert!(Domains.normalize("bad_domain.com").is_err());
    }

    #[test]
    fn ips() {
        assert_eq!(ExcludeIps.normalize(" 10.0.0.0/8 ").unwrap(), "10.0.0.0/8");
        assert_eq!(ExcludeIps.normalize("2001:db8::1").unwrap(), "2001:db8::1");
        assert!(ExcludeIps.normalize("10.0.0.0/33").is_err());
        assert!(ExcludeIps.normalize("example.com").is_err());
    }
}
