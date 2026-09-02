import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * BUGFIX: okładki gier trzymane lokalnie na dysku (cache Epic/GOG/Amazon w
 * `cover_cache.rs`, lokalny cache Steam/Lutrisa) mają `cover_path` w
 * postaci zwykłej, absolutnej ścieżki systemu plików (np.
 * `/home/user/.cache/hacker-mode/covers/gog-123.jpg`) — Tauri v2 nie
 * pozwala wczytać takiej ścieżki bezpośrednio jako `<img src>` (webview
 * nie ma dostępu do `file://`), tylko przez wbudowany protokół `asset:`,
 * do którego trzeba surową ścieżkę przepuścić przez `convertFileSrc()`.
 * Wcześniej `GameCard.tsx`/`GameDetail.tsx` wstawiały `cover_path` wprost
 * jako `src` — więc w REALNEJ apce Tauri (nie w `vite dev` w zwykłej
 * przeglądarce, gdzie tego nie widać) lokalne okładki najpewniej w ogóle
 * się nie renderowały. Zdalne adresy (http/https — np. Steam CDN dla gier
 * tylko posiadanych, patrz `steam::owned_game_cdn_cover_url`, albo ręczne
 * nadpisanie okładki wklejonym linkiem) trzeba zostawić bez zmian —
 * `convertFileSrc` na URL-u zwróciłby coś bezsensownego.
 *
 * Wymaga włączonego `app.security.assetProtocol` w `tauri.conf.json`
 * (patrz zmiana obok) ze scope'em obejmującym katalogi cache.
 */
export function resolveCoverSrc(coverPath: string): string {
  if (/^(https?:|data:|blob:)/i.test(coverPath)) return coverPath;
  return convertFileSrc(coverPath);
}

/** Odpowiednik `commands::stores::Platform` po stronie Rust. */
export type Platform = "steam" | "epic" | "gog" | "amazon" | "lutris" | "ea" | "battlenet";

/** Odpowiednik `Platform::label()` po stronie Rust — czytelne nazwy do
 * wyświetlenia (zamiast surowego sluga typu "battlenet" w badge'ach). */
export const PLATFORM_LABELS: Record<Platform, string> = {
  steam: "Steam",
  epic: "Epic Games",
  gog: "GOG",
  amazon: "Amazon Games",
  lutris: "Lutris",
  ea: "EA app",
  battlenet: "Battle.net",
};

/** Platformy, dla których Hacker Mode zarządza logowaniem z własnego UI
 * (`StoreLogin.tsx`) — odpowiednik `Platform::supports_managed_login()` po
 * stronie Rust. Lutris/EA app/Battle.net logują się same, wewnątrz swojego
 * okna — nie ma tu nic do "zalogowania" z perspektywy Hacker Mode. */
export const MANAGED_LOGIN_PLATFORMS: Platform[] = ["steam", "epic", "gog", "amazon"];

/** Platformy, dla których Hacker Mode potrafi zbudować komendę
 * odinstalowania gry — Lutris/EA app/Battle.net jej nie obsługują (patrz
 * `StoreProvider::uninstall_command`, domyślna implementacja zwracająca
 * błąd), więc `GameDetail.tsx` nie pokazuje dla nich przycisku "Odinstaluj". */
export const UNINSTALL_SUPPORTED_PLATFORMS: Platform[] = ["steam", "epic", "gog", "amazon", "lutris", "ea", "battlenet"];

/** Platformy, dla których Hacker Mode potrafi pokazać/zmienić wersję
 * Proton (Steam) / Wine (Lutris) przypisaną do gry — patrz `compat_tools.rs`.
 * Epic/GOG/Amazon/EA/Battle.net nie używają Proton/Wine w sposób, którym
 * dałoby się tu zarządzać (Epic/GOG/Amazon idą przez `legendary`/`gogdl`/
 * `nile`, które same zarządzają swoim środowiskiem uruchomieniowym; EA/
 * Battle.net działają w ręcznie utworzonym prefiksie Wine, który Hacker
 * Mode tylko uruchamia, nie zarządza jego wersją). */
export const COMPAT_TOOL_SUPPORTED_PLATFORMS: Platform[] = ["steam", "lutris"];

