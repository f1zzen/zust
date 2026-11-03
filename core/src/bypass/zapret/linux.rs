//! zapret на Linux: nfqws + nftables. Всё от root - одним скриптом через pkexec.
//! Списки копируются в /run/zust (root), nfqws лежит в /var/lib/zust: /run часто noexec.

use super::{Launch, Zapret, ZapretDirs};
use crate::Hub;
use crate::logger::info;
use crate::platform::{PidFile, find_in_path, run_privileged, shell_quote};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const RUN_DIR: &str = "/run/zust";
const BIN_DIR: &str = "/var/lib/zust";
const QNUM: u16 = 200;
const NFT_TABLE: &str = "zust";

#[derive(Serialize, Deserialize, Default)]
struct State {
    strategy: String,
    ipset: String,
}

fn state_path() -> PathBuf {
    Zapret::path("state.json")
}

fn read_state() -> Option<State> {
    serde_json::from_str(&fs::read_to_string(state_path()).ok()?).ok()
}

fn pid_file() -> PathBuf {
    Path::new(RUN_DIR).join("nfqws.pid")
}

pub fn is_running() -> bool {
    PidFile::alive(&pid_file(), "nfqws").is_some()
}

pub fn current_strategy() -> Option<String> {
    read_state().map(|s| s.strategy)
}

pub fn current_ipset() -> Option<String> {
    read_state().map(|s| s.ipset)
}

/// nfqws: из PATH, из стандартной установки zapret (/opt/zapret) или скачанный нами.
fn find_nfqws() -> Option<PathBuf> {
    find_in_path("nfqws")
        .or_else(|| Some(PathBuf::from("/opt/zapret/nfq/nfqws")).filter(|p| p.is_file()))
        .or_else(|| Some(Zapret::path("bin").join("nfqws")).filter(|p| p.is_file()))
}

pub async fn prepare(app: &Hub) -> Result<(), String> {
    if find_in_path("nft").is_none() && !Path::new("/usr/sbin/nft").exists() {
        return Err("Не найден nft. Установите пакет nftables.".into());
    }
    if find_nfqws().is_none() {
        download_nfqws(app).await?;
    }
    Ok(())
}

/// Закреплённый релиз bol-van/zapret: хеш проверяется до распаковки, чужой nfqws от root не запустится.
const NFQWS_RELEASE: &str =
    "https://github.com/bol-van/zapret/releases/download/v72.13/zapret-v72.13.tar.gz";
const NFQWS_RELEASE_SHA256: &str =
    "25c74e6c5f48963fa244c2955e76694a07c39447245a0457e2efdc74b3317e68";

async fn download_nfqws(app: &Hub) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    info(app, "nfqws не найден - скачиваю zapret v72.13 с GitHub...");
    let client = reqwest::Client::builder()
        .user_agent(concat!("zust/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let bytes = client
        .get(NFQWS_RELEASE)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    let hash: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if hash != NFQWS_RELEASE_SHA256 {
        return Err("Архив zapret не совпадает с ожидаемым (sha256) - установка отменена".into());
    }

    let dest = Zapret::path("bin").join("nfqws");
    let dest2 = dest.clone();
    tokio::task::spawn_blocking(move || extract_nfqws(&bytes, &dest2))
        .await
        .map_err(|e| e.to_string())??;
    info(app, &format!("nfqws установлен: {}", dest.display()));
    Ok(())
}

fn extract_nfqws(archive: &[u8], dest: &Path) -> Result<(), String> {
    let arch_dirs: &[&str] = match std::env::consts::ARCH {
        "x86_64" => &["linux-x86_64", "x86_64"],
        "aarch64" => &["linux-arm64", "aarch64"],
        "x86" => &["linux-x86", "x86"],
        "arm" => &["linux-arm", "arm"],
        other => return Err(format!("Архитектура {other} не поддерживается")),
    };
    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    for entry in tar.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry
            .path()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .to_string();
        if arch_dirs
            .iter()
            .any(|d| path.ends_with(&format!("/{d}/nfqws")))
        {
            let mut data = Vec::new();
            entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
            fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(dest, data).map_err(|e| e.to_string())?;
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dest, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err("nfqws для этой архитектуры не найден в архиве".into())
}

/// "80,443, 1024-65535" → "80, 443, 1024-65535" (пустые элементы убираются)
fn nft_ports(ports: &str) -> String {
    ports
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit() || c == '-'))
        .collect::<Vec<_>>()
        .join(", ")
}

fn nft_ruleset(tcp: &str, udp: &str) -> String {
    let mut rules = String::new();
    for (proto, ports) in [("tcp", nft_ports(tcp)), ("udp", nft_ports(udp))] {
        if !ports.is_empty() {
            rules.push_str(&format!(
                "        {proto} dport {{ {ports} }} ct original packets 1-12 queue num {QNUM} bypass\n"
            ));
        }
    }
    format!(
        "table inet {NFT_TABLE} {{\n\
         \x20   chain post {{\n\
         \x20       type filter hook postrouting priority mangle; policy accept;\n\
         \x20       meta mark & 0x40000000 == 0x40000000 return\n\
         {rules}\
         \x20   }}\n\
         }}\n"
    )
}

