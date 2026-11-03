//! Один экземпляр: второй запуск передаёт аргументы первому через 127.0.0.1 и выходит.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::time::Duration;

const PORT: u16 = 47_317;
const HELLO: &str = "ZUST-V3";

pub enum Instance {
    /// Мы первые: сюда будут приходить аргументы повторных запусков.
    Primary(TcpListener),
    /// Zust уже запущен и получил наши аргументы - можно выходить.
    Secondary,
    /// Порт занят чем-то другим: работаем без защиты от второго экземпляра.
    Unknown,
}

pub fn acquire() -> Instance {
    if let Ok(l) = TcpListener::bind((Ipv4Addr::LOCALHOST, PORT)) {
        return Instance::Primary(l);
    }
    if notify_running().unwrap_or(false) {
        Instance::Secondary
    } else {
        Instance::Unknown
    }
}

/// Передать аргументы уже запущенному Zust. true - он ответил.
fn notify_running() -> std::io::Result<bool> {
    let mut s = TcpStream::connect_timeout(
        &(Ipv4Addr::LOCALHOST, PORT).into(),
        Duration::from_millis(500),
    )?;
    s.set_read_timeout(Some(Duration::from_secs(2)))?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    writeln!(s, "{HELLO}\t{}", args.join("\t"))?;
    let mut answer = String::new();
    BufReader::new(s).read_line(&mut answer)?;
    Ok(answer.trim() == "OK")
}

/// Слушать повторные запуски в фоне; `on_args` вызывается с их аргументами.
pub fn listen(listener: TcpListener, on_args: impl Fn(Vec<String>) + Send + 'static) {
    std::thread::spawn(move || {
        for conn in listener.incoming().flatten() {
            let _ = conn.set_read_timeout(Some(Duration::from_secs(2)));
            let mut line = String::new();
            let mut reader = BufReader::new(&conn);
            // ссылки и аргументы короткие - ограничиваем строку, чтобы не читать мусор
            if reader.by_ref().take(4096).read_line(&mut line).is_err() {
                continue;
            }
            let mut parts = line.trim_end().split('\t');
            if parts.next() != Some(HELLO) {
                continue;
            }
            let args: Vec<String> = parts.filter(|a| !a.is_empty()).map(String::from).collect();
            let _ = (&conn).write_all(b"OK\n");
            on_args(args);
        }
    });
}
