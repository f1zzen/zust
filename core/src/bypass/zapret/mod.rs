//! zapret: общая часть для всех ОС - стратегии, списки, сборка аргументов.
//! Запуск/остановка движка - в `windows.rs` (winws + служба) `linux.rs` (nfqws + nftables) и `macos.rs` (utunws + pf).

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as backend;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use self::linux as backend;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use self::macos as backend;

mod autotest;
mod policy;
mod user_lists;
pub use autotest::{Progress, TestReport, TestRow};
pub use user_lists::UserList;

use crate::Hub;
use crate::logger::info;
use crate::paths::{self, sanitize_file_name};
use futures::future::join_all;
use std::fs;
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};

pub const STRATEGY_EXT: &str = ".zapret";
const STRATEGIES_REPO: &str =
    "https://raw.githubusercontent.com/f1zzen/zust_strategies/refs/heads/main/";

#[cfg(windows)]
const DIR_NAME: &str = "zapret-winws";
#[cfg(not(windows))]
const DIR_NAME: &str = "zapret";

/// Пользовательские списки (как у flowseal). Если их нет - создаём с "заглушкой",
/// потому что пустой hostlist/ipset в zapret означает "подходит всё".
const USER_LISTS: [(&str, &str); 3] = [
    ("list-general-user.txt", "domain.example.abc\n"),
    ("list-exclude-user.txt", "domain.example.abc\n"),
    ("ipset-exclude-user.txt", "203.0.113.113/32\n"),
];

/// Куда указывают %BIN%, %LISTS% и пользовательские ipset-ы в аргументах.
pub struct ZapretDirs {
    pub bin: PathBuf,
    pub lists: PathBuf,
    pub ipset_configs: PathBuf,
}

/// Готовые аргументы движка + порты, которые надо перехватывать.
#[derive(Debug, PartialEq)]
pub struct Launch {
    pub args: Vec<String>,
    pub tcp_ports: String,
    pub udp_ports: String,
}

pub struct Zapret;

impl Zapret {
    /// Рабочая папка zapret в данных приложения.
    pub fn path(sub: &str) -> PathBuf {
        let dir = paths::data_dir().join(DIR_NAME);
        let _ = fs::create_dir_all(&dir);
        dir.join(sub)
    }

    /// Исходные файлы zapret, которые поставляются с приложением.
    fn storage(sub: &str) -> PathBuf {
        paths::resource_dir().join("zapret").join(sub)
    }

    pub fn real_dirs() -> ZapretDirs {
        ZapretDirs {
            bin: Self::path("bin"),
            lists: Self::path("lists"),
            ipset_configs: Self::path("ipset-configs"),
        }
    }

    // ---------- состояние ----------

    pub fn is_running() -> bool {
        backend::is_running()
    }

    /// Имя запущенной стратегии (None, если zapret не работает).
    pub fn current_strategy() -> Option<String> {
        backend::is_running()
            .then(backend::current_strategy)
            .flatten()
    }

    pub fn current_ipset() -> Option<String> {
        backend::current_ipset()
    }

    // ---------- запуск ----------

