//! TOR с мостами. На Windows используется tor.exe из комплекта, на Linux - системный tor.

use crate::Hub;
use crate::logger::info;
use crate::platform::{self, PROXY_HOST, PROXY_PORT, PidFile};
use regex::Regex;
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};

const WORKER_URL: &str = "https://winter-cherry-c701.f1zzencontact.workers.dev/";
const LOG_FILE: &str = "latest.log";
const START_TIMEOUT: Duration = Duration::from_secs(180);

/// Номер текущего запуска: поток чтения лога завершается, когда номер меняется.
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Последний процент "Bootstrapped N%".
static PROGRESS: AtomicU32 = AtomicU32::new(0);

struct Binaries {
    tor: PathBuf,
    /// ClientTransportPlugin для obfs4 (и webtunnel, если поддерживается)
    transport: Option<String>,
    webtunnel: bool,
    geoip: Option<(String, String)>,
}

pub struct Tor;

impl Tor {
    pub fn dir() -> PathBuf {
        #[cfg(windows)]
        let name = "tor-expert-bundle";
        #[cfg(not(windows))]
        let name = "tor";
        let dir = crate::paths::data_dir().join(name);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn pid_file() -> PathBuf {
        Self::dir().join("tor.pid")
    }

    #[cfg(windows)]
    fn binaries(app: &Hub) -> Result<Binaries, String> {
        Self::sync_resources(app)?;
        Ok(Binaries {
            tor: Self::dir().join("tor").join("tor.exe"),
            // относительный путь: exec в torrc не любит пробелы в путях
            transport: Some("./tor/pluggable_transports/lyrebird.exe".into()),
            webtunnel: true,
            geoip: Some(("./data/geoip".into(), "./data/geoip6".into())),
        })
    }

    /// macOS: TOR из Z.UST.app (Tor Expert Bundle), без него - из Homebrew.
    #[cfg(target_os = "macos")]
    fn binaries(app: &Hub) -> Result<Binaries, String> {
        let bundled = crate::paths::resource_dir()
            .join("tor")
            .join("tor")
            .join("tor");
        if !bundled.is_file() {
            return Self::system_binaries();
        }
        Self::sync_resources(app)?;
        Ok(Binaries {
            tor: Self::dir().join("tor").join("tor"),
            transport: Some("./tor/pluggable_transports/lyrebird".into()),
            webtunnel: true,
            geoip: Some(("./data/geoip".into(), "./data/geoip6".into())),
        })
    }

    #[cfg(target_os = "linux")]
    fn binaries(_app: &Hub) -> Result<Binaries, String> {
        Self::system_binaries()
    }

    /// tor и мосты, установленные в системе.
    #[cfg(unix)]
    fn system_binaries() -> Result<Binaries, String> {
        #[cfg(target_os = "macos")]
        let hint = "TOR не установлен. Установите его через Homebrew: brew install tor obfs4proxy";
        #[cfg(not(target_os = "macos"))]
        let hint =
            "TOR не установлен. Установите пакет tor (и lyrebird или obfs4proxy для мостов obfs4).";
        let tor = platform::find_in_path("tor").ok_or(hint)?;
        let lyrebird = platform::find_in_path("lyrebird");
        let webtunnel = lyrebird.is_some();
        let transport = lyrebird
            .or_else(|| platform::find_in_path("obfs4proxy"))
            .map(|p| p.to_string_lossy().to_string());
        let geoip = [
            "/usr/share/tor/geoip",
            "/usr/local/share/tor/geoip",
            "/opt/homebrew/share/tor/geoip",
        ]
        .into_iter()
        .find(|p| Path::new(p).exists())
        .map(|p| (p.to_string(), format!("{p}6")));
        Ok(Binaries {
            tor,
            transport,
            webtunnel,
            geoip,
        })
    }

    /// Копирует tor из ресурсов приложения. Повторно - только после обновления Zust.
    #[cfg(any(windows, target_os = "macos"))]
    fn sync_resources(app: &Hub) -> Result<(), String> {
        let target = Self::dir();
        let marker = target.join(".zust-version");
        let version = env!("CARGO_PKG_VERSION");
        let exe = format!("tor{}", platform::EXE_SUFFIX);
        if target.join("tor").join(&exe).exists()
            && fs::read_to_string(&marker).ok().as_deref() == Some(version)
        {
            return Ok(());
        }
        let source = crate::paths::resource_dir().join("tor");
        if !source.is_dir() {
            return Err(format!("Не найдены файлы TOR: {}", source.display()));
        }
        let copied =
            copy_tree(&source, &target).map_err(|e| format!("Ошибка копирования TOR: {e}"))?;
        // карантин от скачанного zip: без снятия macOS не даст запустить tor
        #[cfg(target_os = "macos")]
        platform::run(
            "/usr/bin/xattr",
            &["-dr", "com.apple.quarantine", &target.to_string_lossy()],
        );
        let _ = fs::write(marker, version);
        info(app, &format!("[TOR] Ресурсы развёрнуты: {copied} файлов"));
        Ok(())
    }

    pub fn is_running() -> bool {
        PidFile::alive(&Self::pid_file(), "tor").is_some()
    }

    pub fn progress() -> u32 {
        PROGRESS.load(Ordering::Relaxed)
    }

    /// Последние строки лога TOR - чтобы показать их после перезапуска интерфейса.
    pub fn replay_logs(app: &Hub) {
        if let Ok(content) = fs::read_to_string(Self::dir().join(LOG_FILE)) {
            let lines: Vec<&str> = content.lines().collect();
            for line in &lines[lines.len().saturating_sub(15)..] {
                let _ = app.emit("log-event", format!("[TOR] {line}"));
            }
        }
    }

    pub fn stop() {
        GENERATION.fetch_add(1, Ordering::SeqCst);
        PROGRESS.store(0, Ordering::Relaxed);
        platform::disable_system_proxy_if_ours();
        PidFile::kill(&Self::pid_file(), "tor");
    }

    pub async fn start(app: Hub) -> Result<(), String> {
        Self::stop();
        let bins = Self::binaries(&app)?;
        let settings = crate::settings::load();
        let dir = Self::dir();
        let torrc = dir.join("torrc");

        let saved = Self::bridges_from_file(&torrc);
        let bridges = if settings.refresh_bridges || saved.is_empty() {
            info(&app, "[TOR] Получаю новые мосты...");
            Self::collect_bridges(&app).await?
        } else {
            info(&app, "[TOR] Использую сохранённые мосты.");
            saved
        };

        let bridges: Vec<String> = bridges
            .into_iter()
            .filter(|b| bridge_supported(b, &bins))
            .collect();
        if bridges.is_empty() {
            return Err(
                "Нет мостов, которые поддерживает установленный TOR (нужен lyrebird/obfs4proxy)."
                    .into(),
            );
        }

        fs::write(&torrc, Self::config(&bins, &bridges)).map_err(|e| e.to_string())?;
        info(
            &app,
            &format!("[TOR] Конфиг записан, мостов: {}", bridges.len()),
        );

        Self::spawn(&app, &bins.tor, &dir).await
    }

    async fn collect_bridges(app: &Hub) -> Result<Vec<String>, String> {
        let (worker, direct) = tokio::join!(fetch_worker_bridges(), fetch_direct_bridges());
        let mut all = Vec::new();
        match worker {
            Ok(b) => all.extend(b),
            Err(e) => info(app, &format!("[TOR] Облачный метод не удался: {e}")),
        }
        let direct_failed = match direct {
            Ok(b) => {
                all.extend(b);
                false
            }
            Err(e) => {
                info(app, &format!("[TOR] Прямой запрос не удался: {e}"));
                true
            }
        };
        all.sort();
        all.dedup();

        if all.is_empty() {
            // torproject.org заблокирован - предлагаем настроить zapret
            if direct_failed
                && (!crate::bypass::zapret::Zapret::is_running()
                    || !crate::bypass::zapret::Zapret::domain_in_list(
                        "list-general.txt",
                        "torproject.org",
                    ))
            {
                let _ = app.emit("zapret_repair", ());
                return Err("Похоже, провайдер блокирует torproject.org".into());
            }
            return Err("Не удалось получить мосты ни одним из способов".into());
        }
        Ok(all)
    }

    fn config(bins: &Binaries, bridges: &[String]) -> String {
        let mut c = format!(
            "SocksPort {PROXY_HOST}:{PROXY_PORT}\n\
             DataDirectory ./data\n\
             UseBridges 1\n\
             UpdateBridgesFromAuthority 0\n\
             AvoidDiskWrites 1\n\
             SafeLogging 1\n\
             ClientOnly 1\n\
             UseEntryGuards 1\n\
             NumPrimaryGuards 3\n\
             NumEntryGuards 3\n\
             CircuitBuildTimeout 10\n\
             LearnCircuitBuildTimeout 0\n\
             KeepalivePeriod 5\n\
             Log notice file ./{LOG_FILE}\n"
        );
        if let Some((v4, v6)) = &bins.geoip {
            c.push_str(&format!("GeoIPFile {v4}\nGeoIPv6File {v6}\n"));
        }
        if let Some(pt) = &bins.transport {
            c.push_str(&format!("ClientTransportPlugin obfs4 exec {pt}\n"));
            if bins.webtunnel {
                c.push_str(&format!("ClientTransportPlugin webtunnel exec {pt}\n"));
            }
        }
        for b in bridges {
            c.push_str(&format!("Bridge {}\n", strip_bridge(b)));
        }
        c
    }

    fn bridges_from_file(path: &Path) -> Vec<String> {
        fs::read_to_string(path)
            .map(|t| {
                t.lines()
                    .filter(|l| l.starts_with("Bridge "))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn spawn(app: &Hub, tor: &Path, dir: &Path) -> Result<(), String> {
        let data = dir.join("data");
        let _ = fs::create_dir_all(&data);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&data, fs::Permissions::from_mode(0o700));
        }

        let log_path = dir.join(LOG_FILE);
        // читаем лог только с текущего конца, чтобы не подхватить строки прошлого запуска
        let offset = fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0);

        let child = platform::command(tor)
            .args(["-f", "torrc"])
            .current_dir(dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Не удалось запустить TOR: {e}"))?;
        PidFile::write(&Self::pid_file(), child.id(), "");
        drop(child);

        let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        PROGRESS.store(0, Ordering::Relaxed);
        tail_log(app.clone(), log_path.clone(), offset, generation);

        let started = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if GENERATION.load(Ordering::SeqCst) != generation {
                return Err("interrupted".into());
            }
            if PROGRESS.load(Ordering::Relaxed) >= 100 {
                info(app, "[TOR] Подключение установлено.");
                return Ok(());
            }
            if !Self::is_running() {
                let last = fs::read_to_string(&log_path)
                    .ok()
                    .and_then(|t| t.lines().last().map(str::to_string))
                    .unwrap_or_default();
                return Err(format!("TOR завершился: {last}"));
            }
            if started.elapsed() > START_TIMEOUT {
                Self::stop();
                return Err("TOR не подключился за 3 минуты. Попробуйте обновить мосты.".into());
            }
        }
    }
}

/// Поток, который читает лог TOR и пересылает строки и прогресс в интерфейс.
fn tail_log(app: Hub, path: PathBuf, offset: u64, generation: u64) {
    std::thread::spawn(move || {
        let re = Regex::new(r"Bootstrapped (\d+)%").unwrap();
        let alive = || GENERATION.load(Ordering::SeqCst) == generation;

        while !path.exists() {
            if !alive() {
                return;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        let Ok(mut file) = fs::File::open(&path) else {
            return;
        };
        // лог мог быть пересоздан - тогда читаем с начала
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        let _ = file.seek(SeekFrom::Start(if offset <= len { offset } else { 0 }));
        let mut reader = BufReader::new(file);
        let mut line = String::new();

        while alive() {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => std::thread::sleep(Duration::from_millis(300)),
                Ok(_) => {
                    let text = line.trim();
                    let _ = app.emit("log-event", format!("[TOR] {text}"));
                    if let Some(p) = re.captures(text).and_then(|c| c[1].parse::<u32>().ok()) {
                        PROGRESS.store(p, Ordering::Relaxed);
                        let _ = app.emit("tor-progress", p);
                    }
                }
            }
        }
    });
}

fn strip_bridge(b: &str) -> &str {
    b.trim().trim_start_matches("Bridge ").trim()
}

/// Поддерживает ли установленный набор программ этот тип моста.
fn bridge_supported(bridge: &str, bins: &Binaries) -> bool {
    let b = strip_bridge(bridge);
    let kind = b.split_whitespace().next().unwrap_or_default();
    match kind {
        "obfs4" => bins.transport.is_some(),
        "webtunnel" => bins.transport.is_some() && bins.webtunnel,
        // обычный мост: "1.2.3.4:443 FINGERPRINT" или "[::1]:443 FINGERPRINT"
        k => {
            k.contains(':')
                && (k.starts_with('[') || k.chars().next().is_some_and(|c| c.is_ascii_digit()))
        }
    }
}

async fn fetch_worker_bridges() -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = client
        .get(WORKER_URL)
        // воркер пропускает только точную строку "Zust/1.0", с номером версии отвечает 403
        .header("User-Agent", "Zust/1.0")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let bridges: Vec<String> = json["bridges"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    if bridges.is_empty() {
        Err("Список мостов пуст".into())
    } else {
        Ok(bridges)
    }
}

async fn fetch_direct_bridges() -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0")
        .build()
        .map_err(|e| e.to_string())?;
    let urls = ["obfs4", "vanilla", "webtunnel"]
        .map(|t| format!("https://bridges.torproject.org/bridges?transport={t}"));
    let responses = futures::future::join_all(urls.iter().map(|u| {
        client
            .get(u)
            .header("Accept-Language", "ru-RU,ru;q=0.9,en;q=0.8")
            .send()
    }))
    .await;

    let mut html = String::new();
    for r in responses.into_iter().flatten() {
        if let Ok(t) = r.text().await {
            html.push_str(&t);
        }
    }
    if html.is_empty() {
        return Err("bridges.torproject.org не ответил".into());
    }

    let patterns = [
        r"obfs4\s+[0-9a-fA-F.:\[\]]+\s+[0-9a-fA-F]+[^<]*",
        r"[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+:[0-9]+\s+[0-9a-fA-F]{40}",
        r"webtunnel\s+https://[^\s<]+",
    ];
    let mut bridges: Vec<String> = patterns
        .iter()
        .flat_map(|p| {
            Regex::new(p)
                .unwrap()
                .find_iter(&html)
                .map(|m| {
                    format!("Bridge {}", m.as_str().trim())
                        .replace("&#43;", "+")
                        .replace("&amp;", "&")
                        .replace("&quot;", "\"")
                })
                .collect::<Vec<_>>()
        })
        .collect();
    bridges.sort();
    bridges.dedup();
    if bridges.is_empty() {
        Err("Мосты не найдены в ответе (возможно, капча)".into())
    } else {
        Ok(bridges)
    }
}

#[cfg(any(windows, target_os = "macos"))]
fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<u32> {
    fs::create_dir_all(dst)?;
    let mut n = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            n += copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
            n += 1;
        }
    }
    Ok(n)
}
