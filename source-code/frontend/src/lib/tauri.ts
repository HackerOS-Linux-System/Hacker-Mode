import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Odpowiednik `commands::stores::Platform` po stronie Rust. */
export type Platform = "steam" | "epic" | "gog" | "amazon" | "lutris";

/** Odpowiednik `commands::stores::Game`. */
export interface Game {
  id: string;
  title: string;
  platform: Platform;
  installed: boolean;
  install_dir: string | null;
  cover_path: string | null;
  playtime_minutes: number | null;
}

export interface GamingTools {
  gamescope: boolean;
  mangohud: boolean;
  vkbasalt: boolean;
}

export interface Settings {
  language: string;
  theme: string;
  wrapper_mode_enabled: boolean;
  gaming_tools: GamingTools;
  power_profile: "power_saving" | "balanced" | "performance" | string;
  enabled_platforms: string[];
  steam_api_key: string | null;
  steam_id64: string | null;
}

export interface ActionResult {
  ok: boolean;
  message: string;
}

export interface GameDetails {
  description: string;
  screenshots: string[];
}

export interface SteamSearchResult {
  appid: number;
  name: string;
  cover_url: string | null;
  price: string | null;
}

export interface LoginFlow {
  url: string | null;
  instructions: string;
  needs_code: boolean;
}

/** Cienka, typowana warstwa nad `invoke(...)`, żeby reszta appki nigdy nie
 * odwoływała się do nazw komend jako "gołych" stringów. */
export const api = {
  listGames: () => invoke<Game[]>("list_games"),

  launchGame: (platform: Platform, gameId: string) =>
    invoke<void>("launch_game", { platform, gameId }),

  launchStoreClient: (name: "steam" | "heroic" | "hyperplay" | "lutris") =>
    invoke<void>("launch_store_client", { name }),

  restartApps: () => invoke<{ killed: string[] }>("restart_apps"),

  installGame: (platform: Platform, gameId: string) =>
    invoke<void>("install_game", { platform, gameId }),

  uninstallGame: (platform: Platform, gameId: string) =>
    invoke<void>("uninstall_game", { platform, gameId }),

  fetchGameDetails: (platform: Platform, gameId: string) =>
    invoke<GameDetails | null>("fetch_game_details", { platform, gameId }),

  searchSteamStore: (query: string) => invoke<SteamSearchResult[]>("search_steam_store", { query }),

  openSteamStorePage: (appid: number) => invoke<void>("open_steam_store_page", { appid }),

  storeIsLoggedIn: (platform: Platform) => invoke<boolean>("store_is_logged_in", { platform }),

  storeLoginStart: (platform: Platform) => invoke<LoginFlow>("store_login_start", { platform }),

  storeLoginSubmit: (platform: Platform, code: string) =>
    invoke<void>("store_login_submit", { platform, code }),

  openStoreLoginWindow: (platform: Platform) =>
    invoke<void>("open_store_login_window", { platform }),

  submitStoreLoginCode: (platform: Platform, code: string) =>
    invoke<void>("submit_store_login_code", { platform, code }),

  onStoreLoginFinished: (cb: (platform: Platform, ok: boolean) => void): Promise<UnlistenFn> =>
    listen<{ platform: Platform; ok: boolean }>("hacker-mode://store-login-finished", (e) =>
      cb(e.payload.platform, e.payload.ok),
    ),

  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),

  isDevMode: () => invoke<boolean>("is_dev_mode"),

  audioAction: (action: "increase" | "decrease" | "toggle_mute") =>
    invoke<ActionResult>("audio_action", { action }),

  displayAction: (action: "increase" | "decrease") =>
    invoke<ActionResult>("display_action", { action }),

  networkAction: (
    action: "toggle_wifi" | "list_wifi" | "connect_wifi",
    ssid?: string,
    password?: string,
  ) => invoke<ActionResult>("network_action", { action, ssid, password }),

  bluetoothAction: (action: "scan" | "pair", macAddress?: string) =>
    invoke<ActionResult>("bluetooth_action", { action, macAddress }),

  powerAction: (
    action: "power_saving" | "balanced" | "performance" | "shutdown" | "restart" | "sleep",
  ) => invoke<ActionResult>("power_action", { action }),

  switchToDesktopSession: (session: "plasma" | "gnome") =>
    invoke<void>("switch_to_desktop_session", { session }),

  /** Nasłuchuje na event emitowany przez backend po zamknięciu uruchomionej
   * gry/aplikacji (`hacker-mode://app-closed`). */
  onAppClosed: (cb: (label: string) => void): Promise<UnlistenFn> =>
    listen<{ label: string }>("hacker-mode://app-closed", (e) => cb(e.payload.label)),

  onInstallProgress: (cb: (label: string, line: string, percent: number | null) => void): Promise<UnlistenFn> =>
    listen<{ label: string; line: string; percent: number | null }>("hacker-mode://install-progress", (e) =>
      cb(e.payload.label, e.payload.line, e.payload.percent),
    ),

  onInstallFinished: (cb: (label: string, ok: boolean) => void): Promise<UnlistenFn> =>
    listen<{ label: string; ok: boolean }>("hacker-mode://install-finished", (e) =>
      cb(e.payload.label, e.payload.ok),
    ),
};
