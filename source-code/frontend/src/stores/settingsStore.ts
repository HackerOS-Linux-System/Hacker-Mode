import { createSignal, onMount } from "solid-js";
import { api, type Settings } from "@/lib/tauri";
import { detectLang, type Lang } from "@/i18n";

const defaultSettings: Settings = {
  language: detectLang(),
  theme: "dark",
  wrapper_mode_enabled: true,
  gaming_tools: { gamescope: false, mangohud: true, vkbasalt: false },
  power_profile: "balanced",
  enabled_platforms: ["steam", "epic", "gog", "amazon", "lutris"],
  steam_api_key: null,
  steam_id64: null,
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
};
