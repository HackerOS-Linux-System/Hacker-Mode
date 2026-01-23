const { app, BrowserWindow } = require('electron');
const path = require('path');

function createWindow() {
    const win = new BrowserWindow({
        width: 1280,
        height: 800,
        frame: false, // Frameless
        fullscreen: true, // Force fullscreen
        kiosk: true, // Kiosk mode (prevents alt-tab in some DEs, keeps it on top)
    backgroundColor: '#0f1014', // Match the CSS background
    webPreferences: {
        nodeIntegration: true,
        contextIsolation: false,
        webSecurity: false // Allow loading local resources
    }
    });

    const isDev = !app.isPackaged;

    if (isDev) {
        // Wait for Vite to handle the request if running via concurrently
        win.loadURL('http://localhost:3000');
        // win.webContents.openDevTools(); // Uncomment to debug
    } else {
        win.loadFile(path.join(__dirname, '../dist/index.html'));
    }
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
