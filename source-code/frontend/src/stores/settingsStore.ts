import { createSignal, onMount } from "solid-js";
import { api, type Settings } from "@/lib/tauri";
import { detectLang, type Lang } from "@/i18n";

const defaultSettings: Settings = {
  language: detectLang(),
  theme: "dark",
  wrapper_mode_enabled: true,
  gaming_tools: { gamescope: false, mangohud: true, vkbasalt: false },
  power_profile: "balanced",
  enabled_platforms: ["steam", "epic", "gog", "amazon", "lutris", "ea", "battlenet"],
  steam_api_key: null,
  steam_id64: null,
  steamgriddb_api_key: null,
  game_tags: {},
  cloud_saves_backup_dir: null,
  game_save_paths: {},
  game_controller_configs: {},
  custom_launch_prefix: null,
  notifications: { on_install: true, on_game_exit: true, on_backup_error: true },
  ea_wine_prefix: null,
  battlenet_wine_prefix: null,
  crash_detection_threshold_seconds: 8,
};

const [settings, setSettings] = createSignal<Settings>(defaultSettings);
const [lang, setLang] = createSignal<Lang>(detectLang());
const [loaded, setLoaded] = createSignal(false);

async function load() {
  try {
    const loadedSettings = await api.getSettings();
    setSettings(loadedSettings);
    setLang((loadedSettings.language as Lang) ?? detectLang());
  } catch (err) {
    // Poza kontekstem Tauri (np. `vite dev` w przeglądarce bez powłoki
    // Tauri) `invoke` się nie powiedzie — wtedy zostajemy przy domyślnych
    // ustawieniach, żeby dało się pracować nad samym UI.
    console.warn("Nie udało się wczytać ustawień z backendu:", err);
  } finally {
    setLoaded(true);
  }
}

async function update(partial: Partial<Settings>) {
  const next = { ...settings(), ...partial };
  setSettings(next);
  if (partial.language) setLang(partial.language as Lang);
  try {
    await api.saveSettings(next);
  } catch (err) {
    console.warn("Nie udało się zapisać ustawień:", err);
  }
}

/** Hook wywoływany raz w `App.tsx`, ładujący ustawienia z backendu. */
export function useSettingsBootstrap() {
  onMount(() => {
    void load();
  });
}

export const settingsStore = {
  settings,
  lang,
  loaded,
  update,
  /** Ponowne wczytanie ustawień z backendu — używane po komendach, które
   * mutują `Settings` z pominięciem `update()`/`saveSettings` (np.
   * `set_game_tags`, `set_game_cover`), żeby lokalny sygnał `settings()`
   * dogonił to, co faktycznie leży na dysku, bez wysyłania całego obiektu
   * ustawień z powrotem (co `update()` by zrobiło, ryzykując nadpisanie
   * czegoś zmienionego w międzyczasie przez tamtą dedykowaną komendę). */
  reload: load,
};
