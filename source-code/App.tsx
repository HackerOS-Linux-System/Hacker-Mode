import React, { useState, useEffect } from 'react';
import { TRANSLATIONS, USER_IMAGE_PATH_SUFFIX, SYSTEM_ICON_PATH } from './constants';
import { ViewState, Language, AppConfig, AppId } from './types';
import { StatusBar } from './components/StatusBar';
import { Launcher } from './components/Launcher';
import { Settings } from './components/Settings';
import { PowerMenu } from './components/PowerMenu';
import { Settings as SettingsIcon, Power } from 'lucide-react';

const App: React.FC = () => {
    const [lang, setLang] = useState<Language>('en');
    const [view, setView] = useState<ViewState>(ViewState.LAUNCHER);
    const [userHome, setUserHome] = useState<string>('');

    // Detect language and user home
    useEffect(() => {
        const sysLang = navigator.language.split('-')[0];
        if (sysLang === 'pl') setLang('pl');

        // Attempt to get user home dir via Electron/Node
        try {
            if (window.require) {
                const os = window.require('os');
                setUserHome(os.homedir());
            }
        } catch (e) {
            console.warn("Could not determine user home directory (not in Electron?)");
        }
    }, []);

    const t = TRANSLATIONS[lang];

    // Helper to construct local file path.
    // In Electron with webSecurity: false, 'file://' allows absolute paths.
    const getUserImgPath = (filename: string) => {
        if (!userHome) return `file://${USER_IMAGE_PATH_SUFFIX}${filename}`; // Fallback or partial
            return `file://${userHome}${USER_IMAGE_PATH_SUFFIX}${filename}`;
    };

    const getSystemIconPath = (filename: string) => {
        return `file://${SYSTEM_ICON_PATH}${filename}`;
    }

    const apps: AppConfig[] = [
        {
            id: AppId.STEAM,
            name: t.launcher.steam,
            iconUrl: getUserImgPath('steam.png'),
            command: 'flatpak run com.valvesoftware.Steam'
        },
        {
            id: AppId.HEROIC,
            name: t.launcher.heroic,
            iconUrl: getUserImgPath('heroic-games-launcher.png'),
            command: 'heroic'
        },
        {
            id: AppId.HYPERPLAY,
            name: t.launcher.hyperplay,
            iconUrl: getUserImgPath('hyperplay.png'),
            command: 'hyperplay'
        },
        {
            id: AppId.LUTRIS,
            name: t.launcher.lutris,
            iconUrl: getUserImgPath('lutris.png'),
            command: 'flatpak run net.lutris.Lutris'
        },
        {
            id: AppId.HACKER_LAUNCHER,
            name: t.launcher.hacker_launcher,
            iconUrl: getSystemIconPath('Hacker_Launcher.png'),
            command: '/usr/share/HackerOS/Scripts/HackerOS-Apps/Hacker_Launcher'
        }
    ];

    return (
        <div className="relative w-screen h-screen overflow-hidden bg-steam-dark text-white selection:bg-steam-blue selection:text-white font-sans">

        {/* Background Ambience */}
        <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-blue-900/20 via-[#0f1014] to-[#0f1014] z-0 pointer-events-none" />

        {/* Top Bar */}
        <StatusBar />

        {/* Main Content Area */}
        <main className="absolute inset-0 pt-12 pb-20 z-10 overflow-hidden">
        {view === ViewState.LAUNCHER && (
            <div className="h-full animate-fade-in">
            <Launcher apps={apps} />
            </div>
        )}

        {view === ViewState.SETTINGS && (
            <div className="h-full w-full absolute inset-0 z-20 animate-slide-up bg-[#0f1014]">
            <Settings
            t={t}
            lang={lang}
            onLanguageChange={setLang}
            onBack={() => setView(ViewState.LAUNCHER)}
            />
            </div>
        )}
        </main>

        {/* Footer / Navigation Bar */}
        {view === ViewState.LAUNCHER && (
            <footer className="absolute bottom-0 w-full h-20 bg-gradient-to-t from-black to-transparent z-30 flex items-center justify-between px-12">
            <div className="flex gap-4">
            <button
            onClick={() => setView(ViewState.SETTINGS)}
            className="flex items-center gap-2 px-6 py-2 rounded-full bg-white/10 hover:bg-white/20 transition-all backdrop-blur-sm text-sm font-bold uppercase tracking-wide border border-white/5 hover:border-white/20"
            >
            <SettingsIcon size={18} />
            {t.settings}
            </button>
            </div>

            <div className="flex gap-4">
            <button
            onClick={() => setView(ViewState.POWER_MENU)}
            className="flex items-center gap-2 px-6 py-2 rounded-full bg-red-500/10 hover:bg-red-500/20 text-red-200 transition-all backdrop-blur-sm text-sm font-bold uppercase tracking-wide border border-red-500/20"
            >
            <Power size={18} />
            {t.hacker_menu}
            </button>
            </div>
            </footer>
        )}

        {/* Power Menu Overlay */}
        {view === ViewState.POWER_MENU && (
            <PowerMenu t={t} onClose={() => setView(ViewState.LAUNCHER)} />
        )}
        </div>
    );
};

export default App;
