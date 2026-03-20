import { invoke } from '@tauri-apps/api/core';
import { SystemState, WifiNetwork } from '../types';

class SystemService {
    private state: SystemState = {
        volume: 50,
        isMuted: false,
        brightness: 70,
        wifiEnabled: true,
        bluetoothEnabled: false,
        theme: 'dark',
        powerProfile: 'balanced',
        gamescopeEnabled: true,
        mangohudEnabled: false,
        vkbasaltEnabled: false,
        batteryLevel: 100,
        batteryCharging: false,
        networkType: 'unknown'
    };

    private batteryManager: any = null;

    constructor() {
        this.initRealDataListeners();
    }

    private async initRealDataListeners() {
        if ('getBattery' in navigator) {
            try {
                const battery = await (navigator as any).getBattery();
                this.batteryManager = battery;
                this.updateBatteryState(battery);
                battery.addEventListener('levelchange', () => this.updateBatteryState(battery));
                battery.addEventListener('chargingchange', () => this.updateBatteryState(battery));
            } catch (e) {
                console.warn('Battery API not supported');
            }
        }

        this.updateNetworkState();
        window.addEventListener('online', () => this.updateNetworkState());
        window.addEventListener('offline', () => this.updateNetworkState());
    }

    private updateBatteryState(battery: any) {
        this.state.batteryLevel = Math.round(battery.level * 100);
        this.state.batteryCharging = battery.charging;
    }

    private updateNetworkState() {
        const conn = (navigator as any).connection;
        this.state.networkType = conn ? conn.type : (navigator.onLine ? 'wifi' : 'none');
        if (!navigator.onLine) this.state.wifiEnabled = false;
    }

    async getState(): Promise<SystemState> {
        if (this.batteryManager) this.updateBatteryState(this.batteryManager);
        this.updateNetworkState();
        return { ...this.state };
    }

    // --- Actions ---

    async setVolume(value: number): Promise<void> {
        this.state.volume = Math.max(0, Math.min(100, value));
        await invoke('set_volume', { volume: this.state.volume });
    }

    async toggleMute(): Promise<void> {
        this.state.isMuted = !this.state.isMuted;
        await invoke('toggle_mute');
    }

    async setBrightness(value: number): Promise<void> {
        this.state.brightness = Math.max(0, Math.min(100, value));
        await invoke('set_brightness', { brightness: this.state.brightness });
    }

    async toggleWifi(): Promise<boolean> {
        this.state.wifiEnabled = !this.state.wifiEnabled;
        await invoke('toggle_wifi', { enabled: this.state.wifiEnabled });
        return this.state.wifiEnabled;
    }

    async scanWifi(): Promise<WifiNetwork[]> {
        return await invoke('scan_wifi');
    }

    async connectWifi(ssid: string, password?: string): Promise<boolean> {
        return await invoke('connect_wifi', { ssid, password });
    }

    async setPowerProfile(profile: SystemState['powerProfile']): Promise<void> {
        this.state.powerProfile = profile;
        await invoke('set_power_profile', { profile });
    }

    async launchApp(command: string): Promise<void> {
        await invoke('launch_app', { command });
    }

    async systemAction(action: string): Promise<void> {
        await invoke('system_action', { action });
    }
}

export const systemService = new SystemService();
