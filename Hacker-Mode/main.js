const { app, BrowserWindow, ipcMain, Menu } = require('electron');
const { spawn, execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

let mainWindow;

function createWindow() {
    const preloadPath = path.resolve(__dirname, 'preload.js');
    if (!fs.existsSync(preloadPath)) {
        console.error(`Preload script not found at: ${preloadPath}`);
    } else {
        console.log(`Loading preload script from: ${preloadPath}`);
    }

    mainWindow = new BrowserWindow({
        fullscreen: true,
        webPreferences: {
            nodeIntegration: false,
            contextIsolation: true,
            preload: preloadPath
        }
    });
    mainWindow.loadFile('index.html');

    mainWindow.on('close', (event) => {
        if (!app.isQuitting) {
            event.preventDefault();
            mainWindow.hide();
        }
    });

    mainWindow.webContents.on('did-finish-load', () => {
        console.log('Renderer process loaded. Sending initial settings...');
        sendInitialSettings();
    });

    return mainWindow;
}

app.whenReady().then(() => {
    createWindow();

    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) {
            createWindow();
        }
    });
});

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});

ipcMain.on('launch-app', (event, launcher) => {
    let command, args;
    switch (launcher) {
        case 'steam':
            command = 'flatpak';
            args = ['run', 'com.valvesoftware.Steam', '-gamepadui'];
            break;
        case 'hyperplay':
            command = 'flatpak';
            args = ['run', 'xyz.hyperplay.HyperPlay'];
            break;
        case 'lutris':
            command = 'lutris';
            args = [];
            break;
        case 'heroic':
            command = 'flatpak';
            args = ['run', 'com.heroicgameslauncher.hgl'];
            break;
        case 'gamehub':
            command = 'gamehub';
            args = [];
            break;
        case 'sober':
            command = 'flatpak';
            args = ['run', 'org.vinegarhq.Sober'];
            break;
        default:
            console.error('Unknown launcher:', launcher);
            return;
    }

    mainWindow.hide();

    const env = {
        ...process.env,
        DISPLAY: ':0',
        WAYLAND_DISPLAY: process.env.WAYLAND_DISPLAY || 'wayland-0',
        XDG_RUNTIME_DIR: process.env.XDG_RUNTIME_DIR || '/run/user/1000'
    };

    console.log(`Launching ${launcher} with command: ${command} ${args.join(' ')}`);

    const child = spawn(command, args, { env, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });

    let stdoutData = '';
    let stderrData = '';

    child.stdout.on('data', (data) => {
        stdoutData += data.toString();
        console.log(`${launcher} stdout: ${data}`);
    });

    child.stderr.on('data', (data) => {
        stderrData += data.toString();
        console.error(`${launcher} stderr: ${data}`);
    });

    child.on('error', (error) => {
        console.error(`Error launching ${launcher}:`, error);
        console.error(`Stdout: ${stdoutData}`);
        console.error(`Stderr: ${stderrData}`);
        mainWindow.show();
    });

    child.on('spawn', () => {
        console.log(`${launcher} process spawned successfully`);
    });

    child.on('exit', (code, signal) => {
        console.log(`${launcher} process exited with code ${code}, signal ${signal}`);
        console.log(`Stdout: ${stdoutData}`);
        console.log(`Stderr: ${stderrData}`);
        if (mainWindow.isDestroyed()) {
            createWindow();
        } else {
            mainWindow.show();
            mainWindow.setFullScreen(true);
        }
    });

    child.unref();
});

app.on('before-quit', () => {
    app.isQuitting = true;
});

