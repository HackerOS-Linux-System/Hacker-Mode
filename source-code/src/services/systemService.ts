import { invoke } from '@tauri-apps/api/core';
import { SystemState, WifiNetwork, DesktopEnvironment } from '../types';

interface BatteryManagerLocal {
  charging: boolean;
  level: number;
  addEventListener(type: string, listener: () => void): void;
}

class SystemService {
  private state: SystemState = {
    volume: 50, isMuted: false, brightness: 70, wifiEnabled: true,
    bluetoothEnabled: false, theme: 'dark', powerProfile: 'balanced',
    gamescopeEnabled: true, mangohudEnabled: false, vkbasaltEnabled: false,
    batteryLevel: 100, batteryCharging: false, networkType: 'unknown',
    closeOnLaunch: false, reopenOnExit: true, detectedDE: 'unknown',
  };
  private batteryManager: BatteryManagerLocal | null = null;
  private batteryListenersAttached = false;

  constructor() {
    this.initListeners();
    this.loadPersistedSettings();
  }

  private async loadPersistedSettings() {
    try {
      const saved = localStorage.getItem('hacker_mode_settings');
      if (saved) this.state = { ...this.state, ...JSON.parse(saved) as Partial<SystemState> };
    } catch { /* ignore */ }
  }

  private async persistSettings() {
    try {
      localStorage.setItem('hacker_mode_settings', JSON.stringify({
        volume: this.state.volume, brightness: this.state.brightness,
        powerProfile: this.state.powerProfile, gamescopeEnabled: this.state.gamescopeEnabled,
        mangohudEnabled: this.state.mangohudEnabled, vkbasaltEnabled: this.state.vkbasaltEnabled,
        closeOnLaunch: this.state.closeOnLaunch, reopenOnExit: this.state.reopenOnExit,
      }));
    } catch { /* ignore */ }
  }

  private async initListeners() {
    this.pollBattery();
    if ('getBattery' in navigator && !this.batteryListenersAttached) {
      try {
        const battery = await (navigator as unknown as { getBattery: () => Promise<BatteryManagerLocal> }).getBattery();
        this.batteryManager = battery;
        this.updateBatWeb(battery);
        battery.addEventListener('levelchange', () => this.updateBatWeb(battery));
        battery.addEventListener('chargingchange', () => this.updateBatWeb(battery));
        this.batteryListenersAttached = true;
      } catch { /* ignore */ }
    }
    this.updateNet();
    window.addEventListener('online', () => this.updateNet());
    window.addEventListener('offline', () => this.updateNet());
    this.detectDE();
  }

  private async pollBattery() {
    try {
      const bat = await invoke<{ level: number; charging: boolean; present: boolean }>('get_battery_info');
      if (bat.present) { this.state.batteryLevel = bat.level; this.state.batteryCharging = bat.charging; }
    } catch { /* ignore */ }
    setTimeout(() => this.pollBattery(), 5000);
  }

  private updateBatWeb(b: BatteryManagerLocal) {
    this.state.batteryLevel = Math.round(b.level * 100);
    this.state.batteryCharging = b.charging;
  }

  private updateNet() {
    const conn = (navigator as unknown as { connection?: { type: string } }).connection;
    this.state.networkType = conn ? conn.type : navigator.onLine ? 'wifi' : 'none';
    if (!navigator.onLine) this.state.wifiEnabled = false;
  }

  private async detectDE() {
    try { this.state.detectedDE = (await invoke<string>('detect_desktop_environment')) as DesktopEnvironment; }
    catch { /* ignore */ }
  }

  async getState(): Promise<SystemState> {
    if (this.batteryManager) this.updateBatWeb(this.batteryManager);
    this.updateNet();
    return { ...this.state };
  }

  async setVolume(v: number) {
    this.state.volume = Math.max(0, Math.min(100, v));
    try { await invoke('set_volume', { volume: this.state.volume }); } catch { /* ignore */ }
    await this.persistSettings();
  }

  async toggleMute() {
    this.state.isMuted = !this.state.isMuted;
    try { await invoke('toggle_mute'); } catch { /* ignore */ }
  }

  async setBrightness(v: number) {
    this.state.brightness = Math.max(0, Math.min(100, v));
    try { await invoke('set_brightness', { brightness: this.state.brightness }); } catch { /* ignore */ }
    await this.persistSettings();
  }

  async toggleWifi() {
    this.state.wifiEnabled = !this.state.wifiEnabled;
    try { await invoke('toggle_wifi', { enabled: this.state.wifiEnabled }); } catch { /* ignore */ }
    return this.state.wifiEnabled;
  }

  async scanWifi(): Promise<WifiNetwork[]> {
    try { return await invoke<WifiNetwork[]>('scan_wifi'); } catch { return []; }
  }

  async connectWifi(ssid: string, password?: string) {
    try { return await invoke<boolean>('connect_wifi', { ssid, password }); } catch { return false; }
  }

  async setPowerProfile(profile: SystemState['powerProfile']) {
    this.state.powerProfile = profile;
    try { await invoke('set_power_profile', { profile }); } catch { /* ignore */ }
    await this.persistSettings();
  }

  async updateSetting<K extends keyof SystemState>(key: K, value: SystemState[K]) {
    (this.state as unknown as Record<string, unknown>)[key as string] = value;
    await this.persistSettings();
  }

  async systemAction(action: string) { await invoke('system_action', { action }); }
  async runSystemUpdate(): Promise<string> { return await invoke<string>('run_system_update'); }
  async getProtonVersions(): Promise<ProtonVersion[]> { try { return await invoke<ProtonVersion[]>('get_proton_versions'); } catch { return []; } }
  async installProtonGE(downloadUrl: string, tagName: string) { await invoke('install_proton_ge', { downloadUrl, tagName }); }
  async uninstallProton(version: string) { await invoke('uninstall_proton', { version }); }
}

export interface ProtonVersion {
  id: string; name: string; type: string;
  installed: boolean; version: string; install_path?: string;
}

export const systemService = new SystemService();
