//! zapret на macOS: utunws (nfqws для Mac от Flowseal) + pf. Трафик уходит в utun через
//! правило pf, движок живёт в LaunchDaemon: переживает закрытие окна и смену сети.

use super::{Launch, Zapret, ZapretDirs};
use crate::Hub;
use crate::logger::info;
use crate::platform::{PidFile, run_privileged, shell_quote};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const BASE: &str = "/Library/Application Support/Z.UST";
const LABEL: &str = "com.f1zzen.zust.zapret";
const PLIST: &str = "/Library/LaunchDaemons/com.f1zzen.zust.zapret.plist";
// якоря com.apple/* подключены в стандартном /etc/pf.conf
const ANCHOR: &str = "com.apple/zust";
// sc_unit 61 = utun60; у ZapretMac от Flowseal - utun50, не пересекаемся
const UTUN_UNIT: u32 = 61;
const UTUN: &str = "utun60";

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

fn base(sub: &str) -> PathBuf {
    Path::new(BASE).join(sub)
}

pub fn is_running() -> bool {
    PidFile::alive(&base("utunws.pid"), "utunws").is_some()
}

pub fn current_strategy() -> Option<String> {
    read_state().map(|s| s.strategy)
}

pub fn current_ipset() -> Option<String> {
    read_state().map(|s| s.ipset)
}

fn utunws() -> PathBuf {
    Zapret::path("bin").join("utunws")
}

pub async fn prepare(app: &Hub) -> Result<(), String> {
    if !utunws().is_file() {
        download_utunws(app).await?;
    }
    Ok(())
}

/// Закреплённый релиз ZapretMac: хеш проверяется до распаковки, чужой движок от root не запустится.
const RELEASE: &str = "https://github.com/Flowseal/zapret-mac-discord-youtube/releases/download/1.1.2/ZapretMac-macOS-universal.zip";
const RELEASE_SHA256: &str = "9895b0ec5d13ec05676f755cde71107e72d2661538440c3cc4f06e0e6d47844f";
const ENGINE_IN_ZIP: &str = "ZapretMac.app/Contents/Resources/Payload/bin/utunws";

async fn download_utunws(app: &Hub) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    info(
        app,
        "utunws не найден - скачиваю ZapretMac 1.1.2 с GitHub...",
    );
    let client = reqwest::Client::builder()
        .user_agent(concat!("zust/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let bytes = client
        .get(RELEASE)
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
    if hash != RELEASE_SHA256 {
        return Err(
            "Архив ZapretMac не совпадает с ожидаемым (sha256) - установка отменена".into(),
        );
    }

    let zip = std::env::temp_dir().join(format!("zust-{}.zip", uuid::Uuid::new_v4()));
    fs::write(&zip, &bytes).map_err(|e| e.to_string())?;
    let out = std::process::Command::new("/usr/bin/unzip")
        .arg("-p")
        .arg(&zip)
        .arg(ENGINE_IN_ZIP)
        .output();
    let _ = fs::remove_file(&zip);
    let out = out.map_err(|e| e.to_string())?;
    if !out.status.success() || out.stdout.is_empty() {
        return Err("utunws не найден в архиве ZapretMac".into());
    }
    let dest = utunws();
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&dest, out.stdout).map_err(|e| e.to_string())?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&dest, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    info(app, &format!("utunws установлен: {}", dest.display()));
    Ok(())
}

/// "80,443, 1024-65535" → "80,443,1024:65535" (диапазоны в pf - через двоеточие)
fn pf_ports(ports: &str) -> String {
    ports
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit() || c == '-'))
        .map(|p| p.replace('-', ":"))
        .collect::<Vec<_>>()
        .join(",")
}

fn pf_rules(tcp: &str, udp: &str) -> String {
    let mut rules = String::new();
    for (proto, ports) in [("tcp", pf_ports(tcp)), ("udp", pf_ports(udp))] {
        if !ports.is_empty() {
            // user { >root }: пакеты самого движка (root) обратно в utun не попадают
            rules.push_str(&format!(
                "pass out quick route-to ({UTUN} 10.78.0.2) inet proto {proto} from any to any port {{{ports}}} user {{ >root }} no state\n"
            ));
        }
    }
    rules
}