    pub async fn start(app: &Hub, name: &str, ipset: Option<String>) -> Result<bool, String> {
        let (name, raw) = Self::load_strategy(name)?;
        let ipset = ipset.unwrap_or_else(|| "ipset-all.txt".into());
        let game_filter = crate::settings::load().game_filter;

        Self::ensure_user_lists();
        backend::prepare(app).await?;

        let app2 = app.clone();
        let (name2, ipset2) = (name.clone(), ipset.clone());
        tokio::task::spawn_blocking(move || {
            backend::start(&app2, &name2, &ipset2, raw.trim(), game_filter)
        })
        .await
        .map_err(|e| e.to_string())??;

        // даём движку пару секунд подняться
        for _ in 0..10 {
            if backend::is_running() {
                info(app, &format!("zapret запущен: {name} (ipset: {ipset})"));
                return Ok(true);
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
        info(
            app,
            "zapret не перешёл в рабочее состояние - смотрите лог движка",
        );
        Ok(false)
    }

    pub async fn stop(app: &Hub) -> Result<(), String> {
        let app2 = app.clone();
        tokio::task::spawn_blocking(move || backend::stop(&app2))
            .await
            .map_err(|e| e.to_string())?
    }

    /// Имя файла стратегии и её текст.
    fn load_strategy(name: &str) -> Result<(String, String), String> {
        let name = Self::strategy_file_name(name)?;
        let raw = fs::read_to_string(Self::path("strategies").join(&name))
            .map_err(|_| format!("Стратегия не найдена: {name}"))?;
        Ok((name, raw))
    }

    fn strategy_file_name(name: &str) -> Result<String, String> {
        let clean = sanitize_file_name(name).ok_or("Некорректное имя стратегии")?;
        Ok(if clean.ends_with(STRATEGY_EXT) {
            clean
        } else {
            format!("{clean}{STRATEGY_EXT}")
        })
    }

    // ---------- сборка аргументов ----------

    /// Текст .zapret → аргументы движка: переменные, склейка строк с `^`, кавычки.
    pub fn build_launch(
        raw: &str,
        dirs: &ZapretDirs,
        ipset: &str,
        game_filter: bool,
    ) -> Result<Launch, String> {
        let filter = if game_filter { "1024-65535" } else { "12" };
        let with_sep = |p: &Path| format!("{}{}", p.display(), MAIN_SEPARATOR_STR);

        let ipset_path = match ipset.to_lowercase().as_str() {
            "none" => dirs.lists.join("ipset-none-hide.txt"),
            "any" => dirs.lists.join("ipset-any-hide.txt"),
            _ => match sanitize_file_name(ipset) {
                Some(file) if Self::path("ipset-configs").join(&file).exists() => {
                    dirs.ipset_configs.join(file)
                }
                Some(file) => dirs.lists.join(file),
                None => dirs.lists.join("ipset-all.txt"),
            },
        };
        let ipset_arg = format!("--ipset={}", ipset_path.display());

        let text = raw
            .replace("%GameFilterTCP%", filter)
            .replace("%GameFilterUDP%", filter)
            .replace("%GameFilter%", filter)
            // `^!` - экранирование `!` в .bat (раньше его снимал cmd.exe)
            .replace("^!", "!");
        let tokens = split_args(
            &text
                .lines()
                .map(|l| l.trim().trim_end_matches('^').trim())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        );

        // проверяем исходный текст: пути пока ещё в виде %BIN%/%LISTS%
        policy::check(&tokens)?;

        let has_lists = tokens.iter().any(|t| t.contains("%LISTS%"));
        let (bin, lists) = (with_sep(&dirs.bin), with_sep(&dirs.lists));
        let mut args: Vec<String> = tokens
            .into_iter()
            .map(|t| {
                if t == "%IPSET%" || t == "--ipset=%LISTS%ipset-all.txt" {
                    ipset_arg.clone()
                } else {
                    t.replace("%BIN%", &bin).replace("%LISTS%", &lists)
                }
            })
            .collect();

        // старые стратегии без %LISTS%: подключаем все list-*.txt, кроме исключений
        if !has_lists {
            for name in list_files(&Self::path("lists"), ".txt") {
                if name.starts_with("list-") && !name.contains("exclude") {
                    args.push(format!("--hostlist={}", dirs.lists.join(name).display()));
                }
            }
        }

        let port_arg = |prefix: &str| {
            args.iter()
                .find_map(|a| a.strip_prefix(prefix))
                .map(str::to_string)
                .unwrap_or_default()
        };
        Ok(Launch {
            tcp_ports: port_arg("--wf-tcp="),
            udp_ports: port_arg("--wf-udp="),
            args,
        })
    }

    /// Проверка стратегии без запуска (для импорта и обновлений).
    pub fn validate_strategy(raw: &str) -> Result<(), String> {
        Self::build_launch(raw, &Self::real_dirs(), "ipset-all.txt", false).map(|_| ())
    }

    // ---------- файлы ----------

    fn ensure_user_lists() {
        let lists = Self::path("lists");
        let _ = fs::create_dir_all(&lists);
        for (name, placeholder) in USER_LISTS {
            let p = lists.join(name);
            if !p.exists() {
                let _ = fs::write(p, placeholder);
            }
        }
    }

    /// Первый запуск - копируем всё из ресурсов; дальше - только недостающие файлы,
    /// чтобы не затирать правки пользователя.
    pub fn sync_files(app: &Hub) -> Result<(), String> {
        let src = Self::storage("");
        let dst = Self::path("");
        if !src.is_dir() {
            return Err(format!("нет папки с ресурсами: {}", src.display()));
        }
        let has_strategies = !list_files(&dst.join("strategies"), STRATEGY_EXT).is_empty();
        if !has_strategies {
            info(app, "Первичная настройка ресурсов zapret...");
        }
        copy_dir(app, &src, &dst, has_strategies).map_err(|e| e.to_string())?;
        Self::ensure_user_lists();
        Ok(())
    }

    pub async fn restore_files(app: &Hub) -> Result<String, String> {
        Self::stop(app).await?;
        let src = Self::storage("");
        let dst = Self::path("");
        if !src.exists() {
            return Err(format!("Хранилище не найдено: {}", src.display()));
        }
        fs::remove_dir_all(&dst).map_err(|e| format!("Не удалось очистить рабочую папку: {e}"))?;
        copy_dir(app, &src, &dst, false).map_err(|e| format!("Ошибка копирования: {e}"))?;
        Self::ensure_user_lists();
        Ok("Файлы zapret восстановлены".into())
    }

    pub fn list_strategies() -> Vec<String> {
        list_files(&Self::path("strategies"), STRATEGY_EXT)
    }

    pub fn custom_ipset_files() -> Vec<String> {
        let path = Self::path("ipset-configs");
        let _ = fs::create_dir_all(&path);
        list_files(&path, ".txt")
    }

    pub fn domain_in_list(list: &str, domain: &str) -> bool {
        fs::read_to_string(Self::path("lists").join(list))
            .map(|c| c.lines().any(|l| l.trim() == domain))
            .unwrap_or(false)
    }

    pub fn add_domain_to_list(list: &str, domain: &str) -> Result<(), String> {
        if Self::domain_in_list(list, domain) {
            return Ok(());
        }
        let path = Self::path("lists").join(list);
        let mut content = fs::read_to_string(&path).unwrap_or_default();
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(domain);
        content.push('\n');
        fs::write(path, content).map_err(|e| e.to_string())
    }

    /// .bat от flowseal → .zapret (всё, что после `winws.exe"`).
    pub fn convert_bats(paths: Vec<String>) -> Result<usize, String> {
        let dir = Self::path("strategies");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut converted = 0;
        for p in paths {
            let path = Path::new(&p);
            let content =
                fs::read_to_string(path).map_err(|e| format!("Ошибка чтения {p}: {e}"))?;
            let Some(idx) = content.find("winws.exe\"") else {
                continue;
            };
            let args = content[idx + "winws.exe\"".len()..].trim();
            Self::validate_strategy(args).map_err(|e| format!("{p}: {e}"))?;
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let Some(stem) = sanitize_file_name(&stem) else {
                continue;
            };
            fs::write(dir.join(format!("{stem}{STRATEGY_EXT}")), args)
                .map_err(|e| e.to_string())?;
            converted += 1;
        }
        Ok(converted)
    }

    // ---------- обновления ----------

    pub async fn check_strategy_updates() -> Result<Vec<String>, String> {
        let client = reqwest::Client::new();
        let list = fetch_text(&client, &format!("{STRATEGIES_REPO}list")).await?;
        let dir = Self::path("strategies");

        // все стратегии скачиваются параллельно, а не по очереди
        let checks = list
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|name| {
                let client = client.clone();
                let dir = dir.clone();
                let name = name.to_string();
                async move {
                    let remote = fetch_text(&client, &format!("{STRATEGIES_REPO}{name}"))
                        .await
                        .ok()?;
                    // стратегии, не прошедшие белый список, даже не предлагаем
                    Self::validate_strategy(remote.trim()).ok()?;
                    let local = fs::read_to_string(dir.join(&name)).unwrap_or_default();
                    (local.trim() != remote.trim()).then_some(name)
                }
            });
        Ok(join_all(checks).await.into_iter().flatten().collect())
    }

    pub async fn apply_strategy_update(file_name: &str) -> Result<(), String> {
        let name = Self::strategy_file_name(file_name)?;
        let content =
            fetch_text(&reqwest::Client::new(), &format!("{STRATEGIES_REPO}{name}")).await?;
        Self::validate_strategy(content.trim())?;
        let dir = Self::path("strategies");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        fs::write(dir.join(name), content.trim()).map_err(|e| e.to_string())
    }

    /// Обновляет .bin-файлы (fake ClientHello и т.п.) по списку из репозитория стратегий.
    pub async fn update_tls_bin(app: &Hub) -> Result<String, String> {
        let client = reqwest::Client::new();
        let bin_dir = Self::path("bin");
        fs::create_dir_all(&bin_dir).map_err(|e| e.to_string())?;
        let list = fetch_text(&client, &format!("{STRATEGIES_REPO}list_bin")).await?;

        let mut updated = 0;
        for url in list.lines().map(str::trim).filter(|l| !l.is_empty()) {
            // только https с GitHub и только безопасные имена *.bin
            if !url.starts_with("https://github.com/") && !url.starts_with(STRATEGIES_REPO) {
                continue;
            }
            let Some(name) = url.rsplit('/').next().filter(|n| bin_name_ok(n)) else {
                continue;
            };
            let Ok(res) = client.get(url).send().await else {
                continue;
            };
            if !res.status().is_success() {
                continue;
            }
            let bytes = res.bytes().await.map_err(|e| e.to_string())?;
            if bytes.len() > MAX_BIN_SIZE {
                info(
                    app,
                    &format!("Пропущен {name}: слишком большой для fake-пакета"),
                );
                continue;
            }
            let target = bin_dir.join(name);
            if fs::read(&target).ok().as_deref() != Some(&bytes[..]) {
                fs::write(&target, &bytes).map_err(|e| format!("{name}: {e}"))?;
                info(app, &format!("Обновлён бинарник: {name}"));
                updated += 1;
            }
        }
        Ok(format!("bin-файлы проверены, обновлено: {updated}"))
    }

    /// TOR Project заблокирован: добавляем домен в list-general.txt и перезапускаем zapret.
    pub async fn repair_for_tor(app: &Hub, strategy: &str) -> Result<(), String> {
        let domain = "torproject.org";
        if !Self::domain_in_list("list-general.txt", domain) {
            info(app, &format!("Добавление {domain} в list-general.txt"));
            Self::add_domain_to_list("list-general.txt", domain)?;
        }
        let strategy = if strategy.is_empty() {
            Self::list_strategies()
                .into_iter()
                .next()
                .ok_or("Нет ни одной стратегии")?
        } else {
            strategy.to_string()
        };
        let ipset = Self::current_ipset();
        if Self::start(app, &strategy, ipset).await? {
            Ok(())
        } else {
            Err("Не удалось запустить zapret".into())
        }
    }
}

// ---------- вспомогательное ----------

/// Fake-пакеты - сотни байт; ограничение не даёт подсунуть что-то крупное.
const MAX_BIN_SIZE: usize = 64 * 1024;

/// Имя .bin-файла из репозитория стратегий: латиница, цифры, `._-`, расширение .bin.
fn bin_name_ok(name: &str) -> bool {
    name.len() <= 100
        && name.ends_with(".bin")
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    let res = client.get(url).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("{url}: {}", res.status()));
    }
    res.text().await.map_err(|e| e.to_string())
}

