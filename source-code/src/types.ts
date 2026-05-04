export type Language = 'en' | 'pl';

export enum ViewState {
  LIBRARY        = 'LIBRARY',
  STORE          = 'STORE',
  SETTINGS       = 'SETTINGS',
  ADD_GAME       = 'ADD_GAME',
  PROTON_MANAGER = 'PROTON_MANAGER',
}

export type StoreSection = 'epic' | 'gog' | 'amazon' | 'steam_store' | 'lutris_store';

export type GameSource =
  | 'steam'
  | 'epic'
  | 'gog'
  | 'amazon'
  | 'lutris'
  | 'manual';

export type PowerProfile = 'power-saving' | 'balanced' | 'performance';
export type DesktopEnvironment = 'plasma' | 'gnome' | 'xfce' | 'unknown';

export interface Game {
  id: string;
  name: string;
  source: GameSource;
  appId?: string;
  coverUrl?: string;
  backgroundUrl?: string;
  heroImageUrl?: string;
  iconUrl?: string;
  installed: boolean;
  installDir?: string;
  lastPlayed?: number;
  playTime?: number;
  runCommand?: string;
  description?: string;
  developer?: string;
  publisher?: string;
  releaseDate?: string;
  tags?: string[];
  size?: number;
  favorite?: boolean;
  // For legendary-managed games
  legendaryAppName?: string;
}

export interface StoreGame {
  id: string;
  title: string;
  developer?: string;
  description?: string;
  coverUrl?: string;
  backgroundUrl?: string;
  price?: string;
  originalPrice?: string;
  discount?: number;
  isFree?: boolean;
  source: GameSource;
  appId: string;
  owned?: boolean;
  installed?: boolean;
  tags?: string[];
}

export interface WifiNetwork {
  ssid: string;
  signal: number;
  security: boolean;
  connected?: boolean;
}

export interface SystemState {
  volume: number;
  isMuted: boolean;
  brightness: number;
  wifiEnabled: boolean;
  bluetoothEnabled: boolean;
  theme: 'dark' | 'light';
  powerProfile: PowerProfile;
  gamescopeEnabled: boolean;
  mangohudEnabled: boolean;
  vkbasaltEnabled: boolean;
  batteryLevel: number;
  batteryCharging: boolean;
  networkType: string;
  closeOnLaunch: boolean;
  reopenOnExit: boolean;
  detectedDE: DesktopEnvironment;
}

export interface Translation {
  title: string;
  library: string;
  store: string;
  settings: string;
  hacker_menu: string;
  back: string;
  close: string;
  search: string;
  audio: string;
  increase_volume: string;
  decrease_volume: string;
  toggle_mute: string;
  display: string;
  increase_brightness: string;
  decrease_brightness: string;
  toggle_theme: string;
  network: string;
  wifi_settings: string;
  toggle_wifi: string;
  bluetooth: string;
  connect: string;
  scan: string;
  no_networks: string;
  no_selection: string;
  password: string;
  wifi_on: string;
  wifi_off: string;
  power: string;
  power_saving: string;
  balanced: string;
  performance: string;
  general: string;
  language: string;
  gaming_tools: string;
  enable_gamescope: string;
  enable_mangohud: string;
  enable_vkbasalt: string;
  library_settings: string;
  close_on_launch: string;
  reopen_on_exit: string;
  proton_manager: string;
  install_proton: string;
  system_update: string;
  proton: string;
  add_game: string;
  system: {
    shutdown: string;
    restart: string;
    sleep: string;
    hibernate: string;
    logout: string;
    switch_desktop: string;
    switch_user: string;
  };
  play: string;
  install: string;
  uninstall: string;
  last_played: string;
  play_time: string;
  never_played: string;
  imported: string;
  all_games: string;
  installed_only: string;
  favorites: string;
  importing: string;
  import_complete: string;
  no_games: string;
  game_source: {
    steam: string;
    epic: string;
    gog: string;
    amazon: string;
    lutris: string;
    manual: string;
  };
}