/// Скрипт службы: поднимает движок, включает перехват и следит за сменой сети.
/// Выход с ошибкой = launchd перезапустит через 2 секунды (уже с новым шлюзом).
fn run_script(args: &[String], rules: &str) -> String {
    let b = shell_quote(BASE);
    let args_q = args
        .iter()
        .map(|a| shell_quote(a))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"#!/bin/sh
B={b}
TOKEN="$B/pf-token"
ENGINE=
cleanup() {{
    trap - EXIT INT TERM HUP
    /sbin/pfctl -a {ANCHOR} -F all >/dev/null 2>&1 || true
    [ -n "$ENGINE" ] && kill "$ENGINE" 2>/dev/null
    if [ -s "$TOKEN" ]; then /sbin/pfctl -X "$(cat "$TOKEN")" >/dev/null 2>&1 || true; rm -f "$TOKEN"; fi
    rm -f "$B/utunws.pid"
}}
trap cleanup EXIT INT TERM HUP
/sbin/pfctl -a {ANCHOR} -F all >/dev/null 2>&1 || true

route() {{ /sbin/route -n get default 2>/dev/null | /usr/bin/awk "/$1:/{{print \$2; exit}}"; }}
IFACE=$(route interface)
GW=$(route gateway)
[ -n "$IFACE" ] && [ -n "$GW" ] || {{ echo "нет сети"; sleep 3; exit 1; }}
/sbin/ping -c 1 -t 1 "$GW" >/dev/null 2>&1
GW_MAC=$(/usr/sbin/arp -n "$GW" | /usr/bin/awk '/ at /{{print $4; exit}}')
case "$GW_MAC" in *:*:*:*:*:*) ;; *) echo "не удалось узнать MAC шлюза $GW"; sleep 3; exit 1 ;; esac
GW6_MAC=
GW6=$(/sbin/route -n get -inet6 default 2>/dev/null | /usr/bin/awk '/gateway:/{{print $2; exit}}')
if [ -n "$GW6" ]; then
    GW6_MAC=$(/usr/sbin/ndp -n "$GW6" 2>/dev/null | /usr/bin/awk '{{for(i=1;i<=NF;i++) if($i ~ /^([0-9a-f]{{1,2}}:){{5}}[0-9a-f]{{1,2}}$/){{print $i; exit}}}}')
fi

ZAPRET_IFACE="$IFACE" ZAPRET_GATEWAY_MAC="$GW_MAC" ZAPRET_GATEWAY6_MAC="$GW6_MAC" ZAPRET_UTUN_UNIT={UTUN_UNIT} \
    "$B/bin/utunws" {args_q} >>"$B/engine.log" 2>&1 &
ENGINE=$!
echo "$ENGINE" > "$B/utunws.pid"
chmod 644 "$B/utunws.pid"

I=0
while ! /sbin/ifconfig {UTUN} >/dev/null 2>&1; do
    I=$((I + 1))
    if ! kill -0 "$ENGINE" 2>/dev/null || [ "$I" -gt 100 ]; then exit 1; fi
    sleep 0.1
done
/sbin/ifconfig {UTUN} 10.78.0.1 10.78.0.2 netmask 255.255.255.255 up

if /sbin/pfctl -s info 2>/dev/null | /usr/bin/grep -q '^Status: Disabled'; then
    /sbin/pfctl -E 2>&1 | /usr/bin/awk '/Token :/ {{ print $3 }}' > "$TOKEN"
fi
/sbin/pfctl -a {ANCHOR} -f - <<'RULES'
{rules}RULES

while kill -0 "$ENGINE" 2>/dev/null; do
    sleep 2
    [ "$(route interface)" = "$IFACE" ] && [ "$(route gateway)" = "$GW" ] || exit 1
done
exit 1
"#
    )
}

fn plist() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         \x20   <key>Label</key><string>{LABEL}</string>\n\
         \x20   <key>ProgramArguments</key>\n\
         \x20   <array><string>/bin/sh</string><string>{BASE}/run.sh</string></array>\n\
         \x20   <key>RunAtLoad</key><true/>\n\
         \x20   <key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>\n\
         \x20   <key>ThrottleInterval</key><integer>2</integer>\n\
         \x20   <key>StandardOutPath</key><string>{BASE}/zapret.log</string>\n\
         \x20   <key>StandardErrorPath</key><string>{BASE}/zapret.log</string>\n\
         </dict>\n</plist>\n"
    )
}

