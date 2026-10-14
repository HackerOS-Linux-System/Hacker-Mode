#!/usr/bin/env bash
# Test sesji bez GPU i bez monitora: labwc w trybie headless + atrapa powłoki.
# Sprawdza REALNY cykl życia: wybór kompozytora z config.json, kopiowanie
# rc.xml, start labwc, uruchomienie klienta z WAYLAND_DISPLAY i zakończenie
# kompozytora po wyjściu powłoki (`labwc --exit`).
#
# Wymaga: labwc (>= 0.7), bash, coreutils. Użycie: scripts/test-session-headless.sh
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SESSION="$HERE/hacker-mode-session"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "ok:   $*"; }

new_env() { # tworzy izolowane HOME/XDG
  rm -rf "${TMP:?}/home"; mkdir -p "$TMP/home" "$TMP/run"; chmod 700 "$TMP/run"
  export HOME="$TMP/home" XDG_RUNTIME_DIR="$TMP/run"
  unset XDG_CONFIG_HOME XDG_STATE_HOME HACKER_MODE_COMPOSITOR DBUS_SESSION_BUS_ADDRESS
}

# --- 1. rozstrzyganie kompozytora -------------------------------------------
new_env
[ "$("$SESSION" --print-compositor)" = "labwc" ] && pass "domyślnie labwc" || fail "domyślny kompozytor"
mkdir -p "$HOME/.config/Hacker-Mode"
echo '{"compositor": "comphwde"}' > "$HOME/.config/Hacker-Mode/config.json"
[ "$("$SESSION" --print-compositor)" = "comphwde" ] && pass "config.json -> comphwde" || fail "config.json"
echo '{"compositor": "COMPHWDE", "theme":"dark"}' > "$HOME/.config/Hacker-Mode/config.json"
[ "$("$SESSION" --print-compositor)" = "comphwde" ] && pass "wielkość liter ignorowana" || fail "case"
[ "$(HACKER_MODE_COMPOSITOR=labwc "$SESSION" --print-compositor)" = "labwc" ] && pass "zmienna środowiskowa ma pierwszeństwo" || fail "env"
echo '{"compositor": "sway"}' > "$HOME/.config/Hacker-Mode/config.json"
[ "$("$SESSION" --print-compositor 2>/dev/null)" = "labwc" ] && pass "nieznana wartość -> labwc" || fail "nieznana wartość"
new_env; mkdir -p "$HOME/.config/hacker-mode"
echo '{"compositor": "comphwde"}' > "$HOME/.config/hacker-mode/config.json"
[ "$("$SESSION" --print-compositor)" = "comphwde" ] && pass "stary config.json (migracja)" || fail "legacy"

# --- 2. pełny cykl życia labwc (headless) -----------------------------------
command -v labwc >/dev/null || { echo "SKIP: brak labwc - pomijam test cyklu życia"; exit 0; }
new_env
cat > "$TMP/fake-shell" <<STUB
#!/bin/sh
{
  echo "compositor=\$HACKER_MODE_COMPOSITOR"
  echo "wayland=\$WAYLAND_DISPLAY"
  echo "session_type=\$XDG_SESSION_TYPE"
} > "$TMP/shell-env.txt"
sleep 1
exit 0
STUB
chmod +x "$TMP/fake-shell"

export WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman
export HACKER_MODE_SHELL_BIN="$TMP/fake-shell"
export HACKER_MODE_TEMPLATE_DIR="$HERE/labwc"
export HACKER_MODE_DBUS_WRAPPED=1   # bez dbus-run-session w teście

start=$(date +%s)
timeout 40 "$SESSION" || rc=$?
elapsed=$(( $(date +%s) - start ))
[ "${rc:-0}" -ne 124 ] || fail "sesja nie zakończyła się po wyjściu powłoki (timeout) - labwc --exit nie zadziałał"
pass "sesja zakończyła się sama po wyjściu powłoki (${elapsed}s)"

[ -f "$TMP/shell-env.txt" ] || fail "powłoka nigdy nie wystartowała (log: $HOME/.local/state/hacker-mode/compositor.log)"
grep -q '^compositor=labwc$' "$TMP/shell-env.txt" && pass "powłoka widzi HACKER_MODE_COMPOSITOR=labwc" || fail "brak HACKER_MODE_COMPOSITOR"
grep -q '^wayland=.\+' "$TMP/shell-env.txt" && pass "powłoka dostała WAYLAND_DISPLAY" || fail "brak WAYLAND_DISPLAY"
[ -f "$HOME/.config/Hacker-Mode/config.json" ] && pass "utworzono domyślny config.json" || fail "config.json"
[ "$(stat -c %a "$HOME/.config/Hacker-Mode/config.json")" = "600" ] && pass "config.json ma uprawnienia 600" || fail "uprawnienia config.json"
grep -q 'hacker-mode:generated' "$HOME/.config/Hacker-Mode/labwc/rc.xml" && pass "skopiowano rc.xml" || fail "rc.xml"

# własny rc.xml (bez znacznika) nie może być nadpisany
echo '<labwc_config/>' > "$HOME/.config/Hacker-Mode/labwc/rc.xml"
rm -f "$TMP/shell-env.txt"
timeout 40 "$SESSION" 2>/dev/null || true
grep -q '^<labwc_config/>$' "$HOME/.config/Hacker-Mode/labwc/rc.xml" && pass "własny rc.xml zachowany" || fail "własny rc.xml został nadpisany"
echo "WSZYSTKIE TESTY SESJI PRZESZŁY"
