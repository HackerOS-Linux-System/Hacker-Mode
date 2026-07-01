# Hacker Mode — Makefile
#
# Skraca budowanie/instalację jednej binarki `hacker-mode` (Tauri + Svelte +
# Rust) oraz jej dwóch trybów uruchomienia ("ui" / "default" przez SDDM).

SHELL := /bin/bash

PREFIX      ?= /usr/local
BINDIR      := $(PREFIX)/bin
SESSIONDIR  := /usr/share/wayland-sessions
BIN_NAME    := hacker-mode
TARGET_BIN  := target/release/$(BIN_NAME)

UI_DIR := ui

.PHONY: help deps ui-install ui-build build dev run-ui run-default \
        install uninstall clean fmt check

help: ## Pokaż listę dostępnych komend
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

deps: ## Sprawdź obecność narzędzi/bibliotek wymaganych do budowy
	@command -v cargo >/dev/null || { echo "Brak cargo (Rust). Zainstaluj: https://rustup.rs"; exit 1; }
	@command -v npm   >/dev/null || { echo "Brak npm/node.js"; exit 1; }
	@pkg-config --exists webkit2gtk-4.1 2>/dev/null || \
		echo "UWAGA: nie znaleziono webkit2gtk-4.1 przez pkg-config — build Tauri może się nie powieść."
	@echo "Zależności OK."

ui-install: ## Zainstaluj zależności npm frontendu (Svelte)
	cd $(UI_DIR) && npm install

ui-build: ui-install ## Zbuduj frontend (Vite -> ui/dist)
	cd $(UI_DIR) && npm run build

build: ui-build ## Pełny build release (frontend + Rust)
	cargo build --release
	@echo "Gotowe: $(TARGET_BIN)"

check: ## Szybka weryfikacja kompilacji Rust bez pełnego linkowania (cargo check)
	cargo check

fmt: ## Sformatuj kod Rust
	cargo fmt

dev: ## Tryb deweloperski: vite dev server + `cargo run -- ui` w jednym terminalu
	@trap 'kill 0' EXIT; \
	( cd $(UI_DIR) && npm run dev ) & \
	sleep 2; \
	cargo run -- ui

run-ui: build ## Uruchom lokalnie w trybie okienkowym (bez instalacji systemowej)
	./$(TARGET_BIN) ui

run-default: build ## Uruchom lokalnie w trybie kompozytora (wymaga uprawnień seat/DRM, patrz README)
	./$(TARGET_BIN) default

install: build ## Zainstaluj binarkę + sesję SDDM w systemie (wymaga sudo)
	install -Dm755 $(TARGET_BIN) $(DESTDIR)$(BINDIR)/$(BIN_NAME)
	install -Dm755 packaging/hacker-mode-session $(DESTDIR)$(BINDIR)/hacker-mode-session
	install -Dm644 packaging/hacker-mode.desktop $(DESTDIR)$(SESSIONDIR)/hacker-mode.desktop
	@sed -i "s|/usr/bin/hacker-mode|$(BINDIR)/hacker-mode|g" \
		$(DESTDIR)$(BINDIR)/hacker-mode-session \
		$(DESTDIR)$(SESSIONDIR)/hacker-mode.desktop
	@echo "Zainstalowano. Sesja 'Hacker Mode' powinna być teraz dostępna w SDDM."

uninstall: ## Odinstaluj z systemu
	rm -f $(DESTDIR)$(BINDIR)/$(BIN_NAME)
	rm -f $(DESTDIR)$(BINDIR)/hacker-mode-session
	rm -f $(DESTDIR)$(SESSIONDIR)/hacker-mode.desktop
	@echo "Odinstalowano."

clean: ## Wyczyść artefakty budowania (Rust + frontend)
	cargo clean
	rm -rf $(UI_DIR)/dist $(UI_DIR)/node_modules
