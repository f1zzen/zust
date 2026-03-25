use crate::utils::info;
use rand::Rng;
use regex::Regex;
use std::{
    fs,
    io::{BufRead, BufReader},
    net::TcpStream,
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

pub struct Tor;
pub struct HotSpot;
const WORKER_URL: &str = "https://winter-cherry-c701.f1zzencontact.workers.dev/";
const TOR_LOGS: &str = "latest.log";

impl HotSpot {
    pub fn run(app: AppHandle, manual_port: Option<u16>) -> Result<u16, String> {
        let _ = Self::stop();
        let mut rng = rand::thread_rng();
        let max_attempts = 15;

        let mut frpc_path = std::env::current_exe().unwrap();
        frpc_path.pop();
        frpc_path.push("frp");
        let bin_path = frpc_path.join("frpc.exe");
        let config_path = frpc_path.join("config.toml");

        let mut next_port = manual_port.unwrap_or_else(|| rng.gen_range(20000..60000));

        for _ in 1..=max_attempts {
            let id = &Uuid::new_v4().to_string()[..8];
            let toml_content = format!(
                "serverAddr = \"frp.freefrp.net\"\n\
        serverPort = 7000\n\n\
        auth.method = \"token\"\n\
        auth.token = \"freefrp.net\"\n\n\
        [[proxies]]\n\
        name = \"zust_4ever_{id}\"\n\
        type = \"tcp\"\n\
        localIP = \"127.0.0.1\"\n\
        localPort = 9050\n\
        remotePort = {next_port}"
            );

            fs::write(&config_path, toml_content).map_err(|e| e.to_string())?;

            let mut child = Command::new(&bin_path)
                .args(["-c", config_path.to_str().unwrap()])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(0x08000000)
                .spawn()
                .map_err(|e| e.to_string())?;

            let stdout = child.stdout.take().unwrap();
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let mut port_busy = false;

            loop {
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        if let Ok(Some(_)) = child.try_wait() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                    Ok(_) => {
                        let text = line.trim().to_string();
                        app.emit("log-event", format!("[FRP] {}", text)).ok();

                        if text.contains("start proxy success") {
                            return Ok(next_port);
                        }

                        if text.contains("already used") || text.contains("port not allowed") {
                            port_busy = true;
                            let _ = child.kill();
                            break;
                        }
                        line.clear();
                    }
                    Err(_) => break,
                }
            }

            if port_busy {
                next_port = rng.gen_range(20000..60000);
                app.emit(
                    "log-event",
                    format!("Порт {} занят, подбор нового...", next_port),
                )
                .ok();
            } else {
                return Err("FRP был принудительно закрыт.".into());
            }
        }
        Err("Превышено количество попыток запуска".into())
    }

    pub fn stop() -> Result<(), String> {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "frpc.exe", "/T"])
            .creation_flags(0x08000000)
            .output();
        Ok(())
    }

    pub fn get_status() -> bool {
        let output = Command::new("tasklist")
            .args(["/NH", "/FI", "IMAGENAME eq frpc.exe"])
            .creation_flags(0x08000000)
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            return stdout.contains("frpc.exe");
        }
        false
    }
}

impl Tor {
    async fn fetch_bridges_directly() -> Result<Vec<String>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0")
            .build()
            .map_err(|e| e.to_string())?;
        let obfs4_url = "https://bridges.torproject.org/bridges?transport=obfs4";
        let vanilla_url = "https://bridges.torproject.org/bridges?transport=vanilla";
        let web_url = "https://bridges.torproject.org/bridges?transport=webtunnel";

        let headers = [
            ("Host", "bridges.torproject.org"),
            ("Accept-Language", "ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7"),
        ];

        let request_builder = |url: &str| {
            let mut rb = client.get(url);
            for (k, v) in headers {
                rb = rb.header(k, v);
            }
            rb.send()
        };
        let (res_obfs4, res_vanilla, res_web) = futures::join!(
            request_builder(obfs4_url),
            request_builder(vanilla_url),
            request_builder(web_url)
        );
        let mut combined_html = String::new();
        for res in [res_obfs4, res_vanilla, res_web] {
            if let Ok(r) = res {
                if let Ok(t) = r.text().await {
                    combined_html.push_str(&t);
                }
            }
        }

        if combined_html.is_empty() {
            return Err("Не удалось получить ответ от Tor Project локально".into());
        }
        let obfs4_re = Regex::new(r"obfs4\s+[0-9a-fA-F.:\[\]]+\s+[0-9a-fA-F]+[^<]*").unwrap();
        let vanilla_re =
            Regex::new(r"[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+:[0-9]+\s+[0-9a-fA-F]{40}").unwrap();
        let web_re = Regex::new(r"webtunnel\s+https://[^\s<]+").unwrap();

