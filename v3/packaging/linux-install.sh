#!/bin/sh
# Установщик Z.UST для Linux. Внутри файла - архив с программой (после строки __ARCHIVE__).
#   sh Z.UST-linux.sh            установить или обновить
#   sh Z.UST-linux.sh --remove   удалить (настройки в ~/.config/com.f1zzen.zust остаются)
set -eu

DEST="${XDG_DATA_HOME:-$HOME/.local/share}/Z.UST"
BIN="$HOME/.local/bin"

say() { printf '%s\n' "$*"; }
fail() { say "Ошибка: $*" >&2; exit 1; }
ask() {
    printf '%s [y/N] ' "$1"
    { read -r a </dev/tty; } 2>/dev/null || a=""
    case "$a" in [yYдД]*) return 0 ;; *) return 1 ;; esac
}

if [ "${1:-}" = "--remove" ]; then
    pkill -x zust 2>/dev/null || true
    rm -rf "$DEST"
    rm -f "$BIN/zust" \
        "${XDG_DATA_HOME:-$HOME/.local/share}/applications/zust.desktop" \
        "${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/128x128/apps/zust.png"
    say "Z.UST удалён. Настройки и логи остались в ~/.config/com.f1zzen.zust"
    exit 0
fi

[ "$(id -u)" -ne 0 ] || fail "запускайте без sudo: Z.UST ставится для текущего пользователя и сам спросит пароль, когда нужно."
[ "$(uname -m)" = "x86_64" ] || fail "нужен 64-битный x86 (x86_64), а у вас $(uname -m)."
if ldd --version 2>&1 | grep -qi musl; then
    fail "система на musl (Alpine, Void musl). Z.UST собран под glibc и здесь не запустится."
fi
glibc=$(getconf GNU_LIBC_VERSION 2>/dev/null | awk '{print $2}')
if [ -n "$glibc" ]; then
    major=${glibc%%.*}
    minor=${glibc#*.}
    minor=${minor%%.*}
    if [ "$major" -lt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -lt 35 ]; }; then
        fail "слишком старая система (glibc $glibc, нужна 2.35+: Ubuntu 22.04, Debian 12, Fedora 36 и новее)."
    fi
fi

# распаковка во временную папку
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
line=$(awk '/^__ARCHIVE__$/ { print NR + 1; exit }' "$0")
tail -n +"$line" "$0" | tar xz -C "$TMP" || fail "архив внутри установщика повреждён - скачайте его заново."

# библиотеки, без которых программа не стартует
missing_libs=$(ldd "$TMP/zust" 2>/dev/null | awk '/not found/ { print $1 }' | tr '\n' ' ')

# программы: обязательные и для TOR / точки доступа
have() { command -v "$1" >/dev/null 2>&1; }
need=""
have nft || need="$need nftables"
have pkexec || need="$need polkit"
opt=""
have tor || opt="$opt tor"
have lyrebird || have obfs4proxy || opt="$opt obfs4proxy"

# имена пакетов немного отличаются в разных дистрибутивах
if have apt-get; then
    pm="sudo apt-get install -y"; pk_polkit=pkexec; pk_fc=libfontconfig1; pk_ssl=libssl3
elif have dnf; then
    pm="sudo dnf install -y"; pk_polkit=polkit; pk_fc=fontconfig; pk_ssl=openssl-libs
elif have pacman; then
    pm="sudo pacman -S --needed"; pk_polkit=polkit; pk_fc=fontconfig; pk_ssl=openssl
elif have zypper; then
    pm="sudo zypper install -y"; pk_polkit=polkit; pk_fc=fontconfig; pk_ssl=libopenssl3
elif have xbps-install; then
    pm="sudo xbps-install -S"; pk_polkit=polkit; pk_fc=fontconfig; pk_ssl=openssl
else
    pm=""
fi
pkgs() {
    out=""
    for p in "$@"; do
        case "$p" in
            polkit) p=${pk_polkit:-polkit} ;;
            obfs4proxy) case "$pm" in *dnf*|*zypper*) p=obfs4 ;; esac ;;
        esac
        out="$out $p"
    done
    printf '%s' "$out"
}
unknown=""
for l in $missing_libs; do
    case "$l" in
        libfontconfig*) need="$need ${pk_fc:-fontconfig}" ;;
        libssl*|libcrypto*) need="$need ${pk_ssl:-openssl}" ;;
        *) unknown="$unknown $l" ;;
    esac
done
[ -z "$unknown" ] || fail "не хватает библиотек:$unknown. Найдите их в пакетах вашего дистрибутива."
need=$(printf '%s\n' $need | sort -u | tr '\n' ' ')

install_pkgs() {
    list=$(pkgs "$@")
    if [ -z "$pm" ]; then
        say "Установите вручную:$list"
        return 1
    fi
    say "Команда: $pm$list"
    if ask "Установить сейчас?"; then
        $pm $list
    else
        return 1
    fi
}

if [ -n "$(printf '%s' "$need" | tr -d ' ')" ]; then
    say "Для работы Z.UST не хватает:$need"
    install_pkgs $need || fail "без них zapret не запустится."
fi
if [ -n "$opt" ]; then
    say "Для TOR нужны:$opt (для zapret не обязательно)."
    install_pkgs $opt || say "Можно поставить позже, когда понадобится TOR."
fi

# установк
if pgrep -x zust >/dev/null 2>&1; then
    ask "Z.UST сейчас запущен. Закрыть его и обновить?" || fail "установка отменена."
    pkill -x zust || true
    sleep 1
fi
mkdir -p "$DEST" "$BIN"
rm -rf "$DEST/zapret"
cp -r "$TMP"/. "$DEST"/
chmod +x "$DEST/zust"
ln -sf "$DEST/zust" "$BIN/zust"

say ""
say "Z.UST установлен в $DEST"
case ":$PATH:" in
    *":$BIN:"*) say "Запуск: zust (при первом запуске Z.UST предложит добавить себя в меню приложений)" ;;
    *) say "Запуск: $BIN/zust (при первом запуске Z.UST предложит добавить себя в меню приложений)" ;;
esac
say "Удалить: sh $(basename "$0") --remove"
if ask "Запустить Z.UST сейчас?"; then
    nohup "$DEST/zust" >/dev/null 2>&1 &
fi
exit 0
__ARCHIVE__