function sendInitialSettings() {
    try {
        const volOutput = execSync('pactl get-sink-volume @DEFAULT_SINK@').toString();
        const volMatch = volOutput.match(/Volume:.*?(\d+)%/);
        const volume = volMatch ? parseInt(volMatch[1]) : 50;

        const brightOutput = execSync('brightnessctl').toString();
        const brightMatch = brightOutput.match(/Current brightness:.*?(\d+)/);
        const brightness = brightMatch ? Math.round((parseInt(brightMatch[1]) / 1000) * 100) : 50;

        const wifiStatus = execSync('nmcli radio wifi').toString().trim() === 'enabled';

        const btStatus = execSync('bluetoothctl show').toString().includes('Powered: yes');

        mainWindow.webContents.send('initial-settings', { volume, brightness, wifi: wifiStatus, bluetooth: btStatus });
        console.log('Initial settings sent:', { volume, brightness, wifi: wifiStatus, bluetooth: btStatus });
    } catch (error) {
        console.error('Error getting initial settings:', error);
        mainWindow.webContents.send('initial-settings', { volume: 50, brightness: 50, wifi: false, bluetooth: false });
    }
}

ipcMain.on('set-volume', (event, volume) => {
    try {
        spawn('pactl', ['set-sink-volume', '@DEFAULT_SINK@', `${volume}%`]);
        console.log(`Volume set to ${volume}%`);
    } catch (error) {
        console.error('Error setting volume:', error);
    }
});

ipcMain.on('set-brightness', (event, value) => {
    try {
        spawn('brightnessctl', ['set', `${value}%`]);
        console.log(`Brightness set to ${value}%`);
    } catch (error) {
        console.error('Error setting brightness:', error);
    }
});

ipcMain.on('toggle-wifi', (event, enabled) => {
    try {
        spawn('nmcli', ['radio', 'wifi', enabled ? 'on' : 'off']);
        console.log(`Wi-Fi ${enabled ? 'enabled' : 'disabled'}`);
    } catch (error) {
        console.error('Error toggling Wi-Fi:', error);
    }
});

ipcMain.on('get-wifi-list', () => {
    try {
        const output = execSync('nmcli --terse --fields SSID dev wifi').toString();
        const ssids = output.split('\n').filter(s => s.trim()).map(s => s.trim());
        mainWindow.webContents.send('wifi-list', ssids);
        console.log('Wi-Fi networks:', ssids);
    } catch (error) {
        console.error('Error getting Wi-Fi list:', error);
        mainWindow.webContents.send('wifi-list', []);
    }
});

ipcMain.on('connect-wifi', (event, ssid) => {
    try {
        spawn('nmcli', ['dev', 'wifi', 'connect', ssid]);
        console.log(`Connecting to Wi-Fi network ${ssid}`);
        mainWindow.webContents.send('after-connect-wifi', null);
    } catch (error) {
        console.error('Error connecting to Wi-Fi:', error);
    }
});

ipcMain.on('toggle-bluetooth', (event, enabled) => {
    try {
        spawn('bluetoothctl', ['power', enabled ? 'on' : 'off']);
        console.log(`Bluetooth ${enabled ? 'enabled' : 'disabled'}`);
        mainWindow.webContents.send('after-toggle-bluetooth', null);
    } catch (error) {
        console.error('Error toggling Bluetooth:', error);
    }
});

ipcMain.on('get-bluetooth-list', () => {
    try {
        const output = execSync('bluetoothctl devices').toString();
        const devices = output.split('\n').filter(line => line.startsWith('Device')).map(line => {
            const [, id, ...name] = line.split(' ');
            return { id, name: name.join(' ') };
        });
        mainWindow.webContents.send('bluetooth-list', devices);
        console.log('Bluetooth devices:', devices);
    } catch (error) {
        console.error('Error getting Bluetooth list:', error);
        mainWindow.webContents.send('bluetooth-list', []);
    }
});

ipcMain.on('get-initial-settings', () => {
    sendInitialSettings();
});

ipcMain.on('show-hacker-menu', () => {
    const menu = Menu.buildFromTemplate([
        {
            label: 'Wyłącz komputer',
            click: () => {
                spawn('systemctl', ['poweroff']);
            }
        },
        {
            label: 'Uruchom ponownie',
            click: () => {
                spawn('systemctl', ['reboot']);
            }
        },
        {
            label: 'Wyloguj się',
            click: () => {
                spawn('swaymsg', ['exit']);
            }
        }
    ]);
    menu.popup({ window: mainWindow });
});
