document.addEventListener('DOMContentLoaded', () => {
  const hackerMenuBtn = document.getElementById('hacker-menu-btn');
  const hackerMenuModal = document.getElementById('hacker-menu-modal');
  const settingsBtn = document.getElementById('settings-btn');
  const settingsModalBtn = document.getElementById('settings-modal-btn');
  const settingsModal = document.getElementById('settings-modal');
  const cancelMenu = document.getElementById('cancel-menu');
  const shutdownBtn = document.getElementById('shutdown-btn');
  const rebootBtn = document.getElementById('reboot-btn');
  const switchPlasma = document.getElementById('switch-plasma');
  const closeSettings = document.getElementById('close-settings');
  const launchSteam = document.getElementById('launch-steam');
  const launchEpic = document.getElementById('launch-epic');
  const launchGog = document.getElementById('launch-gog');
  const saveAccounts = document.getElementById('save-accounts');
  const brightnessSlider = document.getElementById('brightness-slider');
  const volumeSlider = document.getElementById('volume-slider');
  const checkUpdatesBtn = document.getElementById('check-updates');
  const updateSystemBtn = document.getElementById('update-system');
  const updateStatus = document.getElementById('update-status');
  const languageSelect = document.getElementById('language-select');
  const saveLanguage = document.getElementById('save-language');

  // Hacker Menu
  hackerMenuBtn.addEventListener('click', () => {
    hackerMenuModal.classList.remove('hidden');
  });
  cancelMenu.addEventListener('click', () => {
    hackerMenuModal.classList.add('hidden');
  });
  shutdownBtn.addEventListener('click', () => {
    window.api.shutdown();
  });
  rebootBtn.addEventListener('click', () => {
    window.api.reboot();
  });
  switchPlasma.addEventListener('click', () => {
    window.api.switchToPlasma();
  });
  settingsModalBtn.addEventListener('click', () => {
    hackerMenuModal.classList.add('hidden');
    settingsModal.classList.remove('hidden');
  });

  // Settings
  settingsBtn.addEventListener('click', () => {
    settingsModal.classList.remove('hidden');
  });
  closeSettings.addEventListener('click', () => {
    settingsModal.classList.add('hidden');
  });

  // Launchers
  launchSteam.addEventListener('click', () => {
    window.api.launchSteam();
  });
  launchEpic.addEventListener('click', () => {
    window.api.launchLegendary('epic');
  });
  launchGog.addEventListener('click', () => {
    window.api.launchLegendary('gog');
  });

  // Accounts
  saveAccounts.addEventListener('click', () => {
    const accounts = {
      steam: document.getElementById('steam-account').value,
      epic: document.getElementById('epic-account').value,
      gog: document.getElementById('gog-account').value,
    };
    window.api.setConfig('accounts', accounts);
  });

  // Brightness
  brightnessSlider.addEventListener('input', (e) => {
    window.api.setBrightness(e.target.value);
  });

  // Volume
  volumeSlider.addEventListener('input', (e) => {
    window.api.setVolume(e.target.value);
  });

  // Updates
  checkUpdatesBtn.addEventListener('click', async () => {
    const status = await window.api.checkUpdates();
    updateStatus.textContent = status;
  });
  updateSystemBtn.addEventListener('click', () => {
    window.api.updateSystem();
  });

  // Language
  saveLanguage.addEventListener('click', () => {
    const lang = languageSelect.value;
    window.api.setConfig('language', { lang });
    // Implement language change (reload or update UI texts)
  });

  // Load configs on start
  (async () => {
    const accounts = await window.api.getConfig('accounts');
    document.getElementById('steam-account').value = accounts.steam || '';
    document.getElementById('epic-account').value = accounts.epic || '';
    document.getElementById('gog-account').value = accounts.gog || '';

    const lang = await window.api.getConfig('language');
    languageSelect.value = lang.lang || 'pl';
  })();
});
