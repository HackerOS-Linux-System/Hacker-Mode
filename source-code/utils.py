import os
import datetime

lang = 'en'
enable_gamescope = False
enable_mangohud = False
enable_vkbasalt = False

translations = {
    'en': {
        'title': 'Hacker Mode',
        'settings': 'Settings',
        'hacker_menu': 'Hacker Menu',
        'steam': 'Steam',
        'heroic': 'Heroic',
        'hyperplay': 'HyperPlay',
        'lutris': 'Lutris',
        'audio': 'Audio',
        'increase_volume': 'Increase Volume',
        'decrease_volume': 'Decrease Volume',
        'toggle_mute': 'Toggle Mute',
        'display': 'Display',
        'increase_brightness': 'Increase Brightness',
        'decrease_brightness': 'Decrease Brightness',
        'toggle_theme': 'Toggle Dark/Light Mode',
        'network': 'Network',
        'wifi_settings': 'Wi-Fi Settings',
        'toggle_wifi': 'Toggle Wi-Fi',
        'bluetooth': 'Bluetooth',
        'power': 'Power',
        'power_saving': 'Power Saving',
        'balanced': 'Balanced',
        'performance': 'Performance',
        'general': 'General',
        'gaming_tools': 'Gaming Tools',
        'enable_gamescope': 'Enable Gamescope',
        'enable_mangohud': 'Enable MangoHUD',
        'enable_vkbasalt': 'Enable vkBasalt',
        'app_not_installed': 'To install missing applications, use the package manager.',
        'launch_cooldown': 'Please wait {seconds} seconds before launching {app} again.',
        'no_internet': 'No internet connection. Please enable Wi-Fi.',
        'no_networks': 'No networks found',
        'connection_failed': 'Connection failed: {error}',
        'connecting': 'Connecting to {ssid}...',
        'wifi_toggle_success': 'Wi-Fi turned {state}',
        'wifi_toggle_failed': 'Failed to toggle Wi-Fi: {error}',
        'no_selection': 'Please select an item',
        'pairing': 'Pairing {device}...',
        'pairing_failed': 'Pairing failed: {error}',
        'switch_to_plasma': 'Switch to Plasma',
        'shutdown': 'Shutdown',
        'restart': 'Restart',
        'sleep': 'Sleep',
        'restart_apps': 'Restart Apps',
        'restart_sway': 'Restart Sway',
        'close': 'Close',
        'back': 'Back',
        'connect': 'Connect',
        'scan': 'Scan',
        'pair': 'Pair',
        'no_devices': 'No devices found',
    },
    'pl': {
        'title': 'Tryb Hakera',
        'settings': 'Ustawienia',
        'hacker_menu': 'Menu Hakera',
        'steam': 'Steam',
        'heroic': 'Heroic',
        'hyperplay': 'HyperPlay',
        'lutris': 'Lutris',
        'audio': 'Dźwięk',
        'increase_volume': 'Zwiększ głośność',
        'decrease_volume': 'Zmniejsz głośność',
        'toggle_mute': 'Wycisz/Włącz dźwięk',
        'display': 'Wyświetlacz',
        'increase_brightness': 'Zwiększ jasność',
        'decrease_brightness': 'Zmniejsz jasność',
        'toggle_theme': 'Przełącz tryb ciemny/jasny',
        'network': 'Sieć',
        'wifi_settings': 'Ustawienia Wi-Fi',
        'toggle_wifi': 'Włącz/Wyłącz Wi-Fi',
        'bluetooth': 'Bluetooth',
        'power': 'Zasilanie',
        'power_saving': 'Oszczędzanie energii',
        'balanced': 'Zrównoważony',
        'performance': 'Wydajność',
        'general': 'Ogólne',
        'gaming_tools': 'Narzędzia do gier',
        'enable_gamescope': 'Włącz Gamescope',
        'enable_mangohud': 'Włącz MangoHUD',
        'enable_vkbasalt': 'Włącz vkBasalt',
        'app_not_installed': 'Aby zainstalować brakujące aplikacje, użyj menedżera pakietów.',
        'launch_cooldown': 'Proszę czekać {seconds} sekund przed ponownym uruchomieniem {app}.',
        'no_internet': 'Brak połączenia z internetem. Proszę włączyć Wi-Fi.',
        'no_networks': 'Nie znaleziono sieci',
        'connection_failed': 'Połączenie nieudane: {error}',
        'connecting': 'Łączenie z {ssid}...',
        'wifi_toggle_success': 'Wi-Fi przełączone na {state}',
        'wifi_toggle_failed': 'Nie udało się przełączyć Wi-Fi: {error}',
        'no_selection': 'Proszę wybrać element',
        'pairing': 'Parowanie {device}...',
        'pairing_failed': 'Parowanie nieudane: {error}',
        'switch_to_plasma': 'Przełącz na Plasma',
        'shutdown': 'Wyłącz',
        'restart': 'Uruchom ponownie',
        'sleep': 'Uśpij',
        'restart_apps': 'Restartuj aplikacje',
        'restart_sway': 'Restartuj sesję Sway',
        'close': 'Zamknij',
        'back': 'Wróć',
        'connect': 'Połącz',
        'scan': 'Skanuj',
        'pair': 'Paruj',
        'no_devices': 'Nie znaleziono urządzeń',
    }
}

def setup_language():
    global lang
    try:
        locale = os.environ.get('LANG', 'en_US').split('.')[0].split('_')[0]
        lang = locale if locale in translations else 'en'
        log(f'Language set to: {lang}')
    except Exception as e:
        log(f'Error setting language: {e}', 'error')
        lang = 'en'
    return lang

def set_language(new_lang):
    global lang
    if new_lang in translations:
        lang = new_lang
        log(f'Language changed to: {new_lang}', 'info')

def get_text(key, params=None):
    if params is None:
        params = {}
    text = translations.get(lang, {}).get(key, key)
    for k, v in params.items():
        text = text.replace(f'{{{k}}}', str(v))
    return text

def set_gaming_tool(tool, enabled):
    global enable_gamescope, enable_mangohud, enable_vkbasalt
    if tool == 'gamescope':
        enable_gamescope = enabled
    elif tool == 'mangohud':
        enable_mangohud = enabled
    elif tool == 'vkbasalt':
        enable_vkbasalt = enabled
    log(f'Toggled {tool} to {enabled}', 'info')

def log(message, level='info'):
    log_message = f"{datetime.datetime.now().isoformat()} - {level.upper()} - {message}\n"
    with open('/tmp/hacker-mode.log', 'a') as f:
        f.write(log_message)
