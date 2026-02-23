use crate::utils::info;
use regex::Regex;
use std::{
    fs, io::BufRead, io::BufReader, net::TcpStream, os::windows::process::CommandExt,
    path::PathBuf, process::Command,
};
use tauri::{AppHandle, Emitter, Manager};

pub struct Tor;
const RELAYS_URL: &str = "https://torscan-ru.ntc.party/relays.txt";
const TOR_LOGS: &str = "latest.log";

impl Tor {
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
        let should_refresh = settings.refresh_bridges || !torrc_path.exists();

        let bridges = if should_refresh {
            info(&app, "[TOR] Получение мостов...");
            Self::get_bridges(app.clone()).await?
        } else {
            info(&app, "[TOR] Загрузка существующих мостов!");
            Self::get_current_bridges_from_file(&torrc_path)?
        };

        let config = Self::generate_config(app.clone(), bridges)?;
        info(&app, "[TOR] Конфиг успешно модифицирован.");
        Self::exec_tor_await(app.clone(), &config).await?;

        info(&app, "[TOR] ТОР ЗАПУЩЕН ^DD");
        Ok(())
    }

    pub async fn get_bridges(_app: AppHandle) -> Result<Vec<String>, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;

        let res = client
            .get(RELAYS_URL)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().is_success() {
            let text = res.text().await.map_err(|e| e.to_string())?;
            let lines: Vec<String> = text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();

            if !lines.is_empty() {
                return Ok(lines);
            }
        }
        Err("Список мостов пуст или недоступен".into())
    }

    pub fn generate_config(app: AppHandle, bridges: Vec<String>) -> Result<String, String> {
        let base_dir = Self::tor_data_path(&app, "");
        let data_dir = base_dir.join("data");
        let torrc_path = base_dir.join("torrc");
        let log_file = Self::tor_data_path(&app, TOR_LOGS);
        let mut content = format!(
            "SocksPort 127.0.0.1:9050\n\
      DataDirectory {data}\n\
      GeoIPFile {data}/geoip\n\
      GeoIPv6File {data}/geoip6\n\
      UseBridges 1\n\
      UpdateBridgesFromAuthority 0\n\
      SafeLogging 1\n\
      ClientOnly 1\n\
      AvoidDiskWrites 1\n\
      Log notice file {log}\n",
            data = data_dir.to_string_lossy().replace('\\', "/"),
            log = log_file.to_string_lossy().replace('\\', "/")
        );
        for b in bridges {
            content.push_str(&format!("Bridge {}\n", b));
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
                let _ = app.emit("tor-progress", 100);
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
        let file = fs::File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let bridges: Vec<String> = reader
            .lines()
            .filter_map(|l| l.ok())
            .filter(|l| l.starts_with("Bridge "))
            .map(|l| l.replacen("Bridge ", "", 1))
            .collect();
        if bridges.is_empty() {
            return Err("torrc empty".into());
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
        let target_dir = Self::tor_data_path(app, "tor");
        let target_bin = target_dir.join("tor.exe");

        if target_bin.exists() {
            return Ok(());
        }

        let mut source_dir = std::env::current_exe().map_err(|e| e.to_string())?;
        source_dir.pop();
        source_dir.push("tor");

        if source_dir.exists() && source_dir.is_dir() {
            fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
            fn copy_recursive(src: &PathBuf, dst_dir: &PathBuf) -> u32 {
                let mut count = 0;
                let targets = ["tor.exe", "geoip", "geoip6"];

                if let Ok(entries) = fs::read_dir(src) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            count += copy_recursive(&path, dst_dir);
                        } else if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            if targets.contains(&file_name) {
                                let dest = dst_dir.join(file_name);
                                if fs::copy(&path, &dest).is_ok() {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
                count
            }

            let copied = copy_recursive(&source_dir, &target_dir);

            if copied > 0 {
                info(app, &format!("[TOR] Успешно перенесено файлов: {}", copied));
                return Ok(());
            }
        }

        Err(format!("BINARY_NOT_FOUND ->> {:?}", source_dir))
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

    async fn exec_tor_await(app: AppHandle, config_path: &str) -> Result<(), String> {
        let bin = Self::tor_data_path(&app, "tor/tor.exe");
        Command::new(bin)
            .args(["-f", config_path])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;

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
