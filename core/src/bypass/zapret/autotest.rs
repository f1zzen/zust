//! Тест стратегий: по очереди запускаем каждую и проверяем, открываются ли Discord и YouTube.
//! Идея и набор проверок - как в test-strategies.sh из zapret-mac-discord-youtube (Flowseal).

use super::{Zapret, backend};
use crate::Hub;
use crate::logger::info;
use crate::platform::command;
use futures::future::join_all;
use serde::Serialize;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const TARGETS: [&str; 9] = [
    "discord.com",
    "gateway.discord.gg",
    "cdn.discordapp.com",
    "updates.discord.com",
    "www.youtube.com",
    "youtu.be",
    "i.ytimg.com",
    "redirector.googlevideo.com",
    "www.gstatic.com",
];

#[cfg(windows)]
const NULL: &str = "NUL";
#[cfg(not(windows))]
const NULL: &str = "/dev/null";

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Http,
    Tls12,
    Tls13,
}

struct Probe {
    ok: bool,
    connected: bool,
    secs: f64,
}

/// Один запрос curl к `host` строго на адрес `ip` (как `--resolve` у Flowseal).
async fn probe(kind: Kind, host: &str, ip: Ipv4Addr, max_time: u32) -> Probe {
    let (port, scheme) = if kind == Kind::Http {
        (80, "http")
    } else {
        (443, "https")
    };
    let mut cmd = command("curl");
    cmd.args(["--noproxy", "*", "-sS", "--http1.1", "-o", NULL])
        .args([
            "--connect-timeout",
            "2",
            "--max-time",
            &max_time.to_string(),
        ])
        .args(["--resolve", &format!("{host}:{port}:{ip}")])
        .args(["-w", "%{http_code} %{time_total} %{time_connect}"]);
    match kind {
        Kind::Tls12 => cmd.args(["--tlsv1.2", "--tls-max", "1.2"]),
        Kind::Tls13 => cmd.args(["--tlsv1.3", "--tls-max", "1.3"]),
        Kind::Http => &mut cmd,
    };
    cmd.arg(format!("{scheme}://{host}/"));
    let out = tokio::process::Command::from(cmd)
        .kill_on_drop(true)
        .output()
        .await;
    let Ok(out) = out else {
        return Probe {
            ok: false,
            connected: false,
            secs: 0.0,
        };
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut it = text.split_whitespace();
    let code = it.next().unwrap_or("000");
    let secs = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let connect: f64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    Probe {
        ok: out.status.success() && code != "000",
        connected: connect > 0.0,
        secs,
    }
}

/// Адреса из системного DNS и DoH (Cloudflare, Google): системный может быть подменён.
async fn resolve(client: &reqwest::Client, host: &str) -> Vec<Ipv4Addr> {
    let system = async {
        tokio::net::lookup_host((host, 443))
            .await
            .map(|it| {
                it.filter_map(|a| match a.ip() {
                    std::net::IpAddr::V4(v4) => Some(v4),
                    _ => None,
                })
                .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let doh = |url: String| async move {
        let Ok(res) = client
            .get(url)
            .header("accept", "application/dns-json")
            .send()
            .await
        else {
            return Vec::new();
        };
        let json: serde_json::Value = res.json().await.unwrap_or_default();
        json["Answer"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|r| r["type"] == 1)
                    .filter_map(|r| r["data"].as_str()?.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    };
    let (a, b, c) = tokio::join!(
        system,
        doh(format!(
            "https://cloudflare-dns.com/dns-query?name={host}&type=A"
        )),
        doh(format!("https://dns.google/resolve?name={host}&type=A")),
    );
    let mut ips: Vec<Ipv4Addr> = a.into_iter().chain(b).chain(c).collect();
    ips.sort();
    ips.dedup();
    ips
}

/// Для каждого адреса выбираем IP, до которого есть хотя бы TCP-соединение.
async fn discover(
    client: &reqwest::Client,
    host: &'static str,
) -> Option<(&'static str, Ipv4Addr)> {
    let ips = resolve(client, host).await;
    let probes = join_all(ips.iter().map(|ip| probe(Kind::Tls12, host, *ip, 3))).await;
    ips.into_iter()
        .zip(probes)
        .filter(|(_, p)| p.ok || p.connected)
        .min_by(|(_, a), (_, b)| {
            (!a.ok, a.secs)
                .partial_cmp(&(!b.ok, b.secs))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(ip, _)| (host, ip))
}

/// Результат одной стратегии.
#[derive(Clone, Debug, Serialize)]
pub struct TestRow {
    pub strategy: String,
    pub http: u32,
    pub tls12: u32,
    pub tls13: u32,
    /// Среднее время успешного запроса, мс.
    pub avg_ms: u32,
    /// Стратегия не запустилась.
    pub error: Option<String>,
}

impl TestRow {
    pub fn score(&self) -> u32 {
        self.http + self.tls12 + self.tls13
    }

    fn key(&self) -> (u32, u32, std::cmp::Reverse<u32>) {
        (
            self.score(),
            self.tls12 + self.tls13,
            std::cmp::Reverse(self.avg_ms),
        )
    }
}

#[derive(Debug, Serialize)]
pub struct TestReport {
    /// Лучшие сверху.
    pub rows: Vec<TestRow>,
    /// Сколько адресов удалось проверить (из `TARGETS`).
    pub targets: u32,
    /// Сеть поддерживает TLS 1.3 хотя бы для одной стратегии.
    pub tls13: bool,
    pub cancelled: bool,
}

impl TestReport {
    /// Максимум очков.
    pub fn total(&self) -> u32 {
        self.targets * if self.tls13 { 3 } else { 2 }
    }
}

async fn run_probes(name: &str, targets: &[(&str, Ipv4Addr)]) -> TestRow {
    let jobs = targets.iter().flat_map(|&(host, ip)| {
        [Kind::Http, Kind::Tls12, Kind::Tls13]
            .map(|k| async move { (k, probe(k, host, ip, 4).await) })
    });
    let results = join_all(jobs).await;
    let mut row = TestRow {
        strategy: name.to_string(),
        http: 0,
        tls12: 0,
        tls13: 0,
        avg_ms: 0,
        error: None,
    };
    let mut time = 0.0;
    for (kind, p) in &results {
        if !p.ok {
            continue;
        }
        time += p.secs;
        match kind {
            Kind::Http => row.http += 1,
            Kind::Tls12 => row.tls12 += 1,
            Kind::Tls13 => row.tls13 += 1,
        }
    }
    if row.score() > 0 {
        row.avg_ms = (time * 1000.0 / row.score() as f64) as u32;
    }
    row
}

pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub current: String,
    /// Сколько адресов проверяется (0 - ещё ищем).
    pub targets: usize,
    pub row: Option<TestRow>,
}

impl Zapret {
    /// Проверить стратегии `names` (файлы .zapret). После теста zapret возвращается
    /// в прежнее состояние. `progress` вызывается перед каждой стратегией и после неё.
    pub async fn test_strategies(
        app: &Hub,
        names: Vec<String>,
        ipset: String,
        cancel: Arc<AtomicBool>,
        progress: impl Fn(Progress) + Send + Sync,
    ) -> Result<TestReport, String> {
        let game_filter = crate::settings::load().game_filter;
        let restore = Self::current_strategy().map(|s| (s, Self::current_ipset()));
        Self::ensure_user_lists();
        backend::prepare(app).await?;

        let mut steps = Vec::new();
        for name in &names {
            steps.push(Self::load_strategy(name).map(|(file, raw)| Step {
                name: file,
                ipset: ipset.clone(),
                raw,
            }));
        }
        let restore_step = restore.and_then(|(s, i)| {
            let (file, raw) = Self::load_strategy(&s).ok()?;
            Some(Step {
                name: file,
                ipset: i.unwrap_or_else(|| ipset.clone()),
                raw,
            })
        });

        progress(Progress {
            done: 0,
            total: names.len(),
            current: "Поиск адресов".into(),
            targets: 0,
            row: None,
        });
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|e| e.to_string())?;
        let targets: Vec<(&str, Ipv4Addr)> = join_all(TARGETS.map(|h| discover(&client, h)))
            .await
            .into_iter()
            .flatten()
            .collect();
        info(
            app,
            &format!(
                "Тест стратегий: доступно адресов {} из {}",
                targets.len(),
                TARGETS.len()
            ),
        );
        if targets.is_empty() {
            return Err("Ни один адрес Discord и YouTube не отвечает. Проверьте интернет".into());
        }

        let ok_steps: Vec<&Step> = steps.iter().filter_map(|s| s.as_ref().ok()).collect();
        let mut switch = Switcher::open(app, &ok_steps, restore_step.as_ref(), game_filter).await?;

        let mut rows = Vec::new();
        let mut cancelled = false;
        let mut idx = 0;
        for (n, (name, step)) in names.iter().zip(&steps).enumerate() {
            if cancel.load(Ordering::SeqCst) {
                cancelled = true;
                break;
            }
            let shown = name.trim_end_matches(super::STRATEGY_EXT).to_string();
            progress(Progress {
                done: n,
                total: names.len(),
                current: shown.clone(),
                targets: targets.len(),
                row: None,
            });
            let row = match step {
                Err(e) => TestRow::failed(&shown, e.clone()),
                Ok(_) => {
                    let r = switch.switch(idx).await;
                    idx += 1;
                    match r {
                        Ok(()) => {
                            tokio::time::sleep(Duration::from_millis(700)).await;
                            run_probes(&shown, &targets).await
                        }
                        Err(e) => TestRow::failed(&shown, e),
                    }
                }
            };
            info(
                app,
                &format!(
                    "Тест: {shown} - {}",
                    row.error.clone().unwrap_or_else(|| format!(
                        "HTTP {}, TLS1.2 {}, TLS1.3 {} из {}",
                        row.http,
                        row.tls12,
                        row.tls13,
                        targets.len()
                    ))
                ),
            );
            rows.push(row.clone());
            progress(Progress {
                done: n + 1,
                total: names.len(),
                current: shown,
                targets: targets.len(),
                row: Some(row),
            });
        }

        progress(Progress {
            done: rows.len(),
            total: names.len(),
            current: "Восстановление".into(),
            targets: targets.len(),
            row: None,
        });
        switch.finish().await?;

        rows.sort_by_key(|r| std::cmp::Reverse(r.key()));
        Ok(TestReport {
            tls13: rows.iter().any(|r| r.tls13 > 0),
            targets: targets.len() as u32,
            rows,
            cancelled,
        })
    }
}

impl TestRow {
    fn failed(name: &str, error: String) -> Self {
        Self {
            strategy: name.to_string(),
            http: 0,
            tls12: 0,
            tls13: 0,
            avg_ms: 0,
            error: Some(error),
        }
    }
}

struct Step {
    name: String,
    ipset: String,
    raw: String,
}

// ---------- переключение стратегий ----------

/// Windows: Z.UST уже работает от администратора, просто перезапускаем службу.
#[cfg(windows)]
struct Switcher<'a> {
    app: Hub,
    steps: Vec<&'a Step>,
    restore: Option<&'a Step>,
    game_filter: bool,
}

#[cfg(windows)]
impl<'a> Switcher<'a> {
    async fn open(
        app: &Hub,
        steps: &[&'a Step],
        restore: Option<&'a Step>,
        game_filter: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            app: app.clone(),
            steps: steps.to_vec(),
            restore,
            game_filter,
        })
    }

    async fn start(&self, s: &Step) -> Result<(), String> {
        let (app, name, ipset, raw, gf) = (
            self.app.clone(),
            s.name.clone(),
            s.ipset.clone(),
            s.raw.clone(),
            self.game_filter,
        );
        tokio::task::spawn_blocking(move || backend::start(&app, &name, &ipset, raw.trim(), gf))
            .await
            .map_err(|e| e.to_string())??;
        for _ in 0..10 {
            if backend::is_running() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        Err("не запустилась".into())
    }

    async fn switch(&mut self, i: usize) -> Result<(), String> {
        self.start(self.steps[i]).await
    }

    async fn finish(self) -> Result<(), String> {
        match self.restore {
            Some(s) => self.start(s).await,
            None => {
                let app = self.app.clone();
                tokio::task::spawn_blocking(move || backend::stop(&app))
                    .await
                    .map_err(|e| e.to_string())?
            }
        }
    }
}

/// Linux и macOS: запуск требует пароля. Чтобы не спрашивать его на каждую стратегию,
/// один раз запускаем от root фоновый скрипт, который знает скрипты запуска всех стратегий
/// и выполняет их по номеру из файла-команды. Сам код он берёт только из своей копии.
#[cfg(unix)]
struct Switcher<'a> {
    cmd: std::path::PathBuf,
    seq: u32,
    steps: usize,
    restore: Option<&'a Step>,
}

/// Папка root для ответов скрипта: в папку пользователя root ничего не пишет.
#[cfg(unix)]
const RESULT_DIR: &str = "/var/run/zust-test";
/// Без команд дольше этого скрипт сам выключает zapret и выходит (Z.UST закрыли посреди теста).
#[cfg(unix)]
const IDLE_SECS: u32 = 120;

#[cfg(unix)]
impl<'a> Switcher<'a> {
    async fn open(
        app: &Hub,
        steps: &[&'a Step],
        restore: Option<&'a Step>,
        game_filter: bool,
    ) -> Result<Self, String> {
        use crate::platform::{run_privileged, shell_quote};
        let dir = Zapret::path("test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let cmd = dir.join("cmd");

        let mut arms = String::new();
        for (i, s) in steps.iter().chain(restore.as_slice()).enumerate() {
            let script =
                backend::start_script(app, &s.ipset, s.raw.trim(), game_filter, &format!("-t{i}"))?;
            arms.push_str(&format!("{i}) (\n{script}) ;;\n"));
        }
        let loop_script = loop_script(
            &shell_quote(&cmd.to_string_lossy()),
            &arms,
            &backend::stop_snippet(),
        );
        let r = RESULT_DIR;
        let loop_file = dir.join("loop.sh");
        std::fs::write(&loop_file, &loop_script).map_err(|e| e.to_string())?;
        // копия скрипта принадлежит root: после ввода пароля подменить его уже нельзя
        let launcher = format!(
            "rm -rf {r}\nmkdir -m 755 {r}\ncp {src} {r}/loop.sh\nchown root {r}/loop.sh\n\
             nohup /bin/sh {r}/loop.sh < /dev/null > /dev/null 2>&1 &\n",
            src = shell_quote(&loop_file.to_string_lossy()),
        );
        tokio::task::spawn_blocking(move || run_privileged(&launcher))
            .await
            .map_err(|e| e.to_string())??;
        Ok(Self {
            cmd,
            seq: 0,
            steps: steps.len(),
            restore,
        })
    }

    /// Отправить команду и дождаться ответа скрипта.
    async fn send(&mut self, c: &str) -> Result<(), String> {
        self.seq += 1;
        let tmp = self.cmd.with_extension("new");
        std::fs::write(&tmp, format!("{} {c}\n", self.seq)).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.cmd).map_err(|e| e.to_string())?;
        let res = std::path::Path::new(RESULT_DIR).join("res");
        for _ in 0..600 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let Ok(text) = std::fs::read_to_string(&res) else {
                continue;
            };
            let mut it = text.split_whitespace();
            if it.next() == Some(&self.seq.to_string()) {
                return match it.next() {
                    Some("ok") => Ok(()),
                    _ => Err(
                        std::fs::read_to_string(std::path::Path::new(RESULT_DIR).join("out"))
                            .ok()
                            .and_then(|o| o.lines().last().map(str::to_string))
                            .unwrap_or_else(|| "не запустилась".into()),
                    ),
                };
            }
        }
        Err("скрипт переключения не отвечает".into())
    }

    async fn switch(&mut self, i: usize) -> Result<(), String> {
        self.send(&i.to_string()).await
    }

    async fn finish(mut self) -> Result<(), String> {
        let r = match self.restore {
            // шаг восстановления - последний в списке скрипта
            Some(s) => {
                let i = self.steps;
                let r = self.send(&i.to_string()).await;
                if r.is_ok() {
                    backend::save_state(&s.name, &s.ipset);
                }
                r
            }
            None => self.send("off").await,
        };
        let _ = std::fs::write(&self.cmd, format!("{} quit\n", self.seq + 1));
        // скрипты шагов (run-t0.sh, zust-t0.nft...) больше не нужны
        // скрипт удаляет RESULT_DIR при выходе; пока он не прочитал quit, cmd трогать нельзя,
        // иначе через IDLE_SECS он сам выключит zapret
        for _ in 0..50 {
            if !std::path::Path::new(RESULT_DIR).exists() {
                let _ = std::fs::remove_dir_all(Zapret::path("test"));
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        for f in std::fs::read_dir(Zapret::path(""))
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = f.file_name().to_string_lossy().into_owned();
            if name.starts_with("run-t") || name.starts_with("zust-t") {
                let _ = std::fs::remove_file(f.path());
            }
        }
        r
    }
}

/// Фоновый скрипт: ждёт в файле `cmd` строку "<номер> <шаг>", выполняет шаг (`arms` - ветки case)
/// и пишет "<номер> ok|fail" в RESULT_DIR/res. Всё внутри функции: sh разбирает её целиком
/// до выполнения, так что дописать что-то в файл на ходу бесполезно.
#[cfg(unix)]
fn loop_script(cmd: &str, arms: &str, stop: &str) -> String {
    let r = RESULT_DIR;
    let idle = IDLE_SECS * 10;
    format!(
        r#"main() {{
CMD={cmd}
step() {{
case "$1" in
{arms}off) (
{stop}) ;;
*) return 1 ;;
esac
}}
N=0
while [ $N -lt {idle} ]; do
  if [ -f "$CMD" ]; then
    read -r S C < "$CMD" || true
    rm -f "$CMD"
    N=0
    [ "$C" = quit ] && return 0
    # не в if: там set -e внутри шага не сработал бы
    step "$C" > {r}/out 2>&1
    if [ $? -eq 0 ]; then R=ok; else R=fail; fi
    echo "$S $R" > {r}/res.new && mv -f {r}/res.new {r}/res
  else
    sleep 0.1
    N=$((N + 1))
  fi
done
step off
}}
main > /dev/null 2>&1; rm -rf {r}; exit
"#
    )
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn loop_script_parses() {
        let arms = "0) (\nset -e\necho 'a (b)'\nfalse\necho never\n) ;;\n";
        let s = loop_script("'/tmp/x y/cmd'", arms, "echo stop\n");
        let out = std::process::Command::new("sh")
            .args(["-n", "-c", &s])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[cfg(test)]
mod net_tests {
    use super::*;

    /// Сеть: cargo test -- --ignored probes_live --nocapture
    #[tokio::test]
    #[ignore]
    async fn probes_live() {
        let client = reqwest::Client::new();
        let targets: Vec<_> = join_all(TARGETS.map(|h| discover(&client, h)))
            .await
            .into_iter()
            .flatten()
            .collect();
        println!("{targets:?}");
        let row = run_probes("без zapret", &targets).await;
        println!("{row:?}");
        assert!(!targets.is_empty());
    }
}
