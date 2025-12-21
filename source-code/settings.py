import customtkinter as ctk
import subprocess
import re
import os
import threading
from tkinter import Listbox, messagebox
from utils import get_text, set_language, log, set_gaming_tool

is_dark_mode = True
is_muted = False
wifi_enabled = True
selected_wifi = None
selected_bluetooth = None

def audio_action(action):
    global is_muted
    if action == 'increase_volume':
        log('Increasing volume', 'info')
        is_muted = False
        try:
            subprocess.call(['pactl', 'set-sink-volume', '@DEFAULT_SINK@', '+5%'])
        except Exception as e:
            log(f'Error increasing volume: {e}', 'error')
    elif action == 'decrease_volume':
        log('Decreasing volume', 'info')
        is_muted = False
        try:
            subprocess.call(['pactl', 'set-sink-volume', '@DEFAULT_SINK@', '-5%'])
        except Exception as e:
            log(f'Error decreasing volume: {e}', 'error')
    elif action == 'toggle_mute':
        log('Toggling mute', 'info')
        is_muted = not is_muted
        try:
            subprocess.call(['pactl', 'set-sink-mute', '@DEFAULT_SINK@', 'toggle'])
        except Exception as e:
            log(f'Error toggling mute: {e}', 'error')

def display_action(action):
    global is_dark_mode
    if action == 'increase_brightness':
        log('Increasing brightness', 'info')
        try:
            subprocess.call(['brightnessctl', 'set', '+5%'])
        except Exception as e:
            log(f'Error increasing brightness: {e}', 'error')
    elif action == 'decrease_brightness':
        log('Decreasing brightness', 'info')
        try:
            subprocess.call(['brightnessctl', 'set', '5%-'])
        except Exception as e:
            log(f'Error decreasing brightness: {e}', 'error')
    elif action == 'toggle_theme':
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

def network_action(action, settings, wifi_panel, bluetooth_panel, wifi_list, bluetooth_list):
    if action == 'show_wifi_settings':
        log('Showing Wi-Fi settings', 'info')
        wifi_panel.pack(expand=True, fill='both', pady=10)
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
    elif action == 'toggle_wifi':
        global wifi_enabled
        log('Toggling Wi-Fi', 'info')
        wifi_enabled = not wifi_enabled
        state = 'on' if wifi_enabled else 'off'
        try:
            subprocess.check_call(['nmcli', 'radio', 'wifi', state])
            messagebox.showinfo('Info', get_text('wifi_toggle_success', {'state': state}))
        except Exception as e:
            log(f'Error toggling Wi-Fi: {e}', 'error')
            messagebox.showerror('Error', get_text('wifi_toggle_failed', {'error': str(e)}))
    elif action == 'show_bluetooth':
        log('Showing Bluetooth', 'info')
        bluetooth_panel.pack(expand=True, fill='both', pady=10)
        wifi_panel.pack_forget()
        bluetooth_list.delete(0, 'END')

def power_action(profile):
    log(f'Setting power profile to {profile}', 'info')
    try:
        subprocess.call(['powerprofilesctl', 'set', profile])
    except Exception as e:
        log(f'Error setting power profile: {e}', 'error')

def on_select_wifi(event):
    global selected_wifi
    widget = event.widget
    selection = widget.curselection()
    if selection:
        selected = widget.get(selection[0])
        selected_wifi = selected.split(' (')[0]

def connect_wifi(settings, wifi_password_entry, wifi_list):
    global selected_wifi
    if not selected_wifi:
        messagebox.showerror('Error', get_text('no_selection'))
        return
    password = wifi_password_entry.get()
    cmd = ['nmcli', 'dev', 'wifi', 'connect', selected_wifi]
    if password:
        cmd += ['password', password]
    try:
        process = subprocess.run(cmd, capture_output=True, text=True)
        if process.returncode != 0:
            err = process.stderr.strip()
            messagebox.showerror('Error', get_text('connection_failed', {'error': err}))
            log(f'Wi-Fi connection failed: {err}', 'error')
        else:
            messagebox.showinfo('Info', get_text('connecting', {'ssid': selected_wifi}))
    except Exception as e:
        messagebox.showerror('Error', get_text('connection_failed', {'error': str(e)}))
        log(f'Error connecting to Wi-Fi: {e}', 'error')

def on_select_bluetooth(event):
    global selected_bluetooth
    widget = event.widget
    selection = widget.curselection()
    if selection:
        selected = widget.get(selection[0])
        selected_bluetooth = selected.split(' (')[1].rstrip(')')

