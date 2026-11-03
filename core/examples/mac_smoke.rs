//! Проверка zapret на macOS в CI: sudo ./mac_smoke <папка src-tauri>
//! Запуск, перехват pf, сайты открываются через движок, остановка.

#[cfg(target_os = "macos")]
fn main() {
    use std::process::Command;
    use zust_core::Hub;
    use zust_core::bypass::zapret::Zapret;

    let resources = std::env::args().nth(1).expect("путь к src-tauri");
    let data = std::env::temp_dir().join("zust-smoke");
    zust_core::paths::init(data, resources.into());
    let hub = Hub::new(|event, payload| println!("[{event}] {payload}"));
    Zapret::sync_files(&hub).expect("sync_files");

    let pf_rules = || {
        let out = Command::new("/sbin/pfctl")
            .args(["-a", "com.apple/zust", "-s", "rules"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).to_string()
    };
    // curl от обычного пользователя: трафик root правило pf не перехватывает
    let user = std::env::var("SUDO_USER").expect("запускать через sudo");
    let fetch = |url: &str| {
        Command::new("sudo")
            .args([
                "-u",
                &user,
                "curl",
                "-sS",
                "-o",
                "/dev/null",
                "-m",
                "20",
                url,
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let started = rt
        .block_on(Zapret::start(&hub, "general (ALT)", None))
        .expect("start");
    assert!(started, "utunws не перешёл в рабочее состояние");
    assert!(Zapret::is_running());
    let rules = pf_rules();
    println!("pf:\n{rules}");
    assert!(rules.contains("route-to"), "нет правил перехвата");

    let mut ok = true;
    for url in [
        "https://www.google.com",
        "https://discord.com",
        "https://github.com",
    ] {
        let r = fetch(url);
        println!("{url}: {}", if r { "ok" } else { "НЕ ОТКРЫЛСЯ" });
        ok &= r;
    }
    let log = std::fs::read_to_string("/Library/Application Support/Z.UST/engine.log")
        .unwrap_or_default();
    println!("engine.log:\n{log}");

    rt.block_on(Zapret::stop(&hub)).expect("stop");
    std::thread::sleep(std::time::Duration::from_secs(1));
    assert!(!Zapret::is_running(), "utunws не остановился");
    assert!(!pf_rules().contains("route-to"), "правила pf остались");
    assert!(ok, "сайты не открываются, пока работает zapret");
    println!("OK");
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("только для macOS");
}
