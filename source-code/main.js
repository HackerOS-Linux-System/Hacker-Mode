const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const { exec } = require('child_process');
const fs = require('fs');
const os = require('os');

let mainWindow;

function createWindow() {
  mainWindow = new BrowserWindow({
    fullscreen: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  mainWindow.loadFile('src/index.html');

  // Determine compositor based on GPU (simplified; in real scenario, detect GPU)
  const useCage = true; // Assume newer GPU for now; implement GPU detection if needed

  // Handle system commands via IPC
  ipcMain.handle('shutdown', () => {
    exec('shutdown -h now', (error) => {
      if (error) console.error(error);
    });
  });

  ipcMain.handle('reboot', () => {
    exec('reboot', (error) => {
      if (error) console.error(error);
    });
  });

  ipcMain.handle('switch-to-plasma', () => {
    exec('switch-to-plasma-command', (error) => { // Placeholder for actual command
      if (error) console.error(error);
    });
  });

  ipcMain.handle('launch-steam', () => {
    exec('HackerOS-Steam run -gamepadui', (error) => {
      if (error) console.error(error);
    });
  });

  ipcMain.handle('launch-legendary', (event, platform) => {
    const envPath = '/usr/lib/HackerOS/Hacker-Mode/legendary/';
    exec(`${envPath}legendary --platform ${platform}`, (error) => { // Assuming legendary command
      if (error) console.error(error);
    });
  });

  ipcMain.handle('get-config', (event, key) => {
    const configDir = path.join(os.homedir(), '.config/hacker-mode');
    const configFile = path.join(configDir, `${key}.json`);
    if (fs.existsSync(configFile)) {
      return JSON.parse(fs.readFileSync(configFile, 'utf8'));
    }
    return {};
  });

  ipcMain.handle('set-config', (event, { key, data }) => {
    const configDir = path.join(os.homedir(), '.config/hacker-mode');
    if (!fs.existsSync(configDir)) {
      fs.mkdirSync(configDir, { recursive: true });
    }
    const configFile = path.join(configDir, `${key}.json`);
    fs.writeFileSync(configFile, JSON.stringify(data));
  });

  ipcMain.handle('check-updates', async () => {
    // Placeholder for update check; use exec for actual system update command
    return new Promise((resolve) => {
      exec('check-updates-command', (error, stdout) => { // Placeholder
        resolve(stdout || 'No updates available');
      });
    });
  });

  ipcMain.handle('update-system', () => {
    exec('update-system-command', (error) => { // Placeholder
      if (error) console.error(error);
    });
  });

  // Brightness, volume, etc. - use system commands
  ipcMain.handle('set-brightness', (event, level) => {
    exec(`set-brightness ${level}`, (error) => { // Placeholder
      if (error) console.error(error);
    });
  });

  ipcMain.handle('set-volume', (event, level) => {
    exec(`set-volume ${level}`, (error) => { // Placeholder
      if (error) console.error(error);
    });
  });

  // Other settings similarly...
}

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    createWindow();
  }
});