/** ProtonDB ocenia WYŁĄCZNIE Steam AppID — patrz `protondb.rs`. */
export const PROTONDB_SUPPORTED_PLATFORMS: Platform[] = ["steam"];

/** Platformy, dla których śledzenie stanu "gra aktualnie działa"
 * (`onGameLaunched`/`onGameExited`/`isGameRunning`/`stopGame`) jest
 * WIARYGODNE — czyli proces, który Hacker Mode faktycznie uruchamia i
 * czeka na jego zakończenie, jest tym samym procesem co realna sesja
 * gry, a nie tylko poleceniem przekazującym żądanie gdzie indziej.
 *
 * Steam jest CELOWO wykluczony: `steam -applaunch <appid>` (patrz
 * `steam.rs::launch_command`), gdy klient Steam już działa, kończy się
 * niemal NATYCHMIAST po przekazaniu żądania działającej instancji przez
 * IPC — Hacker Mode widziałby to jako "gra się zakończyła" ułamek
 * sekundy po starcie, mimo że realna sesja dopiero się zaczyna. Pokazanie
 * tu wskaźnika/przycisku "Zatrzymaj" dla Steam byłoby więc aktywnie
 * mylące, nie tylko niedokładne. */
export const RELIABLE_RUNNING_STATE_PLATFORMS: Platform[] = ["epic", "gog", "amazon", "lutris", "ea", "battlenet"];

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

/** Odpowiednik `commands::stores::PlatformWarning`. */
export interface PlatformWarning {
  platform: Platform;
  message: string;
}

/** Odpowiednik `commands::stores::LibraryLoadResult`. */
export interface LibraryLoadResult {
  games: Game[];
  warnings: PlatformWarning[];
}

/** Odpowiednik `first_run::LauncherToolStatus`. */
export interface LauncherToolStatus {
  id: string;
  label: string;
  available: boolean;
}

export interface GamingTools {
  gamescope: boolean;
  mangohud: boolean;
  vkbasalt: boolean;
}

