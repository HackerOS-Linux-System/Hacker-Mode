const { app, BrowserWindow, ipcMain, globalShortcut } = require('electron');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { exec } = require('child_process');
const util = require('util');
const execPromise = util.promisify(exec);
const { setMainWindow } = require('./launchers');
const { setupLanguage, getText } = require('./utils');
let mainWindow;
function log(message, level = 'info') {
    const logMessage = `${new Date().toISOString()} - ${level.toUpperCase()} - ${message}\n`;
    fs.appendFileSync('/tmp/hacker-mode.log', logMessage);
}
function createWindow() {
    mainWindow = new BrowserWindow({
        fullscreen: true,
        webPreferences: {
            nodeIntegration: false,
            contextIsolation: true,
            preload: path.join(__dirname, 'preload.js')
        },
        backgroundColor: '#1A1A1A'
    });
    mainWindow.loadFile('index.html').catch(e => log(`Error loading index.html: ${e}`, 'error'));
    mainWindow.on('closed', () => {
        mainWindow = null;
    });
    setMainWindow(mainWindow);
    mainWindow.webContents.on('did-finish-load', () => {
        mainWindow.webContents.executeJavaScript(`
        document.getElementById('title').innerText = '${getText('title')}';
        document.getElementById('settings-btn').innerText = '${getText('settings')}';
        document.getElementById('hacker-menu-btn').innerText = '${getText('hacker_menu')}';
        document.getElementById('menu-switch-to-plasma').innerText = '${getText('switch_to_plasma')}';
        document.getElementById('menu-shutdown').innerText = '${getText('shutdown')}';
        document.getElementById('menu-restart').innerText = '${getText('restart')}';
        document.getElementById('menu-sleep').innerText = '${getText('sleep')}';
        document.getElementById('menu-restart-apps').innerText = '${getText('restart_apps')}';
        document.getElementById('menu-restart-sway').innerText = '${getText('restart_sway')}';
        gsap.from('.launcher-btn', { duration: 1, y: 50, opacity: 0, stagger: 0.2 });
        `).catch(e => log(`Error executing JavaScript in main window: ${e}`, 'error'));
    });
    exec('swaymsg fullscreen enable', (err) => {
        if (err) log(`Error setting fullscreen: ${err}`, 'error');
    });
}
app.whenReady().then(() => {
    setupLanguage();
    createWindow();
    globalShortcut.register('CommandOrControl+Tab+M', () => {
        if (mainWindow) {
            mainWindow.webContents.executeJavaScript(`
            const menu = document.getElementById('hacker-menu');
            menu.classList.toggle('hidden');
            gsap.fromTo('#hacker-menu', { opacity: 0, y: -20 }, { opacity: 1, y: 0, duration: 0.5 });
            document.querySelectorAll('.menu-item').forEach((item, i) => {
                gsap.from(item, { duration: 0.5, x: -20, opacity: 0, delay: i * 0.1 });
            });
            `).catch(e => log(`Error toggling hacker menu via shortcut: ${e}`, 'error'));
        }
    });
    app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) createWindow();
    });
}).catch(e => log(`Error during app startup: ${e}`, 'error'));
app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') app.quit();
});
ipcMain.handle('launchSettings', async () => {
    log('Launching settings', 'info');
    mainWindow.hide();
    try {
        await execPromise(`${os.homedir()}/.hackeros/Hacker-Mode/settings`);
    } catch (e) {
        log(`Error launching settings binary: ${e}`, 'error');
    }
    mainWindow.show();
    exec('swaymsg fullscreen enable', (err) => {
        if (err) log(`Error restoring fullscreen after settings: ${err}`, 'error');
    });
});

