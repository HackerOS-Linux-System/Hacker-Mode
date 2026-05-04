import React, { useState, useEffect, useCallback } from 'react';
import { TRANSLATIONS } from './constants';
import { ViewState, Language, SystemState, StoreSection } from './types';
import { StatusBar } from './components/StatusBar';
import { Library } from './components/Library';
import { Store } from './components/Store';
import { Settings } from './components/Settings';
import { HackerMenu } from './components/HackerMenu';
import { AddGame } from './components/AddGame';
import { ProtonManager } from './components/ProtonManager';
import { VenvSetup } from './components/VenvSetup';
import { systemService } from './services/systemService';
import { Gamepad2, ShoppingCart, Menu } from 'lucide-react';

const DEFAULT_STATE: SystemState = {
  volume: 50, isMuted: false, brightness: 70, wifiEnabled: true,
  bluetoothEnabled: false, theme: 'dark', powerProfile: 'balanced',
  gamescopeEnabled: true, mangohudEnabled: false, vkbasaltEnabled: false,
  batteryLevel: 100, batteryCharging: false, networkType: 'unknown',
  closeOnLaunch: false, reopenOnExit: true, detectedDE: 'unknown',
};

const MAIN_VIEWS = new Set([ViewState.LIBRARY, ViewState.STORE]);

const App: React.FC = () => {
  const [lang, setLang]                   = useState<Language>('en');
  const [view, setView]                   = useState<ViewState>(ViewState.LIBRARY);
  const [settingsTab, setSettingsTab]     = useState<string>('audio');
  const [storeSection, setStoreSection]   = useState<StoreSection | undefined>(undefined);
  const [showMenu, setShowMenu]           = useState(false);
  const [showVenvSetup, setShowVenvSetup] = useState(false);
  const [systemState, setSystemState]     = useState<SystemState>(DEFAULT_STATE);
  const [libraryKey, setLibraryKey]       = useState(0);

  useEffect(() => {
    if (navigator.language.split('-')[0] === 'pl') setLang('pl');
  }, []);

  useEffect(() => {
    systemService.getState().then(setSystemState);
    const id = setInterval(() => {
      systemService.getState().then(s => setSystemState(p => ({ ...p, ...s })));
    }, 5000);
    return () => clearInterval(id);
  }, []);

  const handleKey = useCallback((e: KeyboardEvent) => {
    if (
      (e.metaKey && e.key === 'm') ||
      (e.metaKey && e.key === 'Tab') ||
      (e.ctrlKey && e.shiftKey && (e.key === 'M' || e.key === 'm'))
    ) {
      e.preventDefault();
      setShowMenu(p => !p);
      return;
    }
    if (e.key === 'Escape') {
      if (showMenu) { setShowMenu(false); return; }
      if (!MAIN_VIEWS.has(view)) setView(ViewState.LIBRARY);
    }
  }, [showMenu, view]);

  useEffect(() => {
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [handleKey]);

  const openSettings = (tab = 'audio') => {
    setSettingsTab(tab);
    setView(ViewState.SETTINGS);
    setShowMenu(false);
  };

  const openStore = (section?: StoreSection) => {
    setStoreSection(section);
    setView(ViewState.STORE);
    setShowMenu(false);
  };

  const t = TRANSLATIONS[lang];
  const isMainView = MAIN_VIEWS.has(view);

  if (showVenvSetup) {
    return (
      <div className="w-screen h-screen overflow-hidden bg-[#0f1014] text-white font-sans select-none flex flex-col">
        <StatusBar onOpenSettings={openSettings} />
        <div className="flex-1 overflow-hidden">
          <VenvSetup onDone={() => setShowVenvSetup(false)} />
        </div>
      </div>
    );
  }

  return (
    <div className="w-screen h-screen overflow-hidden bg-[#0f1014] text-white font-sans select-none flex flex-col">

      {/* Status bar — fixed 40px height */}
      <StatusBar onOpenSettings={openSettings} />

      {/* Main content — takes all space between status bar and nav bar */}
      <main className="flex-1 overflow-hidden min-h-0">
        {view === ViewState.LIBRARY && (
          <div className="h-full animate-fade-in" key={libraryKey}>
            <Library t={t} systemState={systemState} />
          </div>
        )}

        {view === ViewState.STORE && (
          <div className="h-full animate-fade-in">
            <Store t={t} initialSection={storeSection} />
          </div>
        )}

        {view === ViewState.SETTINGS && (
          <div className="h-full animate-fade-in">
            <Settings
              t={t} lang={lang} systemState={systemState}
              initialTab={settingsTab}
              onLanguageChange={setLang}
              onBack={() => setView(ViewState.LIBRARY)}
              onStateChange={(k, v) => setSystemState(p => ({ ...p, [k]: v }))}
            />
          </div>
        )}

        {view === ViewState.ADD_GAME && (
          <div className="h-full animate-fade-in">
            <AddGame
              onBack={() => setView(ViewState.LIBRARY)}
              onGameAdded={() => { setLibraryKey(k => k + 1); setView(ViewState.LIBRARY); }}
            />
          </div>
        )}

        {view === ViewState.PROTON_MANAGER && (
          <div className="h-full animate-fade-in">
            <ProtonManager onBack={() => setView(ViewState.LIBRARY)} />
          </div>
        )}
      </main>

      {/* Bottom nav bar — fixed 56px, always visible */}
      {isMainView && (
        <nav className="flex-shrink-0 h-14 flex items-center justify-between px-4 bg-[#080a0d] border-t border-gray-800">

          {/* LEFT — Hacker Menu button, always visible */}
          <button
            onClick={() => setShowMenu(true)}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-steam-blue hover:bg-steam-blueHover text-white text-sm font-bold transition-all active:scale-95"
          >
            <Menu size={15} />
            {t.hacker_menu}
          </button>

          {/* CENTER — Library / Store tabs */}
          <div className="flex items-center bg-[#1a1d24] rounded-lg border border-gray-800 overflow-hidden">
            {([
              { id: ViewState.LIBRARY, icon: Gamepad2,    key: 'library' as const },
              { id: ViewState.STORE,   icon: ShoppingCart, key: 'store'   as const },
            ]).map(({ id, icon: Icon, key }) => (
              <button
                key={id}
                onClick={() => { if (id === ViewState.STORE) setStoreSection(undefined); setView(id); }}
                className={`flex items-center gap-2 px-5 py-2 text-sm font-medium transition-colors ${view === id ? 'bg-steam-blue text-white' : 'text-gray-500 hover:text-white'}`}
              >
                <Icon size={14} />
                {t[key]}
              </button>
            ))}
          </div>

          {/* RIGHT — spacer */}
          <div className="w-32" />
        </nav>
      )}

      {/* Hacker Menu overlay */}
      {showMenu && (
        <HackerMenu
          t={t}
          detectedDE={systemState.detectedDE}
          onClose={() => setShowMenu(false)}
          onNavigate={v => { setView(v); setShowMenu(false); }}
          onOpenStore={openStore}
          onOpenSettings={openSettings}
          onOpenVenvSetup={() => { setShowMenu(false); setShowVenvSetup(true); }}
        />
      )}
    </div>
  );
};

export default App;