/// Имена файлов с расширением `ext`, отсортированные.
pub fn list_files(dir: &Path, ext: &str) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .map(|it| {
            it.filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(ext))
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|s| s.to_lowercase());
    v
}

/// Копирует папку. `only_missing` - не перезаписывать уже существующие файлы.
/// Занятые файлы (например, работающий winws.exe) пропускаются.
fn copy_dir(app: &Hub, src: &Path, dst: &Path, only_missing: bool) -> std::io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(app, &entry.path(), &target, only_missing)?;
        } else if !(only_missing && target.exists()) {
            if only_missing {
                info(
                    app,
                    &format!(
                        "Добавлен новый файл: {}",
                        entry.file_name().to_string_lossy()
                    ),
                );
            }
            if let Err(e) = fs::copy(entry.path(), &target) {
                info(app, &format!("Пропущен {}: {e}", target.display()));
            }
        }
    }
    Ok(())
}

/// Разбивает командную строку на аргументы. Кавычки группируют и удаляются:
/// `--hostlist="C:\My Dir\a.txt" --new` → [`--hostlist=C:\My Dir\a.txt`, `--new`]
pub fn split_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut has_token = false;
    for c in s.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                has_token = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if has_token {
                    args.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            c => {
                cur.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        args.push(cur);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_args_handles_quotes() {
        assert_eq!(
            split_args(r#"--wf-tcp=80,443 --hostlist="C:\My Dir\a.txt"  --new"#),
            vec!["--wf-tcp=80,443", r"--hostlist=C:\My Dir\a.txt", "--new"]
        );
        assert_eq!(split_args(""), Vec::<String>::new());
    }

    #[test]
    fn build_launch_substitutes_vars() {
        let dirs = ZapretDirs {
            bin: PathBuf::from("/run/zust/bin"),
            lists: PathBuf::from("/run/zust/lists"),
            ipset_configs: PathBuf::from("/run/zust/ipset-configs"),
        };
        let raw = "--wf-tcp=80,443,%GameFilterTCP% --wf-udp=443,%GameFilterUDP% ^\n\
                   --filter-tcp=443 --hostlist=\"%LISTS%list-general.txt\" --dpi-desync-fake-quic=\"%BIN%quic.bin\" --new ^\n\
                   --filter-udp=443 --ipset=\"%LISTS%ipset-all.txt\"";
        let l = Zapret::build_launch(raw, &dirs, "none", false).unwrap();
        assert_eq!(l.tcp_ports, "80,443,12");
        assert_eq!(l.udp_ports, "443,12");
        let sep = MAIN_SEPARATOR_STR;
        assert!(
            l.args
                .contains(&format!("--hostlist=/run/zust/lists{sep}list-general.txt"))
        );
        assert!(l.args.contains(&format!(
            "--dpi-desync-fake-quic=/run/zust/bin{sep}quic.bin"
        )));
        assert!(
            l.args
                .iter()
                .any(|a| a.starts_with("--ipset=") && a.ends_with("ipset-none-hide.txt"))
        );
        assert!(!l.args.iter().any(|a| a.contains('^') || a.contains('%')));

        let l = Zapret::build_launch(raw, &dirs, "any", true).unwrap();
        assert_eq!(l.tcp_ports, "80,443,1024-65535");
        assert!(l.args.iter().any(|a| a.ends_with("ipset-any-hide.txt")));
    }

    /// Все стратегии, которые поставляются с Zust, должны проходить белый список.
    /// ZUST_CHECK_STRATEGIES=<папка> - дополнительно проверить другую папку (например, клон zust_strategies).
    #[test]
    fn bundled_strategies_pass_policy() {
        let dirs = ZapretDirs {
            bin: PathBuf::from("/b"),
            lists: PathBuf::from("/l"),
            ipset_configs: PathBuf::from("/i"),
        };
        let mut folders =
            vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src-tauri/zapret/strategies")];
        if let Some(extra) = std::env::var_os("ZUST_CHECK_STRATEGIES") {
            folders.push(PathBuf::from(extra));
        }
        let mut checked = 0;
        for dir in folders {
            for name in list_files(&dir, STRATEGY_EXT) {
                let raw = fs::read_to_string(dir.join(&name)).unwrap();
                if let Err(e) = Zapret::build_launch(raw.trim(), &dirs, "none", false) {
                    panic!("{name}: {e}");
                }
                checked += 1;
            }
        }
        assert!(checked > 0);
    }
}
