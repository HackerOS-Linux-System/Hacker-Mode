import subprocess
import time
import os
from PySide6.QtWidgets import QMessageBox
from PySide6.QtCore import QTimer
from utils import get_text, logging

running_processes = []
last_launch_times = {}
apps = {
    'steam': {'command': ['flatpak', 'run', 'com.valvesoftware.Steam', '-gamepadui'], 'flatpak': True, 'requires_internet': True, 'title_name': 'Steam'},
    'heroic': {'command': ['flatpak', 'run', 'com.heroicgameslauncher.hgl'], 'flatpak': True, 'requires_internet': True, 'title_name': 'Heroic Games Launcher'},
    'hyperplay': {'command': ['flatpak', 'run', 'xyz.hyperplay.HyperPlay'], 'flatpak': True, 'requires_internet': True, 'title_name': 'HyperPlay'},
    'lutris': {'command': ['lutris'], 'flatpak': False, 'requires_internet': False, 'title_name': 'Lutris'}
}

def check_app_installed(command, app_name):
    try:
        if 'flatpak' in command:
            flatpak_id = command[2]
            result = subprocess.run(['flatpak', 'list', '--app', '--columns=application'], capture_output=True, text=True)
            installed = flatpak_id in result.stdout.splitlines()
            if not installed:
                QMessageBox.warning(None, "Warning", get_text('app_not_installed'))
                logging.error(f'{app_name} not installed')
                return False
            return True
        else:
            result = subprocess.run(['which', command[0]], capture_output=True)
            if result.returncode != 0:
                QMessageBox.warning(None, "Warning", get_text('app_not_installed'))
                logging.error(f'{app_name} not installed')
                return False
            return True
    except Exception as e:
        logging.error(f'Error checking if {app_name} is installed: {e}')
        QMessageBox.warning(None, "Warning", get_text('app_not_installed'))
        return False

def check_internet():
    try:
        result = subprocess.run(['nmcli', 'networking', 'connectivity'], capture_output=True, text=True)
        if result.stdout.strip() == 'full':
            return True
        subprocess.run(['ping', '-c', '1', '8.8.8.8'], check=True)
        return True
    except Exception as e:
        logging.error(f'Error checking internet: {e}')
        return False

def set_fullscreen(app_name, title_name, retries=3, delay=3):
    for i in range(retries):
        try:
            subprocess.run(['xdotool', 'search', '--name', title_name, 'key', 'F11'])
            logging.info(f'Set fullscreen for {app_name} using xdotool')
            return True
        except Exception as err:
            logging.error(f'Attempt {i+1} failed to set fullscreen for {app_name}: {err}')
            time.sleep(delay)
    logging.error(f'Failed to set fullscreen for {app_name} after {retries} attempts')
    return False

def launch_app(app_name, main_window):
    current_time = time.time()
    last_launch = last_launch_times.get(app_name, 0)
    cooldown = 60
    if current_time - last_launch < cooldown:
        remaining = int(cooldown - (current_time - last_launch)) + 1
        QMessageBox.warning(None, "Warning", get_text('launch_cooldown', {'seconds': remaining, 'app': app_name}))
        logging.info(f'Launch blocked for {app_name} due to cooldown: {remaining}s')
        return
    app = apps.get(app_name)
    if not app:
        logging.error(f'Unknown app: {app_name}')
        return
    if not check_app_installed(app['command'], app_name):
        return
    if app['requires_internet'] and not check_internet():
        QMessageBox.warning(None, "Warning", get_text('no_internet'))
        logging.error(f'No internet for {app_name}')
        return
    main_window.hide()
    logging.info(f'Launching {app_name}')
    env = os.environ.copy()
    env['XDG_SESSION_TYPE'] = 'wayland'
    proc = subprocess.Popen(app['command'], env=env)
    running_processes.append((app_name, proc))
    last_launch_times[app_name] = current_time
    QTimer.singleShot(3000, lambda: set_fullscreen(app_name, app['title_name']))
    def check_process():
        if proc.poll() is not None:
            logging.info(f'{app_name} closed')
            running_processes[:] = [p for p in running_processes if p[1].pid != proc.pid]
            main_window.show()
            try:
                subprocess.run(['wf-shell', 'fullscreen', 'enable'])
            except Exception as err:
                logging.error(f'Error restoring fullscreen for Hacker Mode: {err}')
            return
        QTimer.singleShot(1000, check_process)
    QTimer.singleShot(1000, check_process)

def system_action(action, main_window=None):
    actions = {
        'switchToPlasma': lambda: (logging.info('Switching to Plasma'), subprocess.run(['startplasma-wayland']), main_window.close() if main_window else None),
        'shutdown': lambda: (logging.info('Shutting down'), subprocess.run(['systemctl', 'poweroff'])),
        'restart': lambda: (logging.info('Restarting'), subprocess.run(['systemctl', 'reboot'])),
        'sleep': lambda: (logging.info('Suspending'), subprocess.run(['systemctl', 'suspend'])),
        'restartApps': lambda: (logging.info('Restarting apps'), [subprocess.run(['pkill', '-f', app]) for app in ['steam', 'heroic', 'hyperplay', 'lutris']], running_processes.clear(), main_window.show() if main_window else None, subprocess.run(['wf-shell', 'fullscreen', 'enable'])),
        'restartWayfire': lambda: (logging.info('Restarting Wayfire'), subprocess.run(['wayfire', '-c', os.path.expanduser('~/.config/wayfire.ini'), '--replace']))
    }
    func = actions.get(action)
    if func:
        try:
            func()
        except Exception as e:
            logging.error(f'Error performing {action}: {e}')
