PREFIX      ?= /usr/local
BUILD_DIR   := build
FRONTEND    := source-code/frontend
BACKEND     := source-code/backend/src-tauri

.PHONY: all frontend backend backend-dev dev install uninstall clean check test check-comphwde

all: frontend backend

## --- Frontend (Solid.js + TypeScript) -----------------------------------

frontend:
	cd $(FRONTEND) && npm install && npm run build

## --- Backend (Tauri: `hacker-mode`) + IPC CLI (`hacker-mode-ipcctl`) -----
# Jeden `cargo build --workspace` buduje obie binarki tego repozytorium -
# `hacker-mode` (pakiet `source-code/backend/src-tauri`) i
# `hacker-mode-ipcctl` (pakiet `ipc`) - żadna z nich nie wymaga niczego
# z repozytorium HWDE do zbudowania.

backend: frontend
	cargo build --release --workspace

backend-dev: frontend
	cargo build --workspace

## --- Tryb deweloperski: uruchamia tylko powłokę Tauri w oknie ------------

dev: frontend
	cargo run -p hacker-mode -- dev

## --- Testy jednostkowe (ipc + logika sklepów/launchera w backendzie) -----

test:
	cargo test -p hacker-mode-ipc
	cargo test -p hacker-mode

check:
	cargo check --workspace
	cd $(FRONTEND) && npm run check

## --- Instalacja systemowa -------------------------------------------------
# Instaluje: binarki `hacker-mode` i `hacker-mode-ipcctl` (obie zbudowane
# lokalnie, z tego repozytorium), plik sesji dla menedżerów logowania
# (SDDM/GDM/LightDM przez wayland-sessions) oraz skrypt startowy do
# uruchamiania z gołego TTY. WYMAGA jedynie osobno zainstalowanej binarki
# `comphwde` (projekt HWDE) w PATH w czasie działania sesji - patrz
# `check-comphwde` i README.

check-comphwde:
	@command -v comphwde >/dev/null || { echo "Brak 'comphwde' w PATH - zainstaluj projekt HWDE (patrz README, sekcja 'Zależność: comphwde') przed uruchomieniem sesji Hacker Mode. Budowa samego Hacker Mode ('make all') tego nie wymaga."; exit 1; }

install: all
	install -Dm755 target/release/hacker-mode          $(DESTDIR)$(PREFIX)/bin/hacker-mode
	install -Dm755 target/release/hacker-mode-ipcctl    $(DESTDIR)$(PREFIX)/bin/hacker-mode-ipcctl
	install -Dm755 scripts/hacker-mode-session          $(DESTDIR)$(PREFIX)/bin/hacker-mode-session
	install -Dm644 scripts/hacker-mode.desktop          $(DESTDIR)/usr/share/wayland-sessions/hacker-mode.desktop
	@echo "Zainstalowano. Sesję 'Hacker Mode' powinno być widać na ekranie logowania (SDDM/GDM/LightDM)."
	@echo "Do uruchomienia sesji potrzebna jest jeszcze binarka 'comphwde' (projekt HWDE) w PATH - patrz README ('make check-comphwde' sprawdzi)."

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/hacker-mode
	rm -f $(DESTDIR)$(PREFIX)/bin/hacker-mode-ipcctl
	rm -f $(DESTDIR)$(PREFIX)/bin/hacker-mode-session
	rm -f $(DESTDIR)/usr/share/wayland-sessions/hacker-mode.desktop

clean:
	cargo clean
	rm -rf $(FRONTEND)/dist $(FRONTEND)/node_modules
