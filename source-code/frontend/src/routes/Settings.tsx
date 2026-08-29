import { Component, createSignal, For, onMount, Show } from "solid-js";
import { save as saveDialog, open as openDialog } from "@tauri-apps/plugin-dialog";
import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
import { settingsStore } from "@/stores/settingsStore";
import { getText, LANGUAGES, type Lang } from "@/i18n";
import { api, PLATFORM_LABELS, type Platform, type ExternalApp } from "@/lib/tauri";

/// Odpowiada `Platform::slug()`/`Platform::label()` po stronie Rust
/// (`commands/stores/mod.rs`) — lista platform, które backend faktycznie
/// wie jak obsłużyć (patrz `stores::list_all_games`), więc checkboxy tu
/// nigdy nie odwołują się do nieistniejącej platformy.
const PLATFORM_OPTIONS: Platform[] = ["steam", "epic", "gog", "amazon", "lutris", "ea", "battlenet"];

/// Odpowiednik `Settings::theme` — nazwy muszą się zgadzać z selektorami
/// `[data-theme="..."]` w `global.css`.
const THEMES: { id: string; label: string }[] = [
  { id: "dark", label: "Ciemny (domyślny)" },
  { id: "midnight", label: "Midnight Blue" },
  { id: "light", label: "Jasny" },
];

