import React, { useState, useEffect } from 'react';
import { Wifi, Volume2, Sun, Monitor, Battery, Settings as SettingsIcon, Gamepad2, Globe, ChevronLeft } from 'lucide-react';
import { Translation, SystemState, WifiNetwork, Language } from '../types';
import { systemService } from '../services/systemService';
import { Slider } from './ui/Slider';
import { Toggle } from './ui/Toggle';
import { Button } from './ui/Button';

interface SettingsProps {
    t: Translation;
    lang: Language;
    onLanguageChange: (lang: Language) => void;
    onBack: () => void;
}

type SettingsTab = 'general' | 'audio' | 'network' | 'display' | 'power' | 'gaming';

export const Settings: React.FC<SettingsProps> = ({ t, lang, onLanguageChange, onBack }) => {
    const [activeTab, setActiveTab] = useState<SettingsTab>('audio');
    const [systemState, setSystemState] = useState<SystemState | null>(null);
    const [networks, setNetworks] = useState<WifiNetwork[]>([]);
    const [scanning, setScanning] = useState(false);
    const [wifiPassword, setWifiPassword] = useState('');
    const [selectedNetwork, setSelectedNetwork] = useState<string | null>(null);

    useEffect(() => {
        // Initial fetch
        systemService.getState().then(setSystemState);
    }, []);

    const handleScanWifi = async () => {
        setScanning(true);
        const results = await systemService.scanWifi();
        setNetworks(results);
        setScanning(false);
    };

    // Only scan if Network tab is active and wifi is enabled
    useEffect(() => {
        if (activeTab === 'network' && systemState?.wifiEnabled) {
            handleScanWifi();
        }
    }, [activeTab, systemState?.wifiEnabled]);

    const updateState = (key: keyof SystemState, value: any) => {
        if (!systemState) return;
        const newState = { ...systemState, [key]: value };
        setSystemState(newState);

        // Side effects for API calls
        if (key === 'volume') systemService.setVolume(value);
        if (key === 'brightness') systemService.setBrightness(value);
        if (key === 'powerProfile') systemService.setPowerProfile(value);
        if (key === 'wifiEnabled') systemService.toggleWifi();
        // ... add others
    };

        if (!systemState) return <div className="flex h-full items-center justify-center">Loading settings...</div>;

        const renderSidebarItem = (id: SettingsTab, label: string, Icon: React.ElementType) => (
            <button
            onClick={() => setActiveTab(id)}
            className={`w-full flex items-center gap-4 px-6 py-4 text-lg font-medium transition-all ${
                activeTab === id
                ? 'bg-steam-blue text-white shadow-[inset_4px_0_0_0_white]'
                : 'text-gray-400 hover:bg-white/5 hover:text-white'
            }`}
            >
            <Icon size={24} />
            {label}
            </button>
        );

        return (
            <div className="flex h-full w-full bg-[#15191e]">
            {/* Sidebar */}
            <div className="w-80 bg-[#0f1014] flex flex-col pt-20 border-r border-gray-800">
            <div className="px-6 mb-8">
            <Button onClick={onBack} variant="ghost" className="w-full justify-start pl-0 text-gray-400 hover:text-white">
            <ChevronLeft /> {t.back}
            </Button>
            <h1 className="text-3xl font-bold mt-2 text-white">{t.title}</h1>
            </div>

            <nav className="flex-1 overflow-y-auto">
            {renderSidebarItem('general', t.general, SettingsIcon)}
            {renderSidebarItem('audio', t.audio, Volume2)}
            {renderSidebarItem('display', t.display, Monitor)}
            {renderSidebarItem('network', t.network, Wifi)}
            {renderSidebarItem('power', t.power, Battery)}
            {renderSidebarItem('gaming', t.gaming_tools, Gamepad2)}
            </nav>
            </div>

            {/* Content Area */}
            <div className="flex-1 p-12 pt-20 overflow-y-auto bg-gradient-to-br from-[#15191e] to-[#101216]">
            <div className="max-w-3xl mx-auto space-y-8 animate-fade-in">

            {activeTab === 'audio' && (
                <div className="space-y-6">
                <h2 className="text-2xl font-light text-gray-400 border-b border-gray-700 pb-2">{t.audio}</h2>
                <div className="glass-panel p-6 rounded-lg">
                <Slider
                label={t.audio}
                value={systemState.volume}
                onChange={(v) => updateState('volume', v)}
                icon="Volume2"
                />
                <div className="mt-6 flex gap-4">
                <Button onClick={() => updateState('volume', Math.min(100, systemState.volume + 5))}>{t.increase_volume}</Button>
                <Button onClick={() => updateState('volume', Math.max(0, systemState.volume - 5))}>{t.decrease_volume}</Button>
                <Button variant={systemState.isMuted ? 'primary' : 'secondary'} onClick={() => systemService.toggleMute().then(() => updateState('isMuted', !systemState.isMuted))}>
                {t.toggle_mute}
                </Button>
                </div>
                </div>
                </div>
            )}

            {activeTab === 'display' && (
                <div className="space-y-6">
                <h2 className="text-2xl font-light text-gray-400 border-b border-gray-700 pb-2">{t.display}</h2>
                <div className="glass-panel p-6 rounded-lg">
                <Slider
                label={t.display}
                value={systemState.brightness}
                onChange={(v) => updateState('brightness', v)}
                icon="Sun"
                />
                <div className="mt-6">
                <Button onClick={() => updateState('theme', systemState.theme === 'dark' ? 'light' : 'dark')}>
                {t.toggle_theme}
                </Button>
                </div>
                </div>
                </div>
            )}

            {activeTab === 'network' && (
                <div className="space-y-6">
                <h2 className="text-2xl font-light text-gray-400 border-b border-gray-700 pb-2">{t.network}</h2>
                <Toggle
                label={t.toggle_wifi}
                checked={systemState.wifiEnabled}
                onChange={(checked) => updateState('wifiEnabled', checked)}
                />

                {systemState.wifiEnabled && (
                    <div className="glass-panel p-6 rounded-lg mt-4">
                    <div className="flex justify-between items-center mb-4">
                    <h3 className="text-lg font-medium">{t.wifi_settings}</h3>
                    <Button size="sm" onClick={handleScanWifi} disabled={scanning}>
                    {scanning ? 'Scanning...' : t.scan}
                    </Button>
                    </div>

                    <div className="space-y-2">
                    {networks.length === 0 && !scanning && (
                        <div className="p-4 text-center text-gray-500 italic">{t.no_networks}</div>
                    )}
                    {networks.map((net) => (
                        <div
                        key={net.ssid}
                        className={`p-4 rounded-md border border-gray-700 flex justify-between items-center cursor-pointer hover:bg-white/5 ${selectedNetwork === net.ssid ? 'bg-white/10 border-steam-blue' : 'bg-[#1e2126]'}`}
                        onClick={() => setSelectedNetwork(net.ssid)}
                        >
                        <div className="flex flex-col">
                        <span className="font-bold">{net.ssid}</span>
                        <span className="text-xs text-gray-400">{net.security ? 'Secure' : 'Open'} • {net.signal}%</span>
                        </div>
                        {net.connected && <span className="text-green-400 text-sm font-bold">Connected</span>}
                        </div>
                    ))}
                    </div>

                    {selectedNetwork && (
                        <div className="mt-6 pt-4 border-t border-gray-700 animate-slide-up">
                        <label className="block text-sm text-gray-400 mb-2">{t.password} for {selectedNetwork}</label>
                        <div className="flex gap-2">
                        <input
                        type="password"
                        className="flex-1 bg-black/50 border border-gray-600 rounded px-4 py-2 focus:border-steam-blue focus:outline-none text-white"
                        value={wifiPassword}
                        onChange={(e) => setWifiPassword(e.target.value)}
                        />
                        <Button variant="primary" onClick={() => systemService.connectWifi(selectedNetwork, wifiPassword)}>{t.connect}</Button>
                        </div>
                        </div>
                    )}
                    </div>
                )}
                </div>
            )}

            {activeTab === 'power' && (
                <div className="space-y-6">
                <h2 className="text-2xl font-light text-gray-400 border-b border-gray-700 pb-2">{t.power}</h2>
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {(['power-saving', 'balanced', 'performance'] as const).map(mode => (
                    <button
                    key={mode}
                    onClick={() => updateState('powerProfile', mode)}
                    className={`p-6 rounded-lg border-2 flex flex-col items-center gap-2 transition-all ${
                        systemState.powerProfile === mode
                        ? 'border-steam-blue bg-steam-blue/10 text-white'
                        : 'border-gray-700 bg-[#1e2126] text-gray-400 hover:border-gray-500'
                    }`}
                    >
                    <Battery size={32} className={systemState.powerProfile === mode ? 'text-steam-blue' : ''} />
                    <span className="font-bold uppercase tracking-wider text-sm">
                    {mode === 'power-saving' ? t.power_saving : mode === 'balanced' ? t.balanced : t.performance}
                    </span>
                    </button>
                ))}
                </div>
                </div>
            )}

            {activeTab === 'general' && (
                <div className="space-y-6">
                <h2 className="text-2xl font-light text-gray-400 border-b border-gray-700 pb-2">{t.general}</h2>
                <div className="glass-panel p-6 rounded-lg flex items-center justify-between">
                <div className="flex items-center gap-4">
                <Globe size={24} />
                <span className="text-lg">{t.language}</span>
                </div>
                <div className="flex bg-black/40 rounded-full p-1">
                <button
                className={`px-4 py-1 rounded-full transition-colors ${lang === 'en' ? 'bg-steam-blue text-white' : 'text-gray-400'}`}
                onClick={() => onLanguageChange('en')}
                >
                English
                </button>
                <button
                className={`px-4 py-1 rounded-full transition-colors ${lang === 'pl' ? 'bg-steam-blue text-white' : 'text-gray-400'}`}
                onClick={() => onLanguageChange('pl')}
                >
                Polski
                </button>
                </div>
                </div>
                </div>
            )}

            {activeTab === 'gaming' && (
                <div className="space-y-6">
                <h2 className="text-2xl font-light text-gray-400 border-b border-gray-700 pb-2">{t.gaming_tools}</h2>
                <div className="space-y-3">
                <Toggle
                label={t.enable_gamescope}
                checked={systemState.gamescopeEnabled}
                onChange={(v) => updateState('gamescopeEnabled', v)}
                />
                <Toggle
                label={t.enable_mangohud}
                checked={systemState.mangohudEnabled}
                onChange={(v) => updateState('mangohudEnabled', v)}
                />
                <Toggle
                label={t.enable_vkbasalt}
                checked={systemState.vkbasaltEnabled}
                onChange={(v) => updateState('vkbasaltEnabled', v)}
                />
                </div>
                </div>
            )}

            </div>
            </div>
            </div>
        );
};
