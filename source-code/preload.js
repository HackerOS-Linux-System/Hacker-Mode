const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('api', {
  shutdown: () => ipcRenderer.invoke('shutdown'),
  reboot: () => ipcRenderer.invoke('reboot'),
  switchToPlasma: () => ipcRenderer.invoke('switch-to-plasma'),
  launchSteam: () => ipcRenderer.invoke('launch-steam'),
  launchLegendary: (platform) => ipcRenderer.invoke('launch-legendary', platform),
  getConfig: (key) => ipcRenderer.invoke('get-config', key),
  setConfig: (key, data) => ipcRenderer.invoke('set-config', { key, data }),
  checkUpdates: () => ipcRenderer.invoke('check-updates'),
  updateSystem: () => ipcRenderer.invoke('update-system'),
  setBrightness: (level) => ipcRenderer.invoke('set-brightness', level),
  setVolume: (level) => ipcRenderer.invoke('set-volume', level),
  // Add more for bluetooth, wifi, firmware, night mode, performance, language...
});