const Settings: Component = () => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);
  const [syncMessage, setSyncMessage] = createSignal<string | null>(null);
  const [externalApps, setExternalApps] = createSignal<ExternalApp[]>([]);

  onMount(async () => {
    try {
      setExternalApps(await api.listExternalApps());
    } catch (err) {
      console.warn("Nie udało się wykryć zewnętrznych aplikacji:", err);
    }
  });

  /** Eksportuje AKTUALNE ustawienia Hacker Mode do pliku JSON wybranego
   * przez użytkownika — patrz `commands::export_settings`. Ten sam plik
   * da się zaimportować na innym urządzeniu (`importSettingsFromFile`
   * niżej) — świadomie ręczny mechanizm, nie automatyczna synchronizacja
   * w tle (Hacker Mode nie ma do tego żadnej infrastruktury serwerowej). */
  async function exportSettingsToFile() {
    setSyncMessage(null);
    try {
      const path = await saveDialog({
        defaultPath: "hacker-mode-settings.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      const json = await api.exportSettings();
      await writeTextFile(path, json);
      setSyncMessage("Wyeksportowano ustawienia.");
    } catch (err) {
      setSyncMessage(String(err));
    }
  }

  async function importSettingsFromFile() {
    setSyncMessage(null);
    try {
      const path = await openDialog({ filters: [{ name: "JSON", extensions: ["json"] }], multiple: false });
      if (!path || Array.isArray(path)) return;
      const json = await readTextFile(path);
      await api.importSettings(json);
      await settingsStore.reload();
      setSyncMessage("Zaimportowano ustawienia. Niektóre zmiany mogą wymagać odświeżenia widoku.");
    } catch (err) {
      setSyncMessage(String(err));
    }
  }

  return (
    <div class="content">
      <div class="page-title">{t("settings")}</div>

      <div class="settings-panel">
        <h3>{t("general")}</h3>
        <div class="settings-row">
          <span>{t("language")}</span>
          <select data-focusable tabIndex={0}
            value={settingsStore.lang()}
            onChange={(e) => settingsStore.update({ language: e.currentTarget.value as Lang })}
          >
            <For each={LANGUAGES}>
              {(l) => <option value={l.id}>{l.label}</option>}
            </For>
          </select>
        </div>
        <div class="settings-row">
          <span>{t("wrapper_mode")}</span>
          <input data-focusable tabIndex={0}
            type="checkbox"
            checked={settingsStore.settings().wrapper_mode_enabled}
            onChange={(e) => settingsStore.update({ wrapper_mode_enabled: e.currentTarget.checked })}
          />
        </div>
        <div class="settings-row">
          <span>Motyw</span>
          <select data-focusable tabIndex={0}
            value={settingsStore.settings().theme}
            onChange={(e) => settingsStore.update({ theme: e.currentTarget.value })}
          >
            <For each={THEMES}>{(th) => <option value={th.id}>{th.label}</option>}</For>
          </select>
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("visible_platforms")}</h3>
        <For each={PLATFORM_OPTIONS}>
          {(platform) => (
            <div class="settings-row">
              <span>{PLATFORM_LABELS[platform]}</span>
              <input data-focusable tabIndex={0}
                type="checkbox"
                checked={settingsStore.settings().enabled_platforms.includes(platform)}
                onChange={(e) => {
                  const current = settingsStore.settings().enabled_platforms;
                  const next = e.currentTarget.checked
                    ? [...current, platform]
                    : current.filter((p) => p !== platform);
                  settingsStore.update({ enabled_platforms: next });
                }}
              />
            </div>
          )}
        </For>
      </div>

      <div class="settings-panel">
        <h3>EA app / Battle.net — prefiks Wine</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 10px" }}>
          Opcjonalne. Hacker Mode sam wykrywa te platformy pod standardowymi
          lokalizacjami Lutrisa (<code>~/Games/ea-app</code>,{" "}
          <code>~/Games/battlenet</code>). Jeśli zainstalowałeś je gdzie
          indziej, podaj tu pełną ścieżkę do prefiksu Wine.
        </p>
        <div class="settings-row">
          <span>Prefiks EA app</span>
          <input data-focusable tabIndex={0}
            type="text"
            value={settingsStore.settings().ea_wine_prefix ?? ""}
            onChange={(e) => settingsStore.update({ ea_wine_prefix: e.currentTarget.value || null })}
            placeholder="/home/ty/Games/ea-app"
            style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
          />
        </div>
        <div class="settings-row">
          <span>Prefiks Battle.net</span>
          <input data-focusable tabIndex={0}
            type="text"
            value={settingsStore.settings().battlenet_wine_prefix ?? ""}
            onChange={(e) => settingsStore.update({ battlenet_wine_prefix: e.currentTarget.value || null })}
            placeholder="/home/ty/Games/battlenet"
            style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
          />
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("gaming_tools")}</h3>
        <div class="settings-row">
          <span>{t("enable_gamescope")}</span>
          <input data-focusable tabIndex={0}
            type="checkbox"
            checked={settingsStore.settings().gaming_tools.gamescope}
            onChange={(e) =>
              settingsStore.update({
                gaming_tools: { ...settingsStore.settings().gaming_tools, gamescope: e.currentTarget.checked },
              })
            }
          />
        </div>
        <div class="settings-row">
          <span>{t("enable_mangohud")}</span>
          <input data-focusable tabIndex={0}
            type="checkbox"
            checked={settingsStore.settings().gaming_tools.mangohud}
            onChange={(e) =>
              settingsStore.update({
                gaming_tools: { ...settingsStore.settings().gaming_tools, mangohud: e.currentTarget.checked },
              })
            }
          />
        </div>
        <div class="settings-row">
          <span>{t("enable_vkbasalt")}</span>
          <input data-focusable tabIndex={0}
            type="checkbox"
            checked={settingsStore.settings().gaming_tools.vkbasalt}
            onChange={(e) =>
              settingsStore.update({
                gaming_tools: { ...settingsStore.settings().gaming_tools, vkbasalt: e.currentTarget.checked },
              })
            }
          />
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("advanced_launch_panel_title")}</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 10px" }}>
          {t("advanced_launch_panel_body")}
        </p>
        <div class="settings-row">
          <span>{t("custom_launch_prefix_label")}</span>
          <input data-focusable tabIndex={0}
            type="text"
            value={settingsStore.settings().custom_launch_prefix ?? ""}
            onChange={(e) => {
              const value = e.currentTarget.value.trim() || null;
              void api.setCustomLaunchPrefix(value);
              settingsStore.update({ custom_launch_prefix: value });
            }}
            placeholder="gamemoderun"
            style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)", width: "220px" }}
          />
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("notifications_panel_title")}</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 10px" }}>
          {t("notifications_panel_body")}
        </p>
        <div class="settings-row">
          <span>{t("notification_on_install")}</span>
          <input data-focusable tabIndex={0}
            type="checkbox"
            checked={settingsStore.settings().notifications.on_install}
            onChange={(e) => {
              const next = { ...settingsStore.settings().notifications, on_install: e.currentTarget.checked };
              void api.setNotificationSettings(next);
              settingsStore.update({ notifications: next });
            }}
          />
        </div>
        <div class="settings-row">
          <span>{t("notification_on_game_exit")}</span>
          <input data-focusable tabIndex={0}
            type="checkbox"
            checked={settingsStore.settings().notifications.on_game_exit}
            onChange={(e) => {
              const next = { ...settingsStore.settings().notifications, on_game_exit: e.currentTarget.checked };
              void api.setNotificationSettings(next);
              settingsStore.update({ notifications: next });
            }}
          />
        </div>
        <div class="settings-row">
          <span>{t("notification_on_backup_error")}</span>
          <input data-focusable tabIndex={0}
            type="checkbox"
            checked={settingsStore.settings().notifications.on_backup_error}
            onChange={(e) => {
              const next = { ...settingsStore.settings().notifications, on_backup_error: e.currentTarget.checked };
              void api.setNotificationSettings(next);
              settingsStore.update({ notifications: next });
            }}
          />
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("power")}</h3>
        <div class="settings-row">
          <span>{t("power")}</span>
          <select data-focusable tabIndex={0}
            value={settingsStore.settings().power_profile}
            onChange={(e) => {
              settingsStore.update({ power_profile: e.currentTarget.value });
              void api.powerAction(e.currentTarget.value as never);
            }}
          >
            <option value="power_saving">{t("power_saving")}</option>
            <option value="balanced">{t("balanced")}</option>
            <option value="performance">{t("performance")}</option>
          </select>
        </div>
      </div>

      <div class="settings-panel">
        <h3>Steam — pełna biblioteka i czas gry</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 10px" }}>
          Bez tego Hacker Mode widzi tylko gry Steam już zainstalowane na tym
          komputerze. Podaj własny klucz Steam Web API (wygenerowany na{" "}
          <span style={{ "font-family": "var(--font-mono)" }}>steamcommunity.com/dev/apikey</span>), a
          Hacker Mode dociągnie też gry, które POSIADASZ, ale jeszcze nie
          zainstalowałeś (z przyciskiem instalacji), oraz realny czas gry.
        </p>
        <div class="settings-row">
          <span>Klucz API</span>
          <input data-focusable tabIndex={0}
            type="password"
            value={settingsStore.settings().steam_api_key ?? ""}
            onChange={(e) => settingsStore.update({ steam_api_key: e.currentTarget.value || null })}
            style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
          />
        </div>
        <div class="settings-row">
          <span>SteamID64</span>
          <input data-focusable tabIndex={0}
            type="text"
            value={settingsStore.settings().steam_id64 ?? ""}
            onChange={(e) => settingsStore.update({ steam_id64: e.currentTarget.value || null })}
            placeholder="Wykrywane automatycznie z lokalnego Steama"
            style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
          />
        </div>
        <p style={{ "font-size": "11px", color: "var(--text-muted)", "margin": "8px 0 0" }}>
          SteamID64 zwykle nie trzeba wpisywać ręcznie — Hacker Mode
          rozpoznaje je samo z lokalnie zalogowanego konta Steam. Wpisz je
          tu tylko, jeśli chcesz pociągnąć bibliotekę innego konta.
        </p>
      </div>

      <div class="settings-panel">
        <h3>{t("steamgriddb_panel_title")}</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 10px" }}>
          {t("steamgriddb_panel_body")}
        </p>
        <div class="settings-row">
          <span>{t("steamgriddb_api_key_label")}</span>
          <input data-focusable tabIndex={0}
            type="password"
            value={settingsStore.settings().steamgriddb_api_key ?? ""}
            onChange={(e) => settingsStore.update({ steamgriddb_api_key: e.currentTarget.value || null })}
            style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
          />
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("cloud_saves_panel_title")}</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 10px" }}>
          {t("cloud_saves_panel_body")}
        </p>
        <div class="settings-row">
          <span>{t("cloud_saves_backup_dir_label")}</span>
          <input data-focusable tabIndex={0}
            type="text"
            value={settingsStore.settings().cloud_saves_backup_dir ?? ""}
            onChange={(e) => settingsStore.update({ cloud_saves_backup_dir: e.currentTarget.value || null })}
            placeholder="/home/deck/Dropbox/hacker-mode-saves"
            style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)", width: "260px" }}
          />
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("audio")}</h3>
        <div class="settings-row">
          <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => api.audioAction("decrease")}>
            {t("decrease_volume")}
          </button>
          <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => api.audioAction("increase")}>
            {t("increase_volume")}
          </button>
          <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => api.audioAction("toggle_mute")}>
            {t("toggle_mute")}
          </button>
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("display")}</h3>
        <div class="settings-row">
          <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => api.displayAction("decrease")}>
            {t("decrease_brightness")}
          </button>
          <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => api.displayAction("increase")}>
            {t("increase_brightness")}
          </button>
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("network")}</h3>
        <div class="settings-row">
          <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => api.networkAction("toggle_wifi")}>
            {t("toggle_wifi")}
          </button>
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("bluetooth")}</h3>
        <div class="settings-row">
          <button data-focusable tabIndex={0} class="ghost-btn" onClick={() => api.bluetoothAction("scan")}>
            {t("scan")}
          </button>
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("sync_settings_panel_title")}</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 10px" }}>
          {t("sync_settings_panel_body")}
        </p>
        <div style={{ display: "flex", gap: "8px", "margin-bottom": "8px" }}>
          <button data-focusable tabIndex={0} class="ghost-btn" onClick={exportSettingsToFile}>
            {t("sync_settings_export")}
          </button>
          <button data-focusable tabIndex={0} class="ghost-btn" onClick={importSettingsFromFile}>
            {t("sync_settings_import")}
          </button>
        </div>
        <Show when={syncMessage()}>
          <p style={{ "font-size": "11px", color: "var(--text-muted)", margin: "0" }}>{syncMessage()}</p>
        </Show>
      </div>

      <Show when={externalApps().length > 0}>
        <div class="settings-panel">
          <h3>{t("external_apps_panel_title")}</h3>
          <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 10px" }}>
            {t("external_apps_panel_body")}
          </p>
          <div style={{ display: "flex", "flex-direction": "column", gap: "4px" }}>
            <For each={externalApps()}>
              {(app) => (
                <div style={{ display: "flex", "justify-content": "space-between", "font-size": "12px" }}>
                  <span>{app.name}</span>
                  <span style={{ color: "var(--text-muted)" }}>{app.source}</span>
                </div>
              )}
            </For>
          </div>
        </div>
      </Show>
    </div>
  );
};

export default Settings;
