//! Белый список опций winws/nfqws: стратегии приходят из сети, а движок работает от root/SYSTEM.
//! Запрещено всё, что пишет файлы, меняет процесс, читает пути не через %BIN%/%LISTS%
//! (иначе `--dpi-desync-fake-tls=@<файл>` отправил бы файл в сеть) или задаёт свой фильтр (--wf-raw).

/// Опции без значения или со "скалярным" значением (числа, порты, режимы, домены).
const PLAIN: &[&str] = &[
    // общие
    "new",
    "skip",
    "comment",
    "bind-fix4",
    "bind-fix6",
    "ctrack-timeouts",
    "ctrack-disable",
    "ipcache-lifetime",
    "ipcache-hostname",
    // фильтры профиля
    "filter-l3",
    "filter-tcp",
    "filter-udp",
    "filter-l7",
    "filter-ssid",
    "hostlist-domains",
    "hostlist-exclude-domains",
    "ipset-ip",
    "ipset-exclude-ip",
    // windivert (на Linux отбрасываются, порты переносятся в nftables)
    "wf-iface",
    "wf-l3",
    "wf-tcp",
    "wf-udp",
    "wf-filter-lan",
    "ssid-filter",
    "nlm-filter",
    // модификация пакетов
    "wsize",
    "wssize",
    "wssize-cutoff",
    "wssize-forced-cutoff",
    "synack-split",
    "orig-ttl",
    "orig-ttl6",
    "orig-autottl",
    "orig-autottl6",
    "orig-tcp-flags-set",
    "orig-tcp-flags-unset",
    "orig-mod-start",
    "orig-mod-cutoff",
    "dup",
    "dup-replace",
    "dup-ttl",
    "dup-ttl6",
    "dup-autottl",
    "dup-autottl6",
    "dup-tcp-flags-set",
    "dup-tcp-flags-unset",
    "dup-fooling",
    "dup-ts-increment",
    "dup-badseq-increment",
    "dup-badack-increment",
    "dup-ip-id",
    "dup-start",
    "dup-cutoff",
    "hostcase",
    "hostnospace",
    "methodeol",
    "hostspell",
    "domcase",
    "ip-id",
    // десинхронизация
    "dpi-desync",
    "dpi-desync-ttl",
    "dpi-desync-ttl6",
    "dpi-desync-autottl",
    "dpi-desync-autottl6",
    "dpi-desync-tcp-flags-set",
    "dpi-desync-tcp-flags-unset",
    "dpi-desync-fooling",
    "dpi-desync-repeats",
    "dpi-desync-skip-nosni",
    "dpi-desync-split-pos",
    "dpi-desync-split-seqovl",
    "dpi-desync-fakedsplit-mod",
    "dpi-desync-hostfakesplit-midhost",
    "dpi-desync-hostfakesplit-mod",
    "dpi-desync-ipfrag-pos-tcp",
    "dpi-desync-ipfrag-pos-udp",
    "dpi-desync-ts-increment",
    "dpi-desync-badseq-increment",
    "dpi-desync-badack-increment",
    "dpi-desync-any-protocol",
    "dpi-desync-fake-tcp-mod",
    "dpi-desync-fake-tls-mod",
    "dpi-desync-udplen-increment",
    "dpi-desync-start",
    "dpi-desync-cutoff",
];

/// Содержимое фейков: `0xHEX` или файл из %BIN% (`[+ofs]@%BIN%x.bin` / `%BIN%x.bin`).
const PAYLOAD: &[&str] = &[
    "dpi-desync-split-seqovl-pattern",
    "dpi-desync-fakedsplit-pattern",
    "dpi-desync-fake-http",
    "dpi-desync-fake-tls",
    "dpi-desync-fake-unknown",
    "dpi-desync-fake-syndata",
    "dpi-desync-fake-quic",
    "dpi-desync-fake-wireguard",
    "dpi-desync-fake-dht",
    "dpi-desync-fake-discord",
    "dpi-desync-fake-stun",
    "dpi-desync-fake-unknown-udp",
    "dpi-desync-udplen-pattern",
];

/// Списки доменов и IP: только файлы из %LISTS%.
const LIST: &[&str] = &["hostlist", "hostlist-exclude", "ipset", "ipset-exclude"];

/// Символы, допустимые в обычных значениях: порты (80,443,1024-65535,~,*),
/// маркеры (host+1, midsld), autottl (-1:3-20), моды (sni=x.com), домены (#коммент).
fn plain_value_ok(v: &str) -> bool {
    v.chars()
        .all(|c| c.is_ascii_alphanumeric() || "._,:~*+-#!=".contains(c))
}

