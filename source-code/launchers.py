import subprocess
import os
import time
import math
import threading
from tkinter import messagebox
from utils import get_text, log

running_processes = []
last_launch_times = {}

def check_app_installed(command, app_name, main_root):
    try:
        if 'flatpak' in command[0]:
            flatpak_id = command[2]
            stdout = subprocess.check_output(['flatpak', 'list', '--app', '--columns=application']).decode('utf-8')
            installed_apps = [app.strip() for app in stdout.split('\n') if app.strip()]
            if flatpak_id not in installed_apps:
                main_root.after(0, lambda: messagebox.showerror('Error', get_text('app_not_installed')))
                log(f'{app_name} not installed', 'error')
                return False
            return True
        else:
            subprocess.check_call(['which', command[0]])
            return True
    except subprocess.CalledProcessError:
        main_root.after(0, lambda: messagebox.showerror('Error', get_text('app_not_installed')))
        log(f'{app_name} not installed', 'error')
        return False
    except Exception as e:
        log(f'Error checking if {app_name} is installed: {e}', 'error')
        main_root.after(0, lambda: messagebox.showerror('Error', get_text('app_not_installed')))
        return False

def check_internet():
    try:
        stdout = subprocess.check_output(['nmcli', 'networking', 'connectivity']).decode('utf-8').strip()
        if stdout == 'full':
            return True
        subprocess.check_call(['ping', '-c', '1', '8.8.8.8'])
        return True
    except Exception as e:
        log(f'Error checking internet: {e}', 'error')
        return False

def set_fullscreen(app_name, title_name):
    retries = 3
    delay = 3
    for i in range(retries):
        try:
            subprocess.check_call(['xdotool', 'search', '--name', title_name, 'key', 'F11'])
            log(f'Set fullscreen for {app_name} using xdotool', 'info')
            return
        except Exception as e:
            log(f'Attempt {i+1} failed to set fullscreen for {app_name}: {e}', 'error')
            time.sleep(delay)
    log(f'Failed to set fullscreen for {app_name} after {retries} attempts', 'error')

def launch_app(app_name, main_root):
    global last_launch_times
    current_time = time.time()
    cooldown_seconds = 60
    if app_name in last_launch_times and current_time - last_launch_times[app_name] < cooldown_seconds:
        remaining = math.ceil(cooldown_seconds - (current_time - last_launch_times[app_name]))
        main_root.after(0, lambda: messagebox.showinfo('Info', get_text('launch_cooldown', {'seconds': remaining, 'app': app_name})))
        log(f'Launch blocked for {app_name} due to cooldown: {remaining}s', 'info')
        return

    apps = {
        'steam': {'command': ['flatpak', 'run', 'com.valvesoftware.Steam', '-gamepadui'], 'requiresInternet': True, 'titleName': 'Steam'},
        'heroic': {'command': ['flatpak', 'run', 'com.heroicgameslauncher.hgl'], 'requiresInternet': True, 'titleName': 'Heroic Games Launcher'},
        'hyperplay': {'command': ['flatpak', 'run', 'xyz.hyperplay.HyperPlay'], 'requiresInternet': True, 'titleName': 'HyperPlay'},
        'lutris': {'command': ['lutris'], 'requiresInternet': False, 'titleName': 'Lutris'}
    }

    app = apps.get(app_name)
    if not app:
        log(f'Unknown app: {app_name}', 'error')
        return

    if not check_app_installed(app['command'], app_name, main_root):
        return

    if app['requiresInternet'] and not check_internet():
        main_root.after(0, lambda: messagebox.showerror('Error', get_text('no_internet')))
        log(f'No internet for {app_name}', 'error')
        return

    main_root.withdraw()
    log(f'Launching {app_name}', 'info')
    env = os.environ.copy()
    env['XDG_SESSION_TYPE'] = 'x11'  # Assuming x11 as per user request
    proc = subprocess.Popen(app['command'], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    running_processes.append((app_name, proc))
    last_launch_times[app_name] = current_time

    def monitor_proc():
        proc.wait()
        log(f'{app_name} closed', 'info')
        global running_processes
        running_processes = [p for p in running_processes if p[1].pid != proc.pid]
        main_root.deiconify()
        try:
            subprocess.check_call(['swaymsg', 'fullscreen', 'enable'])
        except Exception as e:
            log(f'Error restoring fullscreen: {e}', 'error')

    threading.Thread(target=monitor_proc, daemon=True).start()
    time.sleep(3)
    set_fullscreen(app_name, app['titleName'])

def system_action(action, main_root):
    actions = {
        'switch_to_plasma': lambda: (log('Switching to Plasma', 'info'), subprocess.call(['startplasma-wayland']), main_root.quit()),
        'shutdown': lambda: (log('Shutting down', 'info'), subprocess.call(['systemctl', 'poweroff'])),
        'restart': lambda: (log('Restarting', 'info'), subprocess.call(['systemctl', 'reboot'])),
        'sleep': lambda: (log('Suspending', 'info'), subprocess.call(['systemctl', 'suspend'])),
        'restart_apps': lambda: restart_apps(main_root),
        'restart_sway': lambda: (log('Restarting Sway', 'info'), subprocess.call(['swaymsg', 'reload']))
    }

    def restart_apps(main_root):
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

    if action in actions:
        actions[action]()

