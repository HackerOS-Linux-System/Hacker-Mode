import { Component, For } from "solid-js";
import { settingsStore } from "@/stores/settingsStore";
import { getText, translations, type Lang } from "@/i18n";
import { api } from "@/lib/tauri";

const Settings: Component = () => {
  const t = (key: Parameters<typeof getText>[1]) => getText(settingsStore.lang(), key);

  return (
    <div class="content">
      <div class="page-title">{t("settings")}</div>

      <div class="settings-panel">
        <h3>{t("general")}</h3>
        <div class="settings-row">
          <span>Język / Language</span>
          <select
            value={settingsStore.lang()}
            onChange={(e) => settingsStore.update({ language: e.currentTarget.value as Lang })}
          >
            <For each={Object.keys(translations) as Lang[]}>
              {(l) => <option value={l}>{l.toUpperCase()}</option>}
            </For>
          </select>
        </div>
        <div class="settings-row">
          <span>{t("wrapper_mode")}</span>
          <input
            type="checkbox"
            checked={settingsStore.settings().wrapper_mode_enabled}
            onChange={(e) => settingsStore.update({ wrapper_mode_enabled: e.currentTarget.checked })}
          />
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("gaming_tools")}</h3>
        <div class="settings-row">
          <span>{t("enable_gamescope")}</span>
          <input
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
          <input
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
          <input
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
        <h3>{t("power")}</h3>
        <div class="settings-row">
          <span>{t("power")}</span>
          <select
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
        <h3>Steam — czas gry</h3>
        <p style={{ "font-size": "12px", color: "var(--text-muted)", "margin": "0 0 10px" }}>
          Opcjonalne — Steam nie udostępnia czasu gry lokalnie. Podaj własny klucz
          Steam Web API (steamcommunity.com/dev/apikey) i swój SteamID64, żeby
          Hacker Mode dociągał realne godziny gry.
        </p>
        <div class="settings-row">
          <span>Klucz API</span>
          <input
            type="password"
            value={settingsStore.settings().steam_api_key ?? ""}
            onChange={(e) => settingsStore.update({ steam_api_key: e.currentTarget.value || null })}
            style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
          />
        </div>
        <div class="settings-row">
          <span>SteamID64</span>
          <input
            type="text"
            value={settingsStore.settings().steam_id64 ?? ""}
            onChange={(e) => settingsStore.update({ steam_id64: e.currentTarget.value || null })}
            style={{ padding: "6px", "border-radius": "6px", border: "1px solid rgba(255,255,255,0.1)", background: "var(--bg)", color: "var(--text)" }}
          />
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("audio")}</h3>
        <div class="settings-row">
          <button class="ghost-btn" onClick={() => api.audioAction("decrease")}>
            {t("decrease_volume")}
          </button>
          <button class="ghost-btn" onClick={() => api.audioAction("increase")}>
            {t("increase_volume")}
          </button>
          <button class="ghost-btn" onClick={() => api.audioAction("toggle_mute")}>
            {t("toggle_mute")}
          </button>
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("display")}</h3>
        <div class="settings-row">
          <button class="ghost-btn" onClick={() => api.displayAction("decrease")}>
            {t("decrease_brightness")}
          </button>
          <button class="ghost-btn" onClick={() => api.displayAction("increase")}>
            {t("increase_brightness")}
          </button>
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("network")}</h3>
        <div class="settings-row">
          <button class="ghost-btn" onClick={() => api.networkAction("toggle_wifi")}>
            {t("toggle_wifi")}
          </button>
        </div>
      </div>

      <div class="settings-panel">
        <h3>{t("bluetooth")}</h3>
        <div class="settings-row">
          <button class="ghost-btn" onClick={() => api.bluetoothAction("scan")}>
            {t("scan")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default Settings;