def scan_bluetooth(bluetooth_list):
    log('Scanning Bluetooth', 'info')
    bluetooth_list.delete(0, 'END')
    try:
        subprocess.run('bluetoothctl power on', shell=True, check=True)
        time.sleep(1)
        subprocess.run('bluetoothctl scan on & sleep 5 && bluetoothctl scan off', shell=True)
        stdout = subprocess.check_output(['bluetoothctl', 'devices']).decode('utf-8')
        devices = []
        for line in stdout.split('\n'):
            if line.startswith('Device'):
                parts = line.split(' ', 2)
                device_id = parts[1]
                name = parts[2] if len(parts) > 2 else 'Unknown'
                devices.append((device_id, name))
        if not devices:
            bluetooth_list.insert('END', get_text('no_devices'))
            return
        for device_id, name in devices:
            bluetooth_list.insert('END', f"{name} ({device_id})")
    except Exception as e:
        log(f'Error scanning Bluetooth: {e}', 'error')
        bluetooth_list.insert('END', get_text('no_devices'))

def pair_bluetooth(settings, bluetooth_list):
    global selected_bluetooth
    if not selected_bluetooth:
        messagebox.showerror('Error', get_text('no_selection'))
        return
    try:
        process_pair = subprocess.run(['bluetoothctl', 'pair', selected_bluetooth], capture_output=True, text=True)
        if process_pair.returncode != 0:
            err = process_pair.stderr.strip()
            messagebox.showerror('Error', get_text('pairing_failed', {'error': err}))
            log(f'Bluetooth pairing failed: {err}', 'error')
            return
        process_connect = subprocess.run(['bluetoothctl', 'connect', selected_bluetooth], capture_output=True, text=True)
        if process_connect.returncode != 0:
            err = process_connect.stderr.strip()
            messagebox.showerror('Error', get_text('pairing_failed', {'error': err}))
            log(f'Bluetooth connection failed: {err}', 'error')
        else:
            messagebox.showinfo('Info', get_text('pairing', {'device': selected_bluetooth}))
    except Exception as e:
        messagebox.showerror('Error', get_text('pairing_failed', {'error': str(e)}))
        log(f'Error pairing Bluetooth: {e}', 'error')