/** Odpowiednik `settings::NotificationSettings`. */
export interface NotificationSettings {
  on_install: boolean;
  on_game_exit: boolean;
  on_backup_error: boolean;
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
  /** Opcjonalny klucz API SteamGridDB (https://www.steamgriddb.com/profile/preferences/api)
   * — zapasowe źródło okładek dla EA app/Battle.net, patrz `Settings.tsx`
   * (panel "SteamGridDB") i backendowy moduł `steamgriddb.rs`. */
  steamgriddb_api_key: string | null;
  /** Odpowiednik `Settings::game_tags` — klucz `"<platform>:<id>"`, patrz
   * `cover_override_key` po stronie Rust (ten sam format klucza). */
  game_tags: Record<string, string[]>;
  /** Katalog na kopie zapasowe zapisów (patrz `cloud_saves.rs`). */
  cloud_saves_backup_dir: string | null;
  game_save_paths: Record<string, string>;
  game_controller_configs: Record<string, string>;
  /** Odpowiednik `Settings::custom_launch_prefix` — patrz
   * `launcher::wrap_with_custom_prefix`. */
  custom_launch_prefix: string | null;
  notifications: NotificationSettings;
  /** Ręczne nadpisanie ścieżki do prefiksu Wine z EA app, używane gdy
   * autodetekcja (domyślne lokalizacje Lutrisa) nic nie znajdzie. */
  ea_wine_prefix: string | null;
  /** To samo co `ea_wine_prefix`, ale dla Battle.net. */
  battlenet_wine_prefix: string | null;
  /** Odpowiednik `Settings::crash_detection_threshold_seconds` —
   * jedno, dzielone źródło prawdy dla progu "krótka sesja = pewnie
   * crash", czytane zarówno przez backend (`launcher.rs`), jak i przez
   * `GameCard.tsx`/`GameDetail.tsx` (wcześniej każde z tych trzech
   * miejsc miało własną, niezależnie wpisaną na sztywno stałą `8`). */
  crash_detection_threshold_seconds: number;
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

/** Wynik wyszukiwania w katalogu GOG — patrz `stores::gog::search_store`
 * po stronie backendu i przycisk "Szukaj w katalogu GOG…" w `Store.tsx`. */
export interface GogSearchResult {
  product_id: string;
  title: string;
  cover_url: string | null;
  slug: string | null;
}

/** Odpowiednik `protondb::ProtonDbSummary`. */
export interface ProtonDbSummary {
  tier: string;
  confidence: string;
  score: number;
  total: number;
}

/** Etykiety i kolory dla plakietki ProtonDB — mapowanie surowych wartości
 * `tier` zwracanych przez API (patrz `protondb.rs`) na coś czytelnego w
 * UI. Kolejność od najlepszej do najgorszej oceny. */
export const PROTONDB_TIER_INFO: Record<string, { label: string; color: string }> = {
  platinum: { label: "Platinum — działa bez modyfikacji", color: "#b4c7dc" },
  gold: { label: "Gold — działa po drobnych poprawkach", color: "#cfb53b" },
  silver: { label: "Silver — działa z pomniejszymi problemami", color: "#a8a9ad" },
  bronze: { label: "Bronze — działa, ale z problemami", color: "#cd7f32" },
  borked: { label: "Borked — nie działa", color: "var(--danger)" },
  pending: { label: "Pending — za mało zgłoszeń", color: "var(--text-muted)" },
};

/** Odpowiednik `stores::steam::Achievement`. */
export interface Achievement {
  api_name: string;
  display_name: string;
  description: string;
  achieved: boolean;
  unlock_time: number;
  icon_url: string;
}

/** Odpowiednik `commands::CompatToolOptionDto`. */
export interface CompatToolOptionDto {
  value: string;
  label: string;
}

/** Odpowiednik `commands::CompatToolOptions`. */
export interface CompatToolOptions {
  options: CompatToolOptionDto[];
  current: string | null;
}

/** Odpowiednik `cloud_saves::BackupEntry`. */
export interface BackupEntry {
  file_name: string;
  created_at: number;
  size_bytes: number;
}

/** Odpowiednik `controllers::ConnectedController`. */
export interface ConnectedController {
  name: string;
  handler: string;
}

/** Odpowiednik `playtime::SessionEntry`. */
export interface SessionEntry {
  at: number;
  minutes: number;
}

/** Odpowiednik `external_sources::ExternalApp`. */
export interface ExternalApp {
  source: string;
  name: string;
}

export interface LoginFlow {
  url: string | null;
  instructions: string;
  needs_code: boolean;
}

/** Cienka, typowana warstwa nad `invoke(...)`, żeby reszta appki nigdy nie
 * odwoływała się do nazw komend jako "gołych" stringów. */
export const api = {
  listGames: () => invoke<LibraryLoadResult>("list_games"),

  setGameCover: (platform: Platform, gameId: string, coverUrl: string | null) =>
    invoke<void>("set_game_cover", { platform, gameId, coverUrl }),

  launchGame: (platform: Platform, gameId: string) =>
    invoke<void>("launch_game", { platform, gameId }),

  launchStoreClient: (name: "steam" | "heroic" | "hyperplay" | "lutris" | "ea" | "battlenet") =>
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

  searchGogStore: (query: string) => invoke<GogSearchResult[]>("search_gog_store", { query }),

  openGogStorePage: (slug: string) => invoke<void>("open_gog_store_page", { slug }),

  fetchProtondbRating: (appid: number) => invoke<ProtonDbSummary | null>("fetch_protondb_rating", { appid }),

  fetchSteamAchievements: (appid: string) => invoke<Achievement[]>("fetch_steam_achievements", { appid }),

  getCompatToolOptions: (platform: Platform, gameId: string) =>
    invoke<CompatToolOptions>("get_compat_tool_options", { platform, gameId }),

  setCompatTool: (platform: Platform, gameId: string, value: string) =>
    invoke<string | null>("set_compat_tool", { platform, gameId, value }),

  setGameTags: (platform: Platform, gameId: string, tags: string[]) =>
    invoke<void>("set_game_tags", { platform, gameId, tags }),

  getPlaytimeLastPlayed: () => invoke<Record<string, number>>("get_playtime_last_played"),

  setGameSavePath: (platform: Platform, gameId: string, path: string | null) =>
    invoke<void>("set_game_save_path", { platform, gameId, path }),

  backupGameSave: (platform: Platform, gameId: string) =>
    invoke<BackupEntry>("backup_game_save", { platform, gameId }),

  listGameSaveBackups: (platform: Platform, gameId: string) =>
    invoke<BackupEntry[]>("list_game_save_backups", { platform, gameId }),

  restoreGameSaveBackup: (platform: Platform, gameId: string, backupFileName: string) =>
    invoke<void>("restore_game_save_backup", { platform, gameId, backupFileName }),

  listConnectedControllers: () => invoke<ConnectedController[]>("list_connected_controllers"),

  setGameControllerConfig: (platform: Platform, gameId: string, config: string | null) =>
    invoke<void>("set_game_controller_config", { platform, gameId, config }),

  getGameSessionHistory: (platform: Platform, gameId: string) =>
    invoke<SessionEntry[]>("get_game_session_history", { platform, gameId }),

  exportSettings: () => invoke<string>("export_settings"),

  importSettings: (json: string) => invoke<void>("import_settings", { json }),

  listExternalApps: () => invoke<ExternalApp[]>("list_external_apps"),

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

  /** Nasłuchuje na uruchomienie/zakończenie KONKRETNEJ gry — w
   * przeciwieństwie do `onAppClosed` (dowolna zamknięta aplikacja,
   * łącznie z samym otwarciem klienta sklepu), te dwa eventy są emitowane
   * WYŁĄCZNIE dla realnych uruchomień gry (patrz `launcher.rs::run_wrapped`,
   * `playtime_key`). Używane przez `GameCard`/`GameDetail` do pokazania
   * trwałego stanu "w trakcie gry" i przycisku "Zatrzymaj" zamiast tylko
   * krótkiego spinnera przy starcie. */
  onGameLaunched: (cb: (platform: Platform, gameId: string) => void): Promise<UnlistenFn> =>
    listen<{ platform: Platform; gameId: string }>("hacker-mode://game-launched", (e) =>
      cb(e.payload.platform, e.payload.gameId),
    ),

  /** `secondsRan` pozwala UI odróżnić "gra normalnie działała jakiś czas"
   * od "proces zakończył się niemal natychmiast" (możliwy crash przy
   * starcie) — patrz heurystyka w `GameCard.tsx`. Dla Steam ten sygnał
   * jest MYLĄCY (patrz `AppState::running_games` po wyjaśnienie) i
   * celowo ignorowany przez UI dla tej platformy. */
  onGameExited: (cb: (platform: Platform, gameId: string, ok: boolean, secondsRan: number) => void): Promise<UnlistenFn> =>
    listen<{ platform: Platform; gameId: string; ok: boolean; secondsRan: number }>(
      "hacker-mode://game-exited",
      (e) => cb(e.payload.platform, e.payload.gameId, e.payload.ok, e.payload.secondsRan),
    ),

  stopGame: (platform: Platform, gameId: string, force: boolean) =>
    invoke<void>("stop_game", { platform, gameId, force }),

  isGameRunning: (platform: Platform, gameId: string) => invoke<boolean>("is_game_running", { platform, gameId }),

  setCustomLaunchPrefix: (prefix: string | null) => invoke<void>("set_custom_launch_prefix", { prefix }),

  setCrashDetectionThreshold: (seconds: number) => invoke<void>("set_crash_detection_threshold", { seconds }),

  setNotificationSettings: (notifications: NotificationSettings) =>
    invoke<void>("set_notification_settings", { notifications }),

  // --- Ekran powitalny pierwszego uruchomienia ---------------------------

  isFirstRun: () => invoke<boolean>("is_first_run"),
  markFirstRunComplete: () => invoke<void>("mark_first_run_complete"),
  firstRunToolStatus: () => invoke<LauncherToolStatus[]>("first_run_tool_status"),
  installLauncherTools: () => invoke<void>("install_launcher_tools"),

  onFirstRunProgress: (cb: (line: string) => void): Promise<UnlistenFn> =>
    listen<{ line: string }>("hacker-mode://first-run-progress", (e) => cb(e.payload.line)),

  onFirstRunFinished: (cb: (ok: boolean) => void): Promise<UnlistenFn> =>
    listen<{ ok: boolean }>("hacker-mode://first-run-finished", (e) => cb(e.payload.ok)),
};