/// Общая часть скриптов: остановить старый nfqws и убрать наши правила.
pub fn stop_snippet() -> String {
    format!(
        "if [ -f {RUN_DIR}/nfqws.pid ]; then kill \"$(cat {RUN_DIR}/nfqws.pid)\" 2>/dev/null || true; fi\n\
         nft delete table inet {NFT_TABLE} 2>/dev/null || true\n\
         rm -rf {RUN_DIR}\n"
    )
}

pub fn start(
    app: &Hub,
    name: &str,
    ipset: &str,
    raw: &str,
    game_filter: bool,
) -> Result<(), String> {
    let script = start_script(app, ipset, raw, game_filter, "")?;
    run_privileged(&script).map_err(|e| format!("nfqws не запустился: {e}"))?;
    save_state(name, ipset);
    Ok(())
}

/// Скрипт запуска от root. `tag` отличает файлы, когда скриптов готовится несколько (тест стратегий).
pub fn start_script(
    app: &Hub,
    ipset: &str,
    raw: &str,
    game_filter: bool,
    tag: &str,
) -> Result<String, String> {
    let nfqws = find_nfqws().ok_or("nfqws не найден")?;
    let run_dirs = ZapretDirs {
        bin: PathBuf::from(RUN_DIR).join("bin"),
        lists: PathBuf::from(RUN_DIR).join("lists"),
        ipset_configs: PathBuf::from(RUN_DIR).join("ipset-configs"),
    };
    let Launch {
        args,
        tcp_ports,
        udp_ports,
    } = Zapret::build_launch(raw, &run_dirs, ipset, game_filter)?;
    // --wf-* - фильтры WinDivert, в Linux их роль выполняют правила nftables
    let args: Vec<String> = args
        .into_iter()
        .filter(|a| !a.starts_with("--wf-"))
        .collect();
    let tcp = if tcp_ports.is_empty() {
        "80,443".to_string()
    } else {
        tcp_ports
    };
    let udp = if udp_ports.is_empty() {
        "443".to_string()
    } else {
        udp_ports
    };

    let nft_file = Zapret::path(&format!("zust{tag}.nft"));
    fs::write(&nft_file, nft_ruleset(&tcp, &udp)).map_err(|e| e.to_string())?;

    let q = |p: &Path| shell_quote(&p.to_string_lossy());
    let src = Zapret::path("");
    let args_q = args
        .iter()
        .map(|a| shell_quote(a))
        .collect::<Vec<_>>()
        .join(" ");
    info(app, &format!("Аргументы nfqws: {}", args.join(" ")));

    Ok(format!(
        "set -e\n\
         {stop}\
         mkdir -p {RUN_DIR}/bin {RUN_DIR}/lists {RUN_DIR}/ipset-configs\n\
         cp {src}/lists/*.txt {RUN_DIR}/lists/\n\
         cp {src}/bin/*.bin {RUN_DIR}/bin/ 2>/dev/null || true\n\
         cp {src}/ipset-configs/*.txt {RUN_DIR}/ipset-configs/ 2>/dev/null || true\n\
         mkdir -p {BIN_DIR}\n\
         cp {nfqws} {BIN_DIR}/nfqws.new\n\
         chmod 755 {BIN_DIR}/nfqws.new\n\
         mv -f {BIN_DIR}/nfqws.new {BIN_DIR}/nfqws\n\
         chmod -R a+rX {RUN_DIR}\n\
         nft -f {nft}\n\
         nohup {BIN_DIR}/nfqws --qnum={QNUM} {args_q} < /dev/null > {RUN_DIR}/nfqws.log 2>&1 &\n\
         echo $! > {RUN_DIR}/nfqws.pid\n\
         chmod a+r {RUN_DIR}/nfqws.pid\n\
         sleep 1\n\
         if ! kill -0 \"$(cat {RUN_DIR}/nfqws.pid)\" 2>/dev/null; then\n\
         \x20 cat {RUN_DIR}/nfqws.log >&2\n\
         \x20 nft delete table inet {NFT_TABLE} 2>/dev/null || true\n\
         \x20 exit 1\n\
         fi\n\
         chmod a+r {RUN_DIR}/nfqws.log\n",
        stop = stop_snippet(),
        src = q(&src),
        nfqws = q(&nfqws),
        nft = q(&nft_file),
    ))
}

pub fn save_state(name: &str, ipset: &str) {
    let state = State {
        strategy: name.to_string(),
        ipset: ipset.to_string(),
    };
    let _ = fs::write(
        state_path(),
        serde_json::to_string(&state).unwrap_or_default(),
    );
}

pub fn stop(app: &Hub) -> Result<(), String> {
    // нечего останавливать - не дёргаем пользователя запросом пароля
    if !Path::new(RUN_DIR).exists() {
        return Ok(());
    }
    run_privileged(&stop_snippet())?;
    info(app, "nfqws остановлен, правила nftables удалены.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports() {
        assert_eq!(nft_ports("80,443, 2053,,12"), "80, 443, 2053, 12");
        assert_eq!(
            nft_ports("19294-19344,50000-50100"),
            "19294-19344, 50000-50100"
        );
        assert_eq!(nft_ports("%GameFilter%"), "");
    }

    #[test]
    fn ruleset_skips_empty() {
        let r = nft_ruleset("80,443", "");
        assert!(r.contains("tcp dport { 80, 443 }"));
        assert!(!r.contains("udp dport"));
    }
}
