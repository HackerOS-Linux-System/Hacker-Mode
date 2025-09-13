import logging
import os
import getpass

user_home = os.path.expanduser('~')
log_dir = os.path.join(user_home, '.hackeros')
log_file = os.path.join(log_dir, 'hacker-mode.log')

try:
    os.makedirs(log_dir, exist_ok=True)
    if not os.access(log_dir, os.W_OK):
        raise PermissionError(f"No write permission for directory: {log_dir}")
except PermissionError as e:
    log_dir = os.path.join(os.path.expanduser('~'), '.cache', 'hackeros')
    os.makedirs(log_dir, exist_ok=True)
    log_file = os.path.join(log_dir, 'hacker-mode.log')
    logging.warning(f"Failed to use primary log directory due to: {e}. Using fallback: {log_file}")

try:
    logging.basicConfig(
        filename=log_file,
        level=logging.INFO,
        format='%(asctime)s - %(levelname)s - %(message)s',
        filemode='a'
    )
except PermissionError as e:
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(levelname)s - %(message)s',
        handlers=[logging.StreamHandler()]
    )
    logging.error(f"Failed to open log file {log_file}: {e}. Logging to console instead.")

lang = 'en'

translations = {
    'en': {
        'settings': 'Settings',
        'back': 'Back',
        'close': 'Close',
        'audio': 'Audio',
        'increase_volume': 'Increase Volume',
        'decrease_volume': 'Decrease Volume',
        'toggle_mute': 'Toggle Mute',
        'display': 'Display',
        'increase_brightness': 'Increase Brightness',
        'decrease_brightness': 'Decrease Brightness',
        'toggle_theme': 'Toggle Theme',
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
        'connect': 'Connect',
        'scan': 'Scan',
        'pair': 'Pair',
        'no_networks': 'No networks found',
        'no_selection': 'No network selected',
        'connection_failed': 'Failed to connect: {error}',
        'connecting': 'Connecting to {ssid}',
        'wifi_toggle_failed': 'Failed to toggle Wi-Fi: {error}',
        'wifi_toggle_success': 'Wi-Fi turned {state}',
        'pairing_failed': 'Failed to pair: {error}',
        'pairing': 'Pairing with {device}',
        'error_returning_to_main': 'Error returning to main window: {error}',
        'app_not_installed': '{app} is not installed',
        'no_internet': 'No internet connection available',
        'launch_cooldown': 'Please wait {seconds} seconds before launching {app} again',
        'title': 'HackerOS',
        'hacker_menu': 'Hacker Menu',
        'switch_to_plasma': 'Switch to Plasma',
        'shutdown': 'Shutdown',
        'restart': 'Restart',
        'sleep': 'Sleep',
        'restart_apps': 'Restart Apps',
        'restart_sway': 'Restart Wayfire'
    },
    'pl': {
        'settings': 'Ustawienia',
        'back': 'Wróć',
        'close': 'Zamknij',
        'audio': 'Dźwięk',
        'increase_volume': 'Zwiększ głośność',
        'decrease_volume': 'Zmniejsz głośność',
        'toggle_mute': 'Wycisz/Włącz dźwięk',
        'display': 'Wyświetlacz',
        'increase_brightness': 'Zwiększ jasność',
        'decrease_brightness': 'Zmniejsz jasność',
        'toggle_theme': 'Zmień motyw',
        'network': 'Sieć',
        'wifi_settings': 'Ustawienia Wi-Fi',
        'toggle_wifi': 'Włącz/Wyłącz Wi-Fi',
        'bluetooth': 'Bluetooth',
        'power': 'Zasilanie',
        'power_saving': 'Oszczędzanie energii',
        'balanced': 'Zrównoważony',
        'performance': 'Wydajność',
        'general': 'Ogólne',
        'gaming_tools': 'Narzędzia dla graczy',
        'enable_gamescope': 'Włącz Gamescope',
        'enable_mangohud': 'Włącz MangoHUD',
        'enable_vkbasalt': 'Włącz vkBasalt',
        'connect': 'Połącz',
        'scan': 'Skanuj',
        'pair': 'Paruj',
        'no_networks': 'Nie znaleziono sieci',
        'no_selection': 'Nie wybrano sieci',
        'connection_failed': 'Nie udało się połączyć: {error}',
        'connecting': 'Łączenie z {ssid}',
        'wifi_toggle_failed': 'Nie udało się przełączyć Wi-Fi: {error}',
        'wifi_toggle_success': 'Wi-Fi przełączone na {state}',
        'pairing_failed': 'Nie udało się sparować: {error}',
        'pairing': 'Parowanie z {device}',
        'error_returning_to_main': 'Błąd podczas powrotu do głównego okna: {error}',
        'app_not_installed': '{app} nie jest zainstalowany',
        'no_internet': 'Brak połączenia z internetem',
        'launch_cooldown': 'Proszę czekać {seconds} sekund przed ponownym uruchomieniem {app}',
        'title': 'HackerOS',
        'hacker_menu': 'Menu Hakera',
        'switch_to_plasma': 'Przełącz na Plasma',
        'shutdown': 'Wyłącz',
        'restart': 'Uruchom ponownie',
        'sleep': 'Uśpij',
        'restart_apps': 'Uruchom ponownie aplikacje',
        'restart_sway': 'Uruchom ponownie Wayfire'
    }
}

def get_text(key, params=None):
    text = translations[lang].get(key, key)
    if params:
        return text.format(**params)
    return text

def set_gaming_tool(tool, enabled):
    logging.info(f'Setting {tool} to {"enabled" if enabled else "disabled"}')

def set_language(new_lang):
    global lang
    if new_lang in translations:
        lang = new_lang
        logging.info(f'Language set to {lang}')
    else:
        logging.warning(f'Invalid language: {new_lang}')

def setup_language():
    pass

