const { contextBridge, ipcRenderer } = require('electron');

console.log('Preload script loaded');

contextBridge.exposeInMainWorld('electronAPI', {
    send: (channel, data) => {
        const validChannels = [
            'launch-app',
            'set-volume',
            'set-brightness',
            'toggle-wifi',
            'toggle-bluetooth',
            'connect-wifi',
            'get-wifi-list',
            'get-bluetooth-list',
            'get-initial-settings',
            'show-hacker-menu'
        ];
        if (validChannels.includes(channel)) {
            console.log(`Sending IPC message on channel: ${channel}`, data);
            ipcRenderer.send(channel, data);
        } else {
            console.error(`Invalid IPC send channel: ${channel}`);
        }
    },
    on: (channel, callback) => {
        const validChannels = ['wifi-list', 'bluetooth-list', 'initial-settings'];
        if (validChannels.includes(channel)) {
            console.log(`Registering IPC listener for channel: ${channel}`);
            ipcRenderer.on(channel, (event, ...args) => {
                try {
                    console.log(`Received IPC message on channel: ${channel}`, args);
                    callback(...args);
                } catch (error) {
                    console.error(`Error in IPC callback for channel ${channel}:`, error);
                }
            });
        } else {
            console.error(`Invalid IPC receive channel: ${channel}`);
        }
    }
});
