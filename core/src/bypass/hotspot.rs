//! "Точка доступа": пробрасывает локальный SOCKS TOR-а наружу через публичный frp-сервер,
//! чтобы прокси можно было использовать с телефона.

use crate::Hub;
use crate::platform::{self, PROXY_PORT, PidFile};
use rand::Rng;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub const FRP_SERVER: &str = "frp.freefrp.net";
const ATTEMPTS: usize = 15;
const WAIT: Duration = Duration::from_secs(20);

pub struct HotSpot;

impl HotSpot {
    fn pid_file() -> PathBuf {
        crate::paths::data_dir().join("frpc.pid")
    }

    fn binary() -> Result<PathBuf, String> {
        #[cfg(windows)]
        {
            let p = crate::paths::resource_dir().join("frp").join("frpc.exe");
            p.is_file()
                .then_some(p)
                .ok_or_else(|| "Не найден frpc.exe".to_string())
        }
        #[cfg(not(windows))]
        {
            #[cfg(target_os = "macos")]
            let hint = "Не найден frpc. Установите его через Homebrew: brew install frpc";
            #[cfg(not(target_os = "macos"))]
            let hint = "Не найден frpc. Установите frp (пакет frp/frpc).";
            platform::find_in_path("frpc").ok_or_else(|| hint.to_string())
        }
    }

    /// Блокирующая функция - вызывать из spawn_blocking.
    pub fn run(app: &Hub, manual_port: Option<u16>) -> Result<u16, String> {
        Self::stop();
        let bin = Self::binary()?;
        let config = crate::paths::data_dir().join("frpc.toml");
        let mut rng = rand::thread_rng();
        let mut port = manual_port.unwrap_or_else(|| rng.gen_range(20000..60000));

        for _ in 0..ATTEMPTS {
            let id = &uuid::Uuid::new_v4().to_string()[..8];
            let toml = format!(
                "serverAddr = \"{FRP_SERVER}\"\nserverPort = 7000\n\n\
                 auth.method = \"token\"\nauth.token = \"freefrp.net\"\n\n\
                 [[proxies]]\nname = \"zust_4ever_{id}\"\ntype = \"tcp\"\n\
                 localIP = \"127.0.0.1\"\nlocalPort = {PROXY_PORT}\nremotePort = {port}\n"
            );
            fs::write(&config, toml).map_err(|e| e.to_string())?;

            let mut child = platform::command(&bin)
                .arg("-c")
                .arg(&config)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| e.to_string())?;

            // читаем вывод в отдельном потоке, чтобы можно было ждать с таймаутом
            let (tx, rx) = mpsc::channel::<String>();
            let stdout = child.stdout.take().unwrap();
            let app2 = app.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    let _ = app2.emit("log-event", format!("[FRP] {line}"));
                    let _ = tx.send(line); // после успешного запуска получателя нет - это нормально
                }
            });

            // цикл заканчивается break только если порт занят - пробуем следующий
            let started = Instant::now();
            loop {
                let left = WAIT.saturating_sub(started.elapsed());
                match rx.recv_timeout(left) {
                    Ok(line) if line.contains("start proxy success") => {
                        PidFile::write(&Self::pid_file(), child.id(), &port.to_string());
                        return Ok(port);
                    }
                    Ok(line)
                        if line.contains("already used")
                            || line.contains("port not allowed")
                            || line.contains("port unavailable") =>
                    {
                        break;
                    }
                    Ok(_) => continue,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let _ = child.kill();
                        return Err("frp-сервер не ответил за 20 секунд".into());
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let _ = child.kill();
                        return Err("frpc неожиданно завершился".into());
                    }
                }
            }
            let _ = child.kill();
            let _ = child.wait();
            port = rng.gen_range(20000..60000);
            let _ = app.emit("log-event", format!("[FRP] Порт занят, пробую {port}..."));
        }
        Err("Не удалось подобрать свободный порт".into())
    }

    pub fn stop() {
        PidFile::kill(&Self::pid_file(), "frpc");
    }

    /// Порт, если точка доступа запущена.
    pub fn status() -> Option<u16> {
        PidFile::alive(&Self::pid_file(), "frpc").and_then(|(_, port)| port.parse().ok())
    }
}
