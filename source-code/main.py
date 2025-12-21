import customtkinter as ctk
import subprocess
import os
import time
import datetime
import threading
import math
import re
from tkinter import Listbox, messagebox
from typing import Dict, Any

ctk.set_appearance_mode("dark")
ctk.set_default_color_theme("dark-blue")

lang = 'en'
enable_gamescope = False
enable_mangohud = False
enable_vkbasalt = False
last_launch_times: Dict[str, float] = {}
running_processes = []
is_dark_mode = True
is_muted = False
wifi_enabled = True
selected_wifi = None
selected_bluetooth = None

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
        locale = os.environ.get('LANG', 'en_US').split('_')[0]
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

def get_text(key: str, params: Dict[str, Any] = {}) -> str:
    text = translations.get(lang, {}).get(key, key)
    for k, v in params.items():
        text = text.replace(f'{{{k}}}', str(v))
    return text

def log(message: str, level: str = 'info'):
    log_message = f"{datetime.datetime.now().isoformat()} - {level.upper()} - {message}\n"
    with open('/tmp/hacker-mode.log', 'a') as f:
        f.write(log_message)

def set_gaming_tool(tool: str, enabled: bool):
    global enable_gamescope, enable_mangohud, enable_vkbasalt
    if tool == 'gamescope':
        enable_gamescope = enabled
    elif tool == 'mangohud':
        enable_mangohud = enabled
    elif tool == 'vkbasalt':
        enable_vkbasalt = enabled
    log(f'Toggled {tool} to {enabled}', 'info')

def check_app_installed(command, app_name: str, main_root) -> bool:
    try:
        if 'flatpak' in command:
            flatpak_id = command[2]
            stdout = subprocess.check_output(['flatpak', 'list', '--app', '--columns=application']).decode('utf-8')
            installed_apps = [app.strip() for app in stdout.split('\n') if app.strip()]
            if flatpak_id not in installed_apps:
                main_root.after(0, lambda: messagebox.showinfo('Error', get_text('app_not_installed')))
                log(f'{app_name} not installed', 'error')
                return False
            return True
        else:
            try:
                subprocess.check_output(['which', command[0]])
                return True
            except subprocess.CalledProcessError:
                main_root.after(0, lambda: messagebox.showinfo('Error', get_text('app_not_installed')))
                log(f'{app_name} not installed', 'error')
                return False
    except Exception as e:
        log(f'Error checking if {app_name} is installed: {e}', 'error')
        main_root.after(0, lambda: messagebox.showinfo('Error', get_text('app_not_installed')))
        return False

def check_internet() -> bool:
    try:
        stdout = subprocess.check_output(['nmcli', 'networking', 'connectivity']).decode('utf-8').strip()
        if stdout == 'full':
            return True
        subprocess.check_output(['ping', '-c', '1', '8.8.8.8'])
        return True
    except Exception as e:
        log(f'Error checking internet: {e}', 'error')
        return False

def set_fullscreen(app_name: str, title_name: str):
    retries = 3
    delay = 3
    for i in range(retries):
        try:
            subprocess.check_call(['xdotool', 'search', '--name', title_name, 'key', 'F11'])
            log(f'Set fullscreen for {app_name} using xdotool', 'info')
            return True
        except Exception as err:
            log(f'Attempt {i + 1} failed to set fullscreen for {app_name}: {err}', 'error')
            if i < retries - 1:
                time.sleep(delay)
    log(f'Failed to set fullscreen for {app_name} after {retries} attempts', 'error')
    return False