/// Общая часть скриптов: выгрузить службу и убрать перехват.
pub fn stop_snippet() -> String {
    let b = shell_quote(BASE);
    format!(
        "/bin/launchctl bootout system/{LABEL} >/dev/null 2>&1 || true\n\
         /sbin/pfctl -a {ANCHOR} -F all >/dev/null 2>&1 || true\n\
         if [ -f {b}/utunws.pid ]; then kill \"$(cat {b}/utunws.pid)\" 2>/dev/null || true; rm -f {b}/utunws.pid; fi\n\
         if [ -s {b}/pf-token ]; then /sbin/pfctl -X \"$(cat {b}/pf-token)\" >/dev/null 2>&1 || true; rm -f {b}/pf-token; fi\n\
         rm -f {PLIST}\n"
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
    run_privileged(&script).map_err(|e| format!("utunws не запустился: {e}"))?;
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
    let run_dirs = ZapretDirs {
        bin: base("bin"),
        lists: base("lists"),
        ipset_configs: base("ipset-configs"),
    };
    let Launch {
        args,
        tcp_ports,
        udp_ports,
    } = Zapret::build_launch(raw, &run_dirs, ipset, game_filter)?;
    // --wf-* - фильтры WinDivert, на macOS их роль выполняют правила pf
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
    info(app, &format!("Аргументы utunws: {}", args.join(" ")));

    let run_file = Zapret::path(&format!("run{tag}.sh"));
    let plist_file = Zapret::path("zust-zapret.plist");
    fs::write(&run_file, run_script(&args, &pf_rules(&tcp, &udp))).map_err(|e| e.to_string())?;
    fs::write(&plist_file, plist()).map_err(|e| e.to_string())?;

    let q = |p: &Path| shell_quote(&p.to_string_lossy());
    let b = shell_quote(BASE);
    Ok(format!(
        "set -e\n\
         {stop}\
         mkdir -p {b}/bin {b}/lists {b}/ipset-configs\n\
         rm -f {b}/lists/* {b}/ipset-configs/*\n\
         cp {src}/lists/*.txt {b}/lists/\n\
         cp {src}/bin/*.bin {b}/bin/ 2>/dev/null || true\n\
         cp {src}/ipset-configs/*.txt {b}/ipset-configs/ 2>/dev/null || true\n\
         cp {engine} {b}/bin/utunws\n\
         cp {run} {b}/run.sh\n\
         : > {b}/engine.log\n\
         /usr/bin/xattr -dr com.apple.quarantine {b} 2>/dev/null || true\n\
         chown -R root:wheel {b}\n\
         chmod -R a+rX {b}\n\
         chmod 755 {b}/bin/utunws\n\
         cp {plist} {PLIST}\n\
         chown root:wheel {PLIST}\n\
         chmod 644 {PLIST}\n\
         I=0\n\
         until /bin/launchctl bootstrap system {PLIST} 2>/dev/null; do\n\
         \x20 I=$((I + 1)); [ $I -lt 10 ] || {{ echo 'launchctl bootstrap не удался' >&2; exit 1; }}\n\
         \x20 sleep 0.5\n\
         done\n\
         I=0\n\
         until /sbin/pfctl -a {ANCHOR} -s rules 2>/dev/null | grep -q route-to; do\n\
         \x20 I=$((I + 1))\n\
         \x20 if [ $I -gt 50 ]; then\n\
         \x20   tail -n 20 {b}/zapret.log {b}/engine.log >&2 2>/dev/null\n\
         \x20   {stop_inline}\
         \x20   exit 1\n\
         \x20 fi\n\
         \x20 sleep 0.2\n\
         done\n",
        stop = stop_snippet(),
        stop_inline = stop_snippet().replace('\n', "\n    "),
        src = q(&Zapret::path("")),
        engine = q(&utunws()),
        run = q(&run_file),
        plist = q(&plist_file),
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
    if !Path::new(PLIST).exists() {
        return Ok(());
    }
    run_privileged(&stop_snippet())?;
    info(app, "utunws остановлен, правила pf удалены.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports() {
        assert_eq!(pf_ports("80,443, 2053,,12"), "80,443,2053,12");
        assert_eq!(
            pf_ports("19294-19344,50000-50100"),
            "19294:19344,50000:50100"
        );
        assert_eq!(pf_ports("%GameFilter%"), "");
    }

    #[test]
    fn rules_skip_empty() {
        let r = pf_rules("80,443", "");
        assert!(r.contains("proto tcp from any to any port {80,443}"));
        assert!(!r.contains("proto udp"));
    }
}
