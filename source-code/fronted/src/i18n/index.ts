export type Lang = "pl" | "en";

export const translations = {
  pl: {
    title: "Hacker Mode",
    settings: "Ustawienia",
    hacker_menu: "Menu",
    audio: "Dźwięk",
    display: "Ekran",
    network: "Sieć",
    power: "Zasilanie",
    general: "Ogólne",
    gaming_tools: "Narzędzia do gier",
    wifi_settings: "Ustawienia Wi-Fi",
    bluetooth: "Bluetooth",
    increase_volume: "Zwiększ głośność",
    decrease_volume: "Zmniejsz głośność",
    toggle_mute: "Wycisz / wyłącz wyciszenie",
    increase_brightness: "Zwiększ jasność",
    decrease_brightness: "Zmniejsz jasność",
    toggle_theme: "Zmień motyw",
    toggle_wifi: "Włącz / wyłącz Wi-Fi",
    connect: "Połącz",
    scan: "Skanuj",
    pair: "Paruj",
    close: "Zamknij",
    power_saving: "Oszczędzanie energii",
    balanced: "Zrównoważony",
    performance: "Wydajność",
    enable_gamescope: "Włącz Gamescope",
    enable_mangohud: "Włącz MangoHud",
    enable_vkbasalt: "Włącz vkBasalt",
    library: "Biblioteka",
    store: "Sklepy",
    home: "Start",
    app_not_installed: "Ta aplikacja nie jest zainstalowana.",
    no_internet: "Brak połączenia z internetem.",
    launch_cooldown: "Poczekaj {seconds}s przed ponownym uruchomieniem „{app}”.",
    wrapper_mode: "Tryb wrapper (zamykaj Hacker Mode na czas gry)",
    shutdown: "Wyłącz",
    restart: "Uruchom ponownie",
    sleep: "Uśpij",
    switch_desktop: "Przełącz na pulpit",
    restart_apps: "Zrestartuj aplikacje",
    no_games_found: "Nie znaleziono żadnych gier. Zainstaluj i zaloguj się do sklepu, aby zobaczyć bibliotekę.",
    installed: "Zainstalowana",
  },
  en: {
    title: "Hacker Mode",
    settings: "Settings",
    hacker_menu: "Menu",
    audio: "Audio",
    display: "Display",
    network: "Network",
    power: "Power",
    general: "General",
    gaming_tools: "Gaming tools",
    wifi_settings: "Wi-Fi settings",
    bluetooth: "Bluetooth",
    increase_volume: "Increase volume",
    decrease_volume: "Decrease volume",
    toggle_mute: "Toggle mute",
    increase_brightness: "Increase brightness",
    decrease_brightness: "Decrease brightness",
    toggle_theme: "Toggle theme",
    toggle_wifi: "Toggle Wi-Fi",
    connect: "Connect",
    scan: "Scan",
    pair: "Pair",
    close: "Close",
    power_saving: "Power saving",
    balanced: "Balanced",
    performance: "Performance",
    enable_gamescope: "Enable Gamescope",
    enable_mangohud: "Enable MangoHud",
    enable_vkbasalt: "Enable vkBasalt",
    library: "Library",
    store: "Stores",
    home: "Home",
    app_not_installed: "This application is not installed.",
    no_internet: "No internet connection.",
    launch_cooldown: "Wait {seconds}s before launching \u201e{app}\u201c again.",
    wrapper_mode: "Wrapper mode (close Hacker Mode while playing)",
    shutdown: "Shut down",
    restart: "Restart",
    sleep: "Sleep",
    switch_desktop: "Switch to desktop",
    restart_apps: "Restart apps",
    no_games_found: "No games found. Install and log in to a store to see your library.",
    installed: "Installed",
  },
} as const;

export type TranslationKey = keyof typeof translations.pl;

export function getText(
  lang: Lang,
  key: TranslationKey,
  vars?: Record<string, string | number>,
): string {
  let text: string = translations[lang][key] ?? translations.en[key] ?? key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) {
      text = text.replace(`{${k}}`, String(v));
    }
  }
  return text;
}

export function detectLang(): Lang {
  const nav = typeof navigator !== "undefined" ? navigator.language : "en";
  return nav?.startsWith("pl") ? "pl" : "en";
}