def launch_app(app_name: str, main_root):
    global last_launch_times
    current_time = time.time()
    cooldown_seconds = 60
    if app_name in last_launch_times and current_time - last_launch_times[app_name] < cooldown_seconds:
        remaining = math.ceil(cooldown_seconds - (current_time - last_launch_times[app_name]))
        main_root.after(0, lambda: messagebox.showinfo('Info', get_text('launch_cooldown', {'app': app_name, 'seconds': remaining})))
        log(f'Launch blocked for {app_name} due to cooldown: {remaining}s', 'info')
        return

    apps = {
        'steam': {'command': ['flatpak', 'run', 'com.valvesoftware.Steam', '-gamepadui'], 'flatpak': True, 'requiresInternet': True, 'titleName': 'Steam'},
        'heroic': {'command': ['flatpak', 'run', 'com.heroicgameslauncher.hgl'], 'flatpak': True, 'requiresInternet': True, 'titleName': 'Heroic Games Launcher'},
        'hyperplay': {'command': ['flatpak', 'run', 'xyz.hyperplay.HyperPlay'], 'flatpak': True, 'requiresInternet': True, 'titleName': 'HyperPlay'},
        'lutris': {'command': ['lutris'], 'flatpak': False, 'requiresInternet': False, 'titleName': 'Lutris'}
    }

    app = apps.get(app_name)
    if not app:
        log(f'Unknown app: {app_name}', 'error')
        return

    if not check_app_installed(app['command'], app_name, main_root):
        return

    if app['requiresInternet'] and not check_internet():
        main_root.after(0, lambda: messagebox.showinfo('Error', get_text('no_internet')))
        log(f'No internet for {app_name}', 'error')
        return

    main_root.withdraw()
    log(f'Launching {app_name}', 'info')
    env = os.environ.copy()
    env['XDG_SESSION_TYPE'] = 'x11'
    proc = subprocess.Popen(app['command'], env=env, start_new_session=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    running_processes.append((app_name, proc))
    last_launch_times[app_name] = current_time

    def monitor_process():
        proc.wait()
        log(f'{app_name} closed', 'info')
        global running_processes
        running_processes = [p for p in running_processes if p[1].pid != proc.pid]
        main_root.deiconify()
        try:
            subprocess.check_call(['swaymsg', 'fullscreen', 'enable'])
        except Exception as err:
            log(f'Error restoring fullscreen for Hacker Mode: {err}', 'error')

    threading.Thread(target=monitor_process, daemon=True).start()

    threading.Thread(target=lambda: time.sleep(3) or set_fullscreen(app_name, app['titleName']), daemon=True).start()

def system_action(action: str, main_root):
    def switch_to_plasma():
        log('Switching to Plasma', 'info')
        subprocess.call(['startplasma-wayland'])
        main_root.destroy()

    def shutdown():
        log('Shutting down', 'info')
        subprocess.call(['systemctl', 'poweroff'])

    def restart():
        log('Restarting', 'info')
        subprocess.call(['systemctl', 'reboot'])

    def sleep_():
        log('Suspending', 'info')
        subprocess.call(['systemctl', 'suspend'])

    def restart_apps():
        log('Restarting apps', 'info')
        for app in ['steam', 'heroic', 'hyperplay', 'lutris']:
            try:
                subprocess.call(['pkill', '-f', app])
            except Exception as e:
                log(f'Error killing {app}: {e}', 'error')
        global running_processes
        running_processes.clear()
        main_root.deiconify()
        try:
            subprocess.check_call(['swaymsg', 'fullscreen', 'enable'])
        except Exception as e:
            log(f'Error restoring fullscreen: {e}', 'error')

    def restart_sway():
        log('Restarting Sway', 'info')
        subprocess.call(['swaymsg', 'reload'])

    actions = {
        'switch_to_plasma': switch_to_plasma,
        'shutdown': shutdown,
        'restart': restart,
        'sleep': sleep_,
        'restart_apps': restart_apps,
        'restart_sway': restart_sway,
    }
    if action in actions:
        try:
            actions[action]()
        except Exception as e:
            log(f'Error in system action {action}: {e}', 'error')

def audio_increase_volume():
    global is_muted
    log('Increasing volume', 'info')
    is_muted = False
    try:
        subprocess.call(['pactl', 'set-sink-volume', '@DEFAULT_SINK@', '+5%'])
    except Exception as e:
        log(f'Error increasing volume: {e}', 'error')

def audio_decrease_volume():
    global is_muted
    log('Decreasing volume', 'info')
    is_muted = False
    try:
        subprocess.call(['pactl', 'set-sink-volume', '@DEFAULT_SINK@', '-5%'])
    except Exception as e:
        log(f'Error decreasing volume: {e}', 'error')

def audio_toggle_mute():
    global is_muted
    log('Toggling mute', 'info')
    is_muted = not is_muted
    try:
        subprocess.call(['pactl', 'set-sink-mute', '@DEFAULT_SINK@', 'toggle'])
    except Exception as e:
        log(f'Error toggling mute: {e}', 'error')

def display_increase_brightness():
    log('Increasing brightness', 'info')
    try:
        subprocess.call(['brightnessctl', 'set', '+5%'])
    except Exception as e:
        log(f'Error increasing brightness: {e}', 'error')

def display_decrease_brightness():
    log('Decreasing brightness', 'info')
    try:
        subprocess.call(['brightnessctl', 'set', '5%-'])
    except Exception as e:
        log(f'Error decreasing brightness: {e}', 'error')

def display_toggle_theme():
    global is_dark_mode
    log('Toggling theme', 'info')
    is_dark_mode = not is_dark_mode
    theme = 'dark' if is_dark_mode else 'light'
    config_path = os.path.join(os.path.expanduser('~'), '.config/sway/config')
    try:
        with open(config_path, 'r') as f:
            config = f.read()
        config = re.sub(r'set \$theme (dark|light)', f'set $theme {theme}', config)
        with open(config_path, 'w') as f:
            f.write(config)
        subprocess.call(['swaymsg', 'reload'])
    except Exception as e:
        log(f'Error toggling theme: {e}', 'error')

def network_show_wifi_settings(settings_root, wifi_panel, bluetooth_panel, wifi_list):
    log('Showing Wi-Fi settings', 'info')
    wifi_panel.pack(expand=True, fill='both')
    bluetooth_panel.pack_forget()
    wifi_list.delete(0, 'END')
    try:
        stdout = subprocess.check_output(['nmcli', '-t', '-f', 'SSID,SIGNAL', 'dev', 'wifi']).decode('utf-8')
        networks = [line.split(':') for line in stdout.split('\n') if line]
        if not networks:
            wifi_list.insert('END', get_text('no_networks'))
            return
        for ssid, signal in networks:
            wifi_list.insert('END', f"{ssid} ({signal}%)")
    except Exception as e:
        log(f'Error scanning Wi-Fi: {e}', 'error')
        wifi_list.insert('END', get_text('no_networks'))

def network_toggle_wifi(settings_root):
    global wifi_enabled
    log('Toggling Wi-Fi', 'info')
    wifi_enabled = not wifi_enabled
    state = 'on' if wifi_enabled else 'off'
    try:
        subprocess.check_call(['nmcli', 'radio', 'wifi', state])
        settings_root.after(0, lambda: messagebox.showinfo('Info', get_text('wifi_toggle_success', {'state': state})))
    except Exception as e:
        log(f'Error toggling Wi-Fi: {e}', 'error')
        settings_root.after(0, lambda: messagebox.showinfo('Error', get_text('wifi_toggle_failed', {'error': str(e)})))

def network_show_bluetooth(settings_root, wifi_panel, bluetooth_panel, bluetooth_list):
    log('Showing Bluetooth', 'info')
    bluetooth_panel.pack(expand=True, fill='both')
    wifi_panel.pack_forget()
    bluetooth_list.delete(0, 'END')

def power_action(profile: str):
    log(f'Setting power profile to {profile}', 'info')
    try:
        subprocess.call(['powerprofilesctl', 'set', profile])
    except Exception as e:
        log(f'Error setting power profile: {e}', 'error')

def on_select_wifi(e):
    global selected_wifi
    widget = e.widget
    selection = widget.curselection()
    if selection:
        selected = widget.get(selection[0])
        selected_wifi = selected.split(' (')[0]
    else:
        selected_wifi = None

def connect_wifi(settings_root, wifi_password, wifi_list):
    global selected_wifi
    if selected_wifi is None:
        settings_root.after(0, lambda: messagebox.showinfo('Error', get_text('no_selection')))
        return
    ssid = selected_wifi
    password = wifi_password.get()
    try:
        cmd = ['nmcli', 'dev', 'wifi', 'connect', ssid]
        if password:
            cmd += ['password', password]
        process = subprocess.run(cmd, capture_output=True, text=True)
        if process.returncode != 0:
            err = process.stderr or process.stdout
            settings_root.after(0, lambda: messagebox.showinfo('Error', get_text('connection_failed', {'error': err})))
            log(f'Wi-Fi connection failed: {err}', 'error')
        else:
            settings_root.after(0, lambda: messagebox.showinfo('Info', get_text('connecting', {'ssid': ssid})))
    except Exception as e:
        settings_root.after(0, lambda: messagebox.showinfo('Error', get_text('connection_failed', {'error': str(e)})))
        log(f'Error connecting to Wi-Fi: {e}', 'error')

def on_select_bluetooth(e):
    global selected_bluetooth
    widget = e.widget
    selection = widget.curselection()
    if selection:
        selected = widget.get(selection[0])
        selected_bluetooth = selected.split(' (')[1][:-1]
    else:
        selected_bluetooth = None

def scan_bluetooth(settings_root, bluetooth_list):
    log('Scanning Bluetooth', 'info')
    try:
        stdout = subprocess.check_output('bluetoothctl power on && sleep 1 && bluetoothctl scan on && sleep 5 && bluetoothctl devices && bluetoothctl scan off', shell=True).decode('utf-8')
        devices = []
        for line in stdout.split('\n'):
            if line.startswith('Device'):
                parts = line.split(' ', 2)
                device_id = parts[1]
                name = parts[2] if len(parts) > 2 else 'Unknown'
                devices.append((device_id, name))
        bluetooth_list.delete(0, 'END')
        if not devices:
            bluetooth_list.insert('END', get_text('no_devices'))
            return
        for device_id, name in devices:
            bluetooth_list.insert('END', f"{name} ({device_id})")
    except Exception as e:
        log(f'Error scanning Bluetooth: {e}', 'error')
        bluetooth_list.delete(0, 'END')
        bluetooth_list.insert('END', get_text('no_devices'))

def pair_bluetooth(settings_root, bluetooth_list):
    global selected_bluetooth
    if selected_bluetooth is None:
        settings_root.after(0, lambda: messagebox.showinfo('Error', get_text('no_selection')))
        return
    device_id = selected_bluetooth
    try:
        process_pair = subprocess.run(['bluetoothctl', 'pair', device_id], capture_output=True, text=True)
        if process_pair.returncode != 0:
            err = process_pair.stderr or process_pair.stdout
            settings_root.after(0, lambda: messagebox.showinfo('Error', get_text('pairing_failed', {'error': err})))
            log(f'Bluetooth pairing failed: {err}', 'error')
            return
        process_connect = subprocess.run(['bluetoothctl', 'connect', device_id], capture_output=True, text=True)
        if process_connect.returncode != 0:
            err = process_connect.stderr or process_connect.stdout
            settings_root.after(0, lambda: messagebox.showinfo('Error', get_text('pairing_failed', {'error': err})))
            log(f'Bluetooth connection failed: {err}', 'error')
        else:
            settings_root.after(0, lambda: messagebox.showinfo('Info', get_text('pairing', {'device': device_id})))
    except Exception as e:
        settings_root.after(0, lambda: messagebox.showinfo('Error', get_text('pairing_failed', {'error': str(e)})))
        log(f'Error pairing Bluetooth: {e}', 'error')

def create_main_window():
    root = ctk.CTk()
    root.attributes('-fullscreen', True)
    root.title(get_text('title'))

    title_label = ctk.CTkLabel(root, text=get_text('title'), font=ctk.CTkFont(size=48, weight='bold'))
    title_label.pack(pady=20)

    launch_frame = ctk.CTkFrame(root)
    launch_frame.pack()

    apps = ['steam', 'heroic', 'hyperplay', 'lutris']
    for i, app in enumerate(apps):
        btn = ctk.CTkButton(launch_frame, text=get_text(app), command=lambda a=app: launch_app(a, root), width=200, height=200)
        btn.grid(row=0, column=i, padx=20, pady=20)

    bottom_frame = ctk.CTkFrame(root)
    bottom_frame.pack(side='bottom', fill='x', pady=10)

    settings_btn = ctk.CTkButton(bottom_frame, text=get_text('settings'), command=lambda: create_settings_window(root))
    settings_btn.pack(side='left', padx=20)

    hacker_menu_btn = ctk.CTkButton(bottom_frame, text=get_text('hacker_menu'), command=lambda: toggle_hacker_menu(hacker_menu_frame))
    hacker_menu_btn.pack(side='left', padx=20)

    hacker_menu_frame = ctk.CTkFrame(root)
    hacker_menu_frame.pack_forget()

    menu_actions = ['switch_to_plasma', 'shutdown', 'restart', 'sleep', 'restart_apps', 'restart_sway']
    for act in menu_actions:
        btn = ctk.CTkButton(hacker_menu_frame, text=get_text(act), command=lambda a=act: system_action(a, root))
        btn.pack(fill='x', pady=5)

    def toggle_hacker_menu(frame):
        if frame.winfo_ismapped():
            frame.pack_forget()
        else:
            frame.pack(side='bottom', fill='x', pady=10)

    return root

def create_settings_window(main_root):
    settings = ctk.CTkToplevel(main_root)
    settings.title(get_text('settings'))
    settings.geometry('800x600')
    settings.grab_set()

    title_label = ctk.CTkLabel(settings, text=get_text('settings'), font=ctk.CTkFont(size=32, weight='bold'))
    title_label.pack(pady=10)

    panels_frame = ctk.CTkFrame(settings)
    panels_frame.pack(expand=True, fill='both')

    panels_frame.columnconfigure((0,1), weight=1)
    panels_frame.rowconfigure((0,1,2), weight=1)

    # Audio panel
    audio_panel = ctk.CTkFrame(panels_frame)
    audio_panel.grid(row=0, column=0, padx=10, pady=10, sticky='nsew')
    ctk.CTkLabel(audio_panel, text=get_text('audio')).pack(pady=5)
    ctk.CTkButton(audio_panel, text=get_text('increase_volume'), command=audio_increase_volume).pack(fill='x', pady=2)
    ctk.CTkButton(audio_panel, text=get_text('decrease_volume'), command=audio_decrease_volume).pack(fill='x', pady=2)
    ctk.CTkButton(audio_panel, text=get_text('toggle_mute'), command=audio_toggle_mute).pack(fill='x', pady=2)

    # Display panel
    display_panel = ctk.CTkFrame(panels_frame)
    display_panel.grid(row=0, column=1, padx=10, pady=10, sticky='nsew')
    ctk.CTkLabel(display_panel, text=get_text('display')).pack(pady=5)
    ctk.CTkButton(display_panel, text=get_text('increase_brightness'), command=display_increase_brightness).pack(fill='x', pady=2)
    ctk.CTkButton(display_panel, text=get_text('decrease_brightness'), command=display_decrease_brightness).pack(fill='x', pady=2)
    ctk.CTkButton(display_panel, text=get_text('toggle_theme'), command=display_toggle_theme).pack(fill='x', pady=2)

    # Network panel
    network_panel = ctk.CTkFrame(panels_frame)
    network_panel.grid(row=1, column=0, padx=10, pady=10, sticky='nsew')
    ctk.CTkLabel(network_panel, text=get_text('network')).pack(pady=5)
    ctk.CTkButton(network_panel, text=get_text('wifi_settings'), command=lambda: network_show_wifi_settings(settings, wifi_panel, bluetooth_panel, wifi_list)).pack(fill='x', pady=2)
    ctk.CTkButton(network_panel, text=get_text('toggle_wifi'), command=lambda: network_toggle_wifi(settings)).pack(fill='x', pady=2)
    ctk.CTkButton(network_panel, text=get_text('bluetooth'), command=lambda: network_show_bluetooth(settings, wifi_panel, bluetooth_panel, bluetooth_list)).pack(fill='x', pady=2)

    # Power panel
    power_panel = ctk.CTkFrame(panels_frame)
    power_panel.grid(row=1, column=1, padx=10, pady=10, sticky='nsew')
    ctk.CTkLabel(power_panel, text=get_text('power')).pack(pady=5)
    ctk.CTkButton(power_panel, text=get_text('power_saving'), command=lambda: power_action('power-saver')).pack(fill='x', pady=2)
    ctk.CTkButton(power_panel, text=get_text('balanced'), command=lambda: power_action('balanced')).pack(fill='x', pady=2)
    ctk.CTkButton(power_panel, text=get_text('performance'), command=lambda: power_action('performance')).pack(fill='x', pady=2)

    # General panel
    general_panel = ctk.CTkFrame(panels_frame)
    general_panel.grid(row=2, column=0, padx=10, pady=10, sticky='nsew')
    ctk.CTkLabel(general_panel, text=get_text('general')).pack(pady=5)
    language_combo = ctk.CTkComboBox(general_panel, values=['en', 'pl'])
    language_combo.pack(fill='x', pady=2)
    language_combo.set(lang)
    ctk.CTkButton(general_panel, text='Apply Language', command=lambda: set_language(language_combo.get())).pack(fill='x', pady=2)

    # Gaming tools panel
    gaming_panel = ctk.CTkFrame(panels_frame)
    gaming_panel.grid(row=2, column=1, padx=10, pady=10, sticky='nsew')
    ctk.CTkLabel(gaming_panel, text=get_text('gaming_tools')).pack(pady=5)
    gamescope_check = ctk.CTkCheckBox(gaming_panel, text=get_text('enable_gamescope'), command=lambda: set_gaming_tool('gamescope', gamescope_check.get()))
    gamescope_check.pack(pady=2)
    mangohud_check = ctk.CTkCheckBox(gaming_panel, text=get_text('enable_mangohud'), command=lambda: set_gaming_tool('mangohud', mangohud_check.get()))
    mangohud_check.pack(pady=2)
    vkbasalt_check = ctk.CTkCheckBox(gaming_panel, text=get_text('enable_vkbasalt'), command=lambda: set_gaming_tool('vkbasalt', vkbasalt_check.get()))
    vkbasalt_check.pack(pady=2)

    # WiFi panel
    wifi_panel = ctk.CTkFrame(settings)
    wifi_panel.pack_forget()
    ctk.CTkLabel(wifi_panel, text=get_text('wifi_settings')).pack(pady=5)
    wifi_list = Listbox(wifi_panel, height=10)
    wifi_list.pack(fill='x', pady=5)
    wifi_list.bind('<<ListboxSelect>>', on_select_wifi)
    wifi_password = ctk.CTkEntry(wifi_panel, placeholder_text='Password (if required)')
    wifi_password.pack(fill='x', pady=5)
    ctk.CTkButton(wifi_panel, text=get_text('connect'), command=lambda: connect_wifi(settings, wifi_password, wifi_list)).pack(fill='x', pady=5)

    # Bluetooth panel
    bluetooth_panel = ctk.CTkFrame(settings)
    bluetooth_panel.pack_forget()
    ctk.CTkLabel(bluetooth_panel, text=get_text('bluetooth')).pack(pady=5)
    bluetooth_list = Listbox(bluetooth_panel, height=10)
    bluetooth_list.pack(fill='x', pady=5)
    bluetooth_list.bind('<<ListboxSelect>>', on_select_bluetooth)
    ctk.CTkButton(bluetooth_panel, text=get_text('scan'), command=lambda: scan_bluetooth(settings, bluetooth_list)).pack(fill='x', pady=5)
    ctk.CTkButton(bluetooth_panel, text=get_text('pair'), command=lambda: pair_bluetooth(settings, bluetooth_list)).pack(fill='x', pady=5)

    bottom_frame = ctk.CTkFrame(settings)
    bottom_frame.pack(side='bottom', fill='x', pady=10)

    back_btn = ctk.CTkButton(bottom_frame, text=get_text('back'), command=settings.destroy)
    back_btn.pack(side='right', padx=10)

    close_btn = ctk.CTkButton(bottom_frame, text=get_text('close'), command=settings.destroy)
    close_btn.pack(side='right', padx=10)

if __name__ == "__main__":
    setup_language()
    main_window = create_main_window()
    main_window.mainloop()