        let mut bridges = Vec::new();
        for cap in obfs4_re.find_iter(&combined_html) {
            bridges.push(format!("Bridge {}", cap.as_str().trim()));
        }
        for cap in vanilla_re.find_iter(&combined_html) {
            bridges.push(format!("Bridge {}", cap.as_str().trim()));
        }
        for cap in web_re.find_iter(&combined_html) {
            bridges.push(format!("Bridge {}", cap.as_str().trim()));
        }
        let mut final_bridges: Vec<String> = bridges
            .into_iter()
            .map(|b| {
                b.replace("&#43;", "+")
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"")
            })
            .collect();

        final_bridges.sort();
        final_bridges.dedup();

        if final_bridges.is_empty() {
            return Err("Мосты не найдены в ответе (возможно, капча)".into());
        }

        Ok(final_bridges)
    }

    pub async fn start(app: AppHandle) -> Result<(), String> {
        let _ = Self::stop(app.clone());
        Self::sync_resources(&app)?;

        let settings = crate::settings::load_settings();
        let base_dir = Self::tor_data_path(&app, "");
        let log_path = Self::tor_data_path(&app, TOR_LOGS);

        if log_path.exists() {
            let _ = fs::remove_file(&log_path);
        }

        let torrc_path = base_dir.join("torrc");
        let bridges_in_file = if torrc_path.exists() {
            Self::get_current_bridges_from_file(&torrc_path).ok()
        } else {
            None
        };

        let should_refresh = settings.refresh_bridges
            || !torrc_path.exists()
            || bridges_in_file.as_ref().map_or(true, |b| b.is_empty());

        let bridges = if should_refresh {
            info(&app, "[TOR] Требуется обновление мостов. Сбор мостов.");

            let worker_future = Self::get_bridges(app.clone());
            let local_future = Self::fetch_bridges_directly();

            let (worker_res, local_res) = tokio::join!(worker_future, local_future);

            let mut combined = Vec::new();
            let mut local_failures = 0;

            match worker_res {
                Ok(b) => combined.extend(b),
                Err(e) => info(&app, &format!("[TOR] Облачный метод не удался: {}", e)),
            }

            match local_res {
                Ok(b) => combined.extend(b),
                Err(e) => {
                    local_failures += 1;
                    info(&app, &format!("[TOR] Локальный метод не удался: {}", e));
                }
            }

            if local_failures >= 1 {
                let zapret_running = crate::bypass::zapret::Zapret::get_status();
                let has_domain = crate::bypass::zapret::Zapret::check_domain_in_list(
                    &app,
                    "list-general.txt",
                    "torproject.org",
                );

                if !zapret_running || !has_domain {
                    info(
                        &app,
                        "[TOR] Обнаружены проблемы с доступом. Проверка Запрета...",
                    );
                    app.emit("zapret_repair", ()).ok();
                    return Err("Замедление TORPROJECT со стороны провайдера.".into());
                }
            }

            if combined.is_empty() {
                return Err("Не удалось получить мосты ни одним из способов.".into());
            }

            combined.sort();
            combined.dedup();
            combined
        } else {
            info(&app, "[TOR] Загрузка существующих мостов.");
            bridges_in_file.unwrap()
        };

        let config = Self::generate_config(app.clone(), bridges)?;
        info(&app, "[TOR] Конфиг успешно модифицирован.");
        Self::exec_tor_await(app.clone(), &config).await?;

        info(&app, "[TOR] Сервис TOR-а успешно запущен.");
        Ok(())
    }

    pub async fn get_bridges(_app: AppHandle) -> Result<Vec<String>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| e.to_string())?;

        let res = client
            .get(WORKER_URL)
            .header("User-Agent", "Zust/1.0")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().is_success() {
            let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;

            if let Some(bridges_array) = json.get("bridges").and_then(|b| b.as_array()) {
                let bridges: Vec<String> = bridges_array
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();

                if !bridges.is_empty() {
                    return Ok(bridges);
                }
            }
        }
        Err("Список мостов пуст или недоступен".into())
    }

    pub fn generate_config(app: AppHandle, bridges: Vec<String>) -> Result<String, String> {
        let base_dir = Self::tor_data_path(&app, "");
        let torrc_path = base_dir.join("torrc");
        let data_dir = base_dir.join("data");
        fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
        let mut content = format!(
            "SocksPort 127.0.0.1:9050\n\
    DataDirectory ./data\n\
    GeoIPFile ./data/geoip\n\
    GeoIPv6File ./data/geoip6\n\
    ClientTransportPlugin obfs4 exec ./tor/pluggable_transports/lyrebird.exe\n\
    ClientTransportPlugin webtunnel exec ./tor/pluggable_transports/lyrebird.exe\n\
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
    Log notice file ./latest.log\n"
        );

        for b in bridges {
            let trimmed = b.trim();
            if trimmed.is_empty() {
                continue;
            }
            content.push_str(&format!("Bridge {}\n", trimmed.replace("Bridge ", "")));
        }

        fs::write(&torrc_path, content).map_err(|e| e.to_string())?;
        Ok(torrc_path.to_string_lossy().into_owned())
    }

    pub fn check_existing_tor(app: AppHandle) -> bool {
        let output = Command::new("tasklist")
            .args(["/NH", "/FI", "IMAGENAME eq tor.exe"])
            .creation_flags(0x08000000)
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.contains("tor.exe") {
                Self::sync_logs_from_file(&app);
                return true;
            }
        }
        false
    }

    fn sync_logs_from_file(app: &AppHandle) {
        let log_path = Self::tor_data_path(app, TOR_LOGS);
        if let Ok(content) = fs::read_to_string(log_path) {
            let all_lines: Vec<&str> = content.lines().collect();

            let tor_logs: Vec<String> = all_lines
                .iter()
                .rev()
                .take(15)
                .rev()
                .map(|&s| s.to_string())
                .collect();

            for log_line in tor_logs {
                let _ = app.emit("log-event", format!("[TOR] {}", log_line));
            }
        }
    }

    pub fn tor_data_path(app: &AppHandle, sub: &str) -> PathBuf {
        let mut path = app.path().app_config_dir().expect("config dir error");
        path.push("tor-expert-bundle");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path.join(sub)
    }

    fn get_current_bridges_from_file(path: &PathBuf) -> Result<Vec<String>, String> {
        let file = fs::File::open(path).map_err(|_| "Файл конфигурации не найден".to_string())?;
        let reader = BufReader::new(file);
        let bridges: Vec<String> = reader
            .lines()
            .filter_map(|l| l.ok())
            .filter(|l| l.starts_with("Bridge "))
            .map(|l| l.to_string())
            .collect();

        if bridges.is_empty() {
            return Err("Файл конфигурации пуст!".into());
        }
        Ok(bridges)
    }

    pub fn stop(_app: AppHandle) -> Result<(), String> {
        let _ = Self::switch_proxy(false);
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "tor.exe", "/T"])
            .creation_flags(0x08000000)
            .output();
        Ok(())
    }

    pub fn sync_resources(app: &AppHandle) -> Result<(), String> {
        let target_dir = Self::tor_data_path(app, "");
        let target_bin = target_dir.join("tor.exe");
        if target_bin.exists() {
            return Ok(());
        }

        let mut source_dir = std::env::current_exe().map_err(|e| e.to_string())?;
        source_dir.pop();
        source_dir.push("tor");

        if source_dir.exists() && source_dir.is_dir() {
            fn copy_dir_contents(src: PathBuf, dst: PathBuf) -> std::io::Result<u32> {
                let mut count = 0;
                if !dst.exists() {
                    fs::create_dir_all(&dst)?;
                }

                for entry in fs::read_dir(src)? {
                    let entry = entry?;
                    let ty = entry.file_type()?;
                    let dest_path = dst.join(entry.file_name());

                    if ty.is_dir() {
                        count += copy_dir_contents(entry.path(), dest_path)?;
                    } else {
                        fs::copy(entry.path(), &dest_path)?;
                        count += 1;
                    }
                }
                Ok(count)
            }

            match copy_dir_contents(source_dir, target_dir) {
                Ok(copied) => {
                    info(app, &format!("[TOR] Ресурсы развернуты: {} файлов", copied));
                    return Ok(());
                }
                Err(e) => return Err(format!("COPY_ERROR: {}", e)),
            }
        }

        Err(format!("SOURCE_NOT_FOUND: {:?}", source_dir))
    }

    pub fn switch_proxy(enable: bool) -> Result<(), std::io::Error> {
        use winreg::{RegKey, enums::*};
        let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            KEY_SET_VALUE,
        )?;
        if enable {
            let _ = key.set_value("ProxyEnable", &1u32);
            let _ = key.set_value("ProxyServer", &"socks=127.0.0.1:9050");
        } else {
            let _ = key.set_value("ProxyEnable", &0u32);
        }
        Ok(())
    }

    async fn wait_for_port(port: u32, timeout: u32) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", port);
        for _ in 0..timeout {
            if TcpStream::connect(&addr).is_ok() {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        Err("Port timeout".into())
    }

    // мне лень убирать _config_path как аргумент, sorry
    async fn exec_tor_await(app: AppHandle, _config_path: &str) -> Result<(), String> {
        let base_dir = Self::tor_data_path(&app, "");
        let tor_exe = base_dir.join("tor/tor.exe");

        let _ = std::process::Command::new(tor_exe)
            .arg("-f")
            .arg("torrc")
            .creation_flags(0x08000000)
            .current_dir(&base_dir)
            .spawn();

        let app_inner = app.clone();
        let log_path = Self::tor_data_path(&app, TOR_LOGS);

        std::thread::spawn(move || {
            let re = Regex::new(r"Bootstrapped (\d+)%").unwrap();
            while !log_path.exists() {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            if let Ok(file) = fs::File::open(&log_path) {
                let mut reader = BufReader::new(file);
                let mut line = String::new();
                loop {
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        continue;
                    }
                    let text = line.trim();
                    app_inner.emit("log-event", format!("[TOR] {}", text)).ok();
                    if let Some(cap) = re.captures(text) {
                        if let Ok(p) = cap[1].parse::<u32>() {
                            app_inner.emit("tor-progress", p).ok();
                        }
                    }
                    line.clear();
                }
            }
        });
        Self::wait_for_port(9050, 60).await
    }
}
