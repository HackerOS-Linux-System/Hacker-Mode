export type Language = 'en' | 'pl';

export enum ViewState {
    LAUNCHER = 'LAUNCHER',
    SETTINGS = 'SETTINGS',
    POWER_MENU = 'POWER_MENU'
}

export enum AppId {
    STEAM = 'steam',
    HEROIC = 'heroic',
    HYPERPLAY = 'hyperplay',
    LUTRIS = 'lutris',
    HACKER_LAUNCHER = 'hacker_launcher'
}

export interface AppConfig {
    id: AppId;
    name: string;
    iconUrl: string;
    command: string;
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
    powerProfile: 'power-saving' | 'balanced' | 'performance';
    gamescopeEnabled: boolean;
    mangohudEnabled: boolean;
    vkbasaltEnabled: boolean;
    // Real Data
    batteryLevel: number;
    batteryCharging: boolean;
    networkType: string;
}

// Web Battery API Interfaces
export interface BatteryManager extends EventTarget {
    charging: boolean;
    chargingTime: number;
    dischargingTime: number;
    level: number;
    onchargingchange: ((this: BatteryManager, ev: Event) => any) | null;
    onlevelchange: ((this: BatteryManager, ev: Event) => any) | null;
}

declare global {
    interface Window {
        require: any;
    }

    interface Navigator {
        getBattery?: () => Promise<BatteryManager>;
        connection?: {
            effectiveType: string;
            rtt: number;
            downlink: number;
            saveData: boolean;
            type: string;
        };
    }
}

export interface Translation {
    title: string;
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
    power: string;
    power_saving: string;
    balanced: string;
    performance: string;
    general: string;
    gaming_tools: string;
    enable_gamescope: string;
    enable_mangohud: string;
    enable_vkbasalt: string;
    connect: string;
    scan: string;
    close: string;
    back: string;
    no_networks: string;
    no_selection: string;
    password: string;
    wifi_on: string;
    wifi_off: string;
    language: string;
    settings: string;
    hacker_menu: string;
    launcher: {
        steam: string;
        heroic: string;
        hyperplay: string;
        lutris: string;
        hacker_launcher: string;
    };
    system: {
        shutdown: string;
        restart: string;
        sleep: string;
        restart_apps: string;
        restart_sway: string;
        switch_plasma: string;
    }
}
