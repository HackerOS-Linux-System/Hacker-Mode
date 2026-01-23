import { SystemState, WifiNetwork, BatteryManager } from '../types';
import { MOCK_WIFI_NETWORKS } from '../constants';

// NOTE: In Electron with nodeIntegration:true, we can use window.require to get Node modules
// This prevents webpack/vite from trying to bundle 'child_process' for the browser.
let exec: any;
try {
    if (window.require) {
        exec = window.require('child_process').exec;
    }
} catch (e) {
    console.log("Not running in Electron or nodeIntegration disabled.");
}

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

    private batteryManager: BatteryManager | null = null;

    constructor() {
        this.initRealDataListeners();
    }

    private async initRealDataListeners() {
        if (typeof navigator !== 'undefined' && 'getBattery' in navigator) {
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
        if (typeof window !== 'undefined') {
            window.addEventListener('online', () => this.updateNetworkState());
            window.addEventListener('offline', () => this.updateNetworkState());
        }
    }

    private updateBatteryState(battery: BatteryManager) {
        this.state.batteryLevel = Math.round(battery.level * 100);
        this.state.batteryCharging = battery.charging;
    }

    private updateNetworkState() {
        if (typeof navigator !== 'undefined') {
            const conn = (navigator as any).connection;
            this.state.networkType = conn ? conn.type : (navigator.onLine ? 'wifi' : 'none');
            if (!navigator.onLine) this.state.wifiEnabled = false;
        }
    }

    async getState(): Promise<SystemState> {
        if (this.batteryManager) this.updateBatteryState(this.batteryManager);
        this.updateNetworkState();

        return new Promise((resolve) => {
            setTimeout(() => resolve({ ...this.state }), 50);
        });
    }

    private executeCommand(command: string) {
        console.log(`[EXEC] ${command}`);
        if (exec) {
            exec(command, (error: any, stdout: string, stderr: string) => {
                if (error) {
                    console.error(`Error executing ${command}:`, error);
                    return;
                }
                if (stderr) console.error(`Stderr: ${stderr}`);
                console.log(`Stdout: ${stdout}`);
            });
        } else {
            console.warn("Cannot execute command: Not in Electron environment.");
        }
    }

    // --- Actions ---

    async setVolume(value: number): Promise<void> {
        this.state.volume = Math.max(0, Math.min(100, value));
        this.executeCommand(`pactl set-sink-volume @DEFAULT_SINK@ ${this.state.volume}%`);
    }

    async toggleMute(): Promise<void> {
        this.state.isMuted = !this.state.isMuted;
        this.executeCommand(`pactl set-sink-mute @DEFAULT_SINK@ toggle`);
    }

    async setBrightness(value: number): Promise<void> {
        this.state.brightness = Math.max(0, Math.min(100, value));
        this.executeCommand(`brightnessctl set ${this.state.brightness}%`);
    }

    async toggleWifi(): Promise<boolean> {
        this.state.wifiEnabled = !this.state.wifiEnabled;
        const stateStr = this.state.wifiEnabled ? 'on' : 'off';
        this.executeCommand(`nmcli radio wifi ${stateStr}`);
        return this.state.wifiEnabled;
    }

    async scanWifi(): Promise<WifiNetwork[]> {
        this.executeCommand(`nmcli -t -f SSID,SIGNAL dev wifi list`);
        // Parsing logic would go here in a real implementation
        return new Promise((resolve) => {
            setTimeout(() => {
                if (!this.state.wifiEnabled) return resolve([]);
                resolve(MOCK_WIFI_NETWORKS);
            }, 1500);
        });
    }

    async connectWifi(ssid: string, password?: string): Promise<boolean> {
        this.executeCommand(`nmcli dev wifi connect "${ssid}" password "${password ? '***' : ''}"`);
        return new Promise((resolve) => setTimeout(() => resolve(true), 2000));
    }

    async setPowerProfile(profile: SystemState['powerProfile']): Promise<void> {
        this.state.powerProfile = profile;
        this.executeCommand(`powerprofilesctl set ${profile}`);
    }

    async launchApp(command: string): Promise<void> {
        // Adding nohup and & to detach process, ensuring it runs independent of the launcher
        this.executeCommand(`nohup ${command} >/dev/null 2>&1 &`);
    }

    async systemAction(action: string): Promise<void> {
        const commands: Record<string, string> = {
            'shutdown': 'systemctl poweroff',
            'restart': 'systemctl reboot',
            'sleep': 'systemctl suspend',
            'switch_plasma': 'qdbus org.kde.ksmserver /KSMServer logout 0 0 0'
        };
        this.executeCommand(commands[action] || action);
    }
}

export const systemService = new SystemService();
