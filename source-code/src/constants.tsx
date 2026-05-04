import { Translation } from './types';

// These paths will be used dynamically in App.tsx combined with os.homedir()
export const USER_IMAGE_PATH_SUFFIX = '/.hackeros/Hacker-Mode/images/';
export const SYSTEM_ICON_PATH = '/usr/share/HackerOS/ICONS/';

export const TRANSLATIONS: Record<'en' | 'pl', Translation> = {
    en: {
        title: "Quick Settings",
        audio: "Audio",
        increase_volume: "Volume Up",
        decrease_volume: "Volume Down",
        toggle_mute: "Mute",
        display: "Display",
        increase_brightness: "Brightness Up",
        decrease_brightness: "Brightness Down",
        toggle_theme: "Toggle Theme",
        network: "Network",
        wifi_settings: "Wi-Fi Networks",
        toggle_wifi: "Enable Wi-Fi",
        bluetooth: "Bluetooth",
        power: "Power Profile",
        power_saving: "Power Saver",
        balanced: "Balanced",
        performance: "Performance",
        general: "General",
        gaming_tools: "Gaming Tools",
        enable_gamescope: "Enable Gamescope",
        enable_mangohud: "Enable MangoHUD",
        enable_vkbasalt: "Enable vkBasalt",
        connect: "Connect",
        scan: "Scan Again",
        close: "Close",
        back: "Back",
        no_networks: "No networks found",
        no_selection: "Please select a network",
        password: "Password",
        wifi_on: "On",
        wifi_off: "Off",
        language: "Language",
        settings: "Settings",
        hacker_menu: "Power",
        launcher: {
            steam: "Steam",
            heroic: "Heroic Games Launcher",
            hyperplay: "HyperPlay",
            lutris: "Lutris",
            hacker_launcher: "Hacker Launcher"
        },
        system: {
            shutdown: "Shutdown",
            restart: "Restart",
            sleep: "Sleep",
            restart_apps: "Restart Apps",
            restart_sway: "Restart Interface",
            switch_plasma: "Switch to Desktop"
        }
    },
    pl: {
        title: "Szybkie Ustawienia",
        audio: "Dźwięk",
        increase_volume: "Głośniej",
        decrease_volume: "Ciszej",
        toggle_mute: "Wycisz",
        display: "Ekran",
        increase_brightness: "Jaśniej",
        decrease_brightness: "Ciemniej",
        toggle_theme: "Przełącz motyw",
        network: "Sieć",
        wifi_settings: "Sieci Wi-Fi",
        toggle_wifi: "Włącz Wi-Fi",
        bluetooth: "Bluetooth",
        power: "Profil zasilania",
        power_saving: "Oszczędzanie",
        balanced: "Zrównoważony",
        performance: "Wydajność",
        general: "Ogólne",
        gaming_tools: "Narzędzia do gier",
        enable_gamescope: "Włącz Gamescope",
        enable_mangohud: "Włącz MangoHUD",
        enable_vkbasalt: "Włącz vkBasalt",
        connect: "Połącz",
        scan: "Skanuj",
        close: "Zamknij",
        back: "Wróć",
        no_networks: "Nie znaleziono sieci",
        no_selection: "Wybierz sieć",
        password: "Hasło",
        wifi_on: "Włączone",
        wifi_off: "Wyłączone",
        language: "Język",
        settings: "Ustawienia",
        hacker_menu: "Zasilanie",
        launcher: {
            steam: "Steam",
            heroic: "Heroic Games Launcher",
            hyperplay: "HyperPlay",
            lutris: "Lutris",
            hacker_launcher: "Hacker Launcher"
        },
        system: {
            shutdown: "Wyłącz",
            restart: "Uruchom ponownie",
            sleep: "Uśpij",
            restart_apps: "Restartuj aplikacje",
            restart_sway: "Restartuj interfejs",
            switch_plasma: "Pulpit"
        }
    }
};

export const MOCK_WIFI_NETWORKS = [
    { ssid: 'Home_Network_5G', signal: 95, security: true },
{ ssid: 'Free_Public_WiFi', signal: 60, security: false },
{ ssid: 'Neighbor_Wifi', signal: 30, security: true },
{ ssid: 'HackerOS_Hotspot', signal: 100, security: true },
];