def create_settings_window(main_root):
    settings = ctk.CTkToplevel(main_root)
    settings.title(get_text('settings'))
    settings.geometry('800x600')
    settings.configure(bg='#1A1A1A')
    settings.after(200, settings.grab_set)  # Delay grab_set to avoid error

    title_label = ctk.CTkLabel(settings, text=get_text('settings'), font=ctk.CTkFont(size=32, weight='bold'), text_color='#0FF')
    title_label.pack(pady=20)

    panels_frame = ctk.CTkScrollableFrame(settings, fg_color='transparent')
    panels_frame.pack(expand=True, fill='both', padx=20)

    # Audio
    audio_panel = ctk.CTkFrame(panels_frame, fg_color='#2D2D2D', corner_radius=10)
    audio_panel.pack(fill='x', pady=10)
    ctk.CTkLabel(audio_panel, text=get_text('audio'), font=ctk.CTkFont(weight='bold')).pack(pady=5)
    ctk.CTkButton(audio_panel, text=get_text('increase_volume'), command=lambda: audio_action('increase_volume')).pack(fill='x', padx=10, pady=2)
    ctk.CTkButton(audio_panel, text=get_text('decrease_volume'), command=lambda: audio_action('decrease_volume')).pack(fill='x', padx=10, pady=2)
    ctk.CTkButton(audio_panel, text=get_text('toggle_mute'), command=lambda: audio_action('toggle_mute')).pack(fill='x', padx=10, pady=2)

    # Display
    display_panel = ctk.CTkFrame(panels_frame, fg_color='#2D2D2D', corner_radius=10)
    display_panel.pack(fill='x', pady=10)
    ctk.CTkLabel(display_panel, text=get_text('display'), font=ctk.CTkFont(weight='bold')).pack(pady=5)
    ctk.CTkButton(display_panel, text=get_text('increase_brightness'), command=lambda: display_action('increase_brightness')).pack(fill='x', padx=10, pady=2)
    ctk.CTkButton(display_panel, text=get_text('decrease_brightness'), command=lambda: display_action('decrease_brightness')).pack(fill='x', padx=10, pady=2)
    ctk.CTkButton(display_panel, text=get_text('toggle_theme'), command=lambda: display_action('toggle_theme')).pack(fill='x', padx=10, pady=2)

    # Network
    network_panel = ctk.CTkFrame(panels_frame, fg_color='#2D2D2D', corner_radius=10)
    network_panel.pack(fill='x', pady=10)
    ctk.CTkLabel(network_panel, text=get_text('network'), font=ctk.CTkFont(weight='bold')).pack(pady=5)
    ctk.CTkButton(network_panel, text=get_text('wifi_settings'), command=lambda: network_action('show_wifi_settings', settings, wifi_panel, bluetooth_panel, wifi_list, bluetooth_list)).pack(fill='x', padx=10, pady=2)
    ctk.CTkButton(network_panel, text=get_text('toggle_wifi'), command=lambda: network_action('toggle_wifi', settings, wifi_panel, bluetooth_panel, wifi_list, bluetooth_list)).pack(fill='x', padx=10, pady=2)
    ctk.CTkButton(network_panel, text=get_text('bluetooth'), command=lambda: network_action('show_bluetooth', settings, wifi_panel, bluetooth_panel, wifi_list, bluetooth_list)).pack(fill='x', padx=10, pady=2)

    # Power
    power_panel = ctk.CTkFrame(panels_frame, fg_color='#2D2D2D', corner_radius=10)
    power_panel.pack(fill='x', pady=10)
    ctk.CTkLabel(power_panel, text=get_text('power'), font=ctk.CTkFont(weight='bold')).pack(pady=5)
    ctk.CTkButton(power_panel, text=get_text('power_saving'), command=lambda: power_action('power-saver')).pack(fill='x', padx=10, pady=2)
    ctk.CTkButton(power_panel, text=get_text('balanced'), command=lambda: power_action('balanced')).pack(fill='x', padx=10, pady=2)
    ctk.CTkButton(power_panel, text=get_text('performance'), command=lambda: power_action('performance')).pack(fill='x', padx=10, pady=2)

    # General
    general_panel = ctk.CTkFrame(panels_frame, fg_color='#2D2D2D', corner_radius=10)
    general_panel.pack(fill='x', pady=10)
    ctk.CTkLabel(general_panel, text=get_text('general'), font=ctk.CTkFont(weight='bold')).pack(pady=5)
    language_combo = ctk.CTkComboBox(general_panel, values=['en', 'pl'])
    language_combo.set('en')
    language_combo.pack(fill='x', padx=10, pady=2)
    ctk.CTkButton(general_panel, text='Apply Language', command=lambda: set_language(language_combo.get())).pack(fill='x', padx=10, pady=2)

    # Gaming Tools
    gaming_panel = ctk.CTkFrame(panels_frame, fg_color='#2D2D2D', corner_radius=10)
    gaming_panel.pack(fill='x', pady=10)
    ctk.CTkLabel(gaming_panel, text=get_text('gaming_tools'), font=ctk.CTkFont(weight='bold')).pack(pady=5)
    gamescope_check = ctk.CTkCheckBox(gaming_panel, text=get_text('enable_gamescope'), command=lambda: set_gaming_tool('gamescope', gamescope_check.get()))
    gamescope_check.pack(padx=10, pady=2, anchor='w')
    mangohud_check = ctk.CTkCheckBox(gaming_panel, text=get_text('enable_mangohud'), command=lambda: set_gaming_tool('mangohud', mangohud_check.get()))
    mangohud_check.pack(padx=10, pady=2, anchor='w')
    vkbasalt_check = ctk.CTkCheckBox(gaming_panel, text=get_text('enable_vkbasalt'), command=lambda: set_gaming_tool('vkbasalt', vkbasalt_check.get()))
    vkbasalt_check.pack(padx=10, pady=2, anchor='w')

    # WiFi Panel
    wifi_panel = ctk.CTkFrame(settings, fg_color='#2D2D2D', corner_radius=10)
    wifi_panel.pack_forget()
    ctk.CTkLabel(wifi_panel, text=get_text('wifi_settings'), font=ctk.CTkFont(weight='bold')).pack(pady=5)
    wifi_list = Listbox(wifi_panel, height=10, bg='#3D3D3D', fg='white', selectbackground='#0FF')
    wifi_list.pack(fill='x', padx=10, pady=5)
    wifi_list.bind('<<ListboxSelect>>', on_select_wifi)
    wifi_password_entry = ctk.CTkEntry(wifi_panel, placeholder_text='Password (if required)')
    wifi_password_entry.pack(fill='x', padx=10, pady=5)
    ctk.CTkButton(wifi_panel, text=get_text('connect'), command=lambda: connect_wifi(settings, wifi_password_entry, wifi_list)).pack(fill='x', padx=10, pady=5)

    # Bluetooth Panel
    bluetooth_panel = ctk.CTkFrame(settings, fg_color='#2D2D2D', corner_radius=10)
    bluetooth_panel.pack_forget()
    ctk.CTkLabel(bluetooth_panel, text=get_text('bluetooth'), font=ctk.CTkFont(weight='bold')).pack(pady=5)
    bluetooth_list = Listbox(bluetooth_panel, height=10, bg='#3D3D3D', fg='white', selectbackground='#0FF')
    bluetooth_list.pack(fill='x', padx=10, pady=5)
    bluetooth_list.bind('<<ListboxSelect>>', on_select_bluetooth)
    ctk.CTkButton(bluetooth_panel, text=get_text('scan'), command=lambda: scan_bluetooth(bluetooth_list)).pack(fill='x', padx=10, pady=5)
    ctk.CTkButton(bluetooth_panel, text=get_text('pair'), command=lambda: pair_bluetooth(settings, bluetooth_list)).pack(fill='x', padx=10, pady=5)

    # Bottom buttons
    bottom_frame = ctk.CTkFrame(settings, fg_color='transparent')
    bottom_frame.pack(side='bottom', fill='x', pady=10, padx=20)
    ctk.CTkButton(bottom_frame, text=get_text('back'), command=settings.destroy).pack(side='right', padx=10)
    ctk.CTkButton(bottom_frame, text=get_text('close'), command=settings.destroy).pack(side='right', padx=10)
