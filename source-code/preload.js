const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('electronApi', {
    launchApp: (appName) => ipcRenderer.invoke('launchApp', appName),
                                systemAction: (action) => ipcRenderer.invoke('systemAction', action),
                                launchSettings: () => ipcRenderer.invoke('launchSettings')
});