fn hex_ok(v: &str) -> bool {
    v.strip_prefix("0x")
        .or_else(|| v.strip_prefix("0X"))
        .is_some_and(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Имя файла внутри папки zapret: без разделителей пути и `..`.
fn file_name_ok(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// `%VAR%имя` → имя, если префикс подходящий.
fn in_dir<'a>(v: &'a str, var: &str) -> Option<&'a str> {
    v.strip_prefix(var).filter(|n| file_name_ok(n))
}

fn payload_ok(name: &str, v: &str) -> bool {
    // необязательное смещение +N
    let v = match v.strip_prefix('+') {
        Some(rest) => {
            let digits = rest.chars().take_while(char::is_ascii_digit).count();
            if digits == 0 {
                return false;
            }
            &rest[digits..]
        }
        None => v,
    };
    if hex_ok(v) {
        return true;
    }
    // '!' - встроенный фейк (только для tls)
    if name == "dpi-desync-fake-tls" && v.starts_with('!') {
        return plain_value_ok(v);
    }
    in_dir(v.strip_prefix('@').unwrap_or(v), "%BIN%").is_some()
}

/// Проверяет одну опцию. Ошибка - человекочитаемая причина.
pub fn check_arg(arg: &str) -> Result<(), String> {
    // %IPSET% - место, куда Zust подставляет выбранный ipset
    if arg == "%IPSET%" {
        return Ok(());
    }
    let Some(opt) = arg.strip_prefix("--") else {
        return Err(format!("\"{arg}\" не является опцией (ожидается --имя)"));
    };
    let (name, value) = match opt.split_once('=') {
        Some((n, v)) => (n, Some(v)),
        None => (opt, None),
    };

    if PLAIN.contains(&name) {
        return match value {
            Some(v) if !plain_value_ok(v) => {
                Err(format!("недопустимые символы в значении --{name}"))
            }
            _ => Ok(()),
        };
    }
    if PAYLOAD.contains(&name) {
        return match value {
            Some(v) if payload_ok(name, v) => Ok(()),
            _ => Err(format!(
                "--{name}: разрешены только 0xHEX или файл из %BIN%"
            )),
        };
    }
    if LIST.contains(&name) {
        return match value.and_then(|v| in_dir(v, "%LISTS%")) {
            Some(_) => Ok(()),
            None => Err(format!("--{name}: разрешены только файлы из %LISTS%")),
        };
    }
    Err(format!("опция --{name} не разрешена в стратегиях"))
}

/// Проверяет все аргументы стратегии; собирает до 5 ошибок для понятного сообщения.
pub fn check(args: &[String]) -> Result<(), String> {
    let errors: Vec<String> = args.iter().filter_map(|a| check_arg(a).err()).collect();
    if errors.is_empty() {
        return Ok(());
    }
    let mut msg = String::from("Стратегия отклонена из соображений безопасности:");
    for e in errors.iter().take(5) {
        msg.push_str("\n• ");
        msg.push_str(e);
    }
    if errors.len() > 5 {
        msg.push_str(&format!("\n• …и ещё {}", errors.len() - 5));
    }
    Err(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(a: &str) {
        assert!(
            check_arg(a).is_ok(),
            "должно пройти: {a} → {:?}",
            check_arg(a)
        );
    }
    fn bad(a: &str) {
        assert!(check_arg(a).is_err(), "должно быть отклонено: {a}");
    }

    #[test]
    fn allows_normal_options() {
        ok("--new");
        ok("--wf-tcp=80,443,2053,1024-65535");
        ok("--filter-udp=19294-19344,50000-50100");
        ok("--filter-l7=discord,stun");
        ok("--dpi-desync=fake,fakedsplit");
        ok("--dpi-desync-autottl=-1:3-20");
        ok("--dpi-desync-split-pos=1,midsld");
        ok("--dpi-desync-fake-tls-mod=rnd,dupsid,sni=www.google.com");
        ok("--dpi-desync-fakedsplit-pattern=0x00");
        ok("--dpi-desync-fake-tls=%BIN%tls_clienthello_www_google_com.bin");
        ok("--dpi-desync-fake-tls=+8@%BIN%stun.bin");
        ok("--dpi-desync-fake-tls=!");
        ok("--dpi-desync-fake-tls=!+2");
        ok("--hostlist=%LISTS%list-general.txt");
        ok("--hostlist-domains=discord.media,#old.example");
        ok("--ipset=%LISTS%ipset-all.txt");
        ok("%IPSET%");
    }

    #[test]
    fn rejects_dangerous_options() {
        // запись файлов / управление процессом
        bad("--debug=@C:\\Windows\\System32\\x.dll");
        bad("--debug=1");
        bad("--pidfile=/etc/passwd");
        bad("--daemon");
        bad("--user=root");
        bad("--qnum=0");
        bad("--wf-save=C:\\x");
        bad("--hostlist-auto=%LISTS%auto.txt");
        bad("--wf-raw=true");
        bad("@C:\\config.txt");
        // произвольные файлы
        bad("--dpi-desync-fake-tls=@C:\\Users\\me\\secret.txt");
        bad("--dpi-desync-fake-tls=@/etc/shadow");
        bad("--dpi-desync-fake-tls=%BIN%..\\..\\secret");
        bad("--dpi-desync-fake-tls=%BIN%../x.bin");
        bad("--hostlist=C:\\list.txt");
        bad("--hostlist=%BIN%list.txt");
        bad("--ipset=%LISTS%sub/x.txt");
        // инъекции в командную строку
        bad("--filter-tcp=443&calc.exe");
        bad("--new|whoami");
        bad("--comment=$(id)");
        bad("--dpi-desync-repeats=6\"");
        bad("calc.exe");
        bad("--unknown-option");
    }

    #[test]
    fn error_lists_reasons() {
        let args = vec!["--new".to_string(), "--debug=@x".to_string()];
        let e = check(&args).unwrap_err();
        assert!(e.contains("--debug"));
    }
}
