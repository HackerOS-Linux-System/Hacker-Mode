import subprocess
import time
import os
from PySide6.QtWidgets import QMainWindow, QLabel, QPushButton, QGridLayout, QVBoxLayout, QWidget, QHBoxLayout, QComboBox, QCheckBox, QLineEdit, QScrollArea, QMessageBox
from PySide6.QtGui import QFont
from PySide6.QtCore import Qt, QPropertyAnimation, QRect, QEasingCurve, QTimer
from utils import get_text, set_gaming_tool, set_language, logging, lang

is_dark_mode = True
is_muted = False
wifi_enabled = True
selected_wifi = None
selected_bluetooth = None

class SettingsWindow(QMainWindow):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setWindowTitle(get_text('settings'))
        self.showFullScreen()
        self.central_widget = QWidget()
        self.setCentralWidget(self.central_widget)
        self.layout = QVBoxLayout(self.central_widget)
        self.layout.setAlignment(Qt.AlignCenter)
        self.layout.setSpacing(70)
        self.layout.setContentsMargins(80, 80, 80, 80)

        self.title = QLabel(get_text('settings'))
        self.title.setFont(QFont('Hack', 55, QFont.Bold))
        self.title.setObjectName('neon-text')
        self.layout.addWidget(self.title, alignment=Qt.AlignCenter)

        self.grid_layout = QGridLayout()
        self.grid_layout.setSpacing(60)
        self.grid_layout.setContentsMargins(60, 60, 60, 60)
        self.layout.addLayout(self.grid_layout)

        self.add_audio_panel()
        self.add_display_panel()
        self.add_network_panel()
        self.add_power_panel()
        self.add_general_panel()
        self.add_gaming_tools_panel()

        self.wifi_panel = QWidget(objectName='setting-panel')
        self.wifi_layout = QVBoxLayout(self.wifi_panel)
        self.wifi_layout.setSpacing(30)
        self.wifi_title = QLabel(get_text('wifi_settings'))
        self.wifi_title.setFont(QFont('Hack', 24, QFont.Bold))
        self.wifi_layout.addWidget(self.wifi_title)
        self.wifi_list = QScrollArea()
        self.wifi_list.setWidgetResizable(True)
        self.wifi_list_widget = QWidget()
        self.wifi_list_layout = QVBoxLayout(self.wifi_list_widget)
        self.wifi_list_layout.setSpacing(25)
        self.wifi_list.setWidget(self.wifi_list_widget)
        self.wifi_layout.addWidget(self.wifi_list)
        self.wifi_password = QLineEdit()
        self.wifi_password.setPlaceholderText('Password (if required)')
        self.wifi_password.setObjectName('input-field')
        self.wifi_password.setFixedHeight(70)
        self.wifi_layout.addWidget(self.wifi_password)
        self.connect_btn = QPushButton(get_text('connect'))
        self.connect_btn.setObjectName('setting-btn')
        self.connect_btn.setFixedHeight(70)
        self.connect_btn.clicked.connect(self.connect_wifi)
        self.wifi_layout.addWidget(self.connect_btn)
        self.wifi_panel.setHidden(True)
        self.layout.addWidget(self.wifi_panel)

        self.bluetooth_panel = QWidget(objectName='setting-panel')
        self.bluetooth_layout = QVBoxLayout(self.bluetooth_panel)
        self.bluetooth_layout.setSpacing(30)
        self.bluetooth_title = QLabel(get_text('bluetooth'))
        self.bluetooth_title.setFont(QFont('Hack', 24, QFont.Bold))
        self.bluetooth_layout.addWidget(self.bluetooth_title)
        self.bluetooth_list = QScrollArea()
        self.bluetooth_list.setWidgetResizable(True)
        self.bluetooth_list_widget = QWidget()
        self.bluetooth_list_layout = QVBoxLayout(self.bluetooth_list_widget)
        self.bluetooth_list_layout.setSpacing(25)
        self.bluetooth_list.setWidget(self.bluetooth_list_widget)
        self.bluetooth_layout.addWidget(self.bluetooth_list)
        self.scan_btn = QPushButton(get_text('scan'))
        self.scan_btn.setObjectName('setting-btn')
        self.scan_btn.setFixedHeight(70)
        self.scan_btn.clicked.connect(self.scan_bluetooth)
        self.bluetooth_layout.addWidget(self.scan_btn)
        self.pair_btn = QPushButton(get_text('pair'))
        self.pair_btn.setObjectName('setting-btn')
        self.pair_btn.setFixedHeight(70)
        self.pair_btn.clicked.connect(self.pair_bluetooth)
        self.bluetooth_layout.addWidget(self.pair_btn)
        self.bluetooth_panel.setHidden(True)
        self.layout.addWidget(self.bluetooth_panel)

        self.bottom_layout = QHBoxLayout()
        self.bottom_layout.setAlignment(Qt.AlignRight)
        self.bottom_layout.setSpacing(50)
        self.bottom_layout.addStretch()
        self.back_btn = QPushButton(get_text('back'))
        self.back_btn.setObjectName('action-btn')
        self.back_btn.setFixedSize(320, 110)
        self.back_btn.setFont(QFont('Hack', 26, QFont.Bold))
        self.back_btn.clicked.connect(self.back_to_main)
        self.bottom_layout.addWidget(self.back_btn)
        logging.info(f'Back button added to settings window at position: {self.back_btn.geometry().getRect()}')
        self.close_btn = QPushButton(get_text('close'))
        self.close_btn.setObjectName('action-btn')
        self.close_btn.setFixedSize(320, 110)
        self.close_btn.setFont(QFont('Hack', 26, QFont.Bold))
        self.close_btn.clicked.connect(self.close)
        self.bottom_layout.addWidget(self.close_btn)
        self.layout.addLayout(self.bottom_layout)
        self.layout.addStretch()

        self.update_texts()
        QTimer.singleShot(100, self.animate_panels)
        QTimer.singleShot(500, self.log_button_visibility)
        logging.info('SettingsWindow initialized successfully')

    def log_button_visibility(self):
        logging.info(f'Back button visible: {self.back_btn.isVisible()}, geometry: {self.back_btn.geometry().getRect()}')

    def back_to_main(self):
        try:
            if self.parent():
                self.parent().showFullScreen()
                logging.info('Returning to main window in fullscreen mode')
            else:
                logging.warning('No parent window found when returning from settings')
                QMessageBox.warning(self, "Warning", get_text('error_returning_to_main', {'error': 'No parent window'}))
            self.close()
        except Exception as e:
            logging.error(f'Error returning to main window: {e}')
            QMessageBox.warning(self, "Warning", get_text('error_returning_to_main', {'error': str(e)}))

    def animate_panels(self):
        panels = self.central_widget.findChildren(QWidget, 'setting-panel')
        for i, panel in enumerate(panels):
            anim = QPropertyAnimation(panel, b"geometry")
            rect = panel.geometry()
            anim.setStartValue(QRect(rect.x(), rect.y() + 300, rect.width(), rect.height()))
            anim.setEndValue(rect)
            anim.setDuration(2000)
            anim.setEasingCurve(QEasingCurve.OutQuint)
            QTimer.singleShot(i * 400, anim.start)

    def add_audio_panel(self):
        panel = QWidget(objectName='setting-panel')
        layout = QVBoxLayout(panel)
        layout.setSpacing(30)
        title = QLabel(get_text('audio'))
        title.setFont(QFont('Hack', 22, QFont.Bold))
        layout.addWidget(title)
        inc_vol = QPushButton(get_text('increase_volume'))
        inc_vol.setObjectName('setting-btn')
        inc_vol.setFixedHeight(70)
        inc_vol.clicked.connect(lambda: self.audio_action('increaseVolume'))
        layout.addWidget(inc_vol)
        dec_vol = QPushButton(get_text('decrease_volume'))
        dec_vol.setObjectName('setting-btn')
        dec_vol.setFixedHeight(70)
        dec_vol.clicked.connect(lambda: self.audio_action('decreaseVolume'))
        layout.addWidget(dec_vol)
        toggle_mute = QPushButton(get_text('toggle_mute'))
        toggle_mute.setObjectName('setting-btn')
        toggle_mute.setFixedHeight(70)
        toggle_mute.clicked.connect(lambda: self.audio_action('toggleMute'))
        layout.addWidget(toggle_mute)
        self.grid_layout.addWidget(panel, 0, 0)

    def add_display_panel(self):
        panel = QWidget(objectName='setting-panel')
        layout = QVBoxLayout(panel)
        layout.setSpacing(30)
        title = QLabel(get_text('display'))
        title.setFont(QFont('Hack', 22, QFont.Bold))
        layout.addWidget(title)
        inc_bright = QPushButton(get_text('increase_brightness'))
        inc_bright.setObjectName('setting-btn')
        inc_bright.setFixedHeight(70)
        inc_bright.clicked.connect(lambda: self.display_action('increaseBrightness'))
        layout.addWidget(inc_bright)
        dec_bright = QPushButton(get_text('decrease_brightness'))
        dec_bright.setObjectName('setting-btn')
        dec_bright.setFixedHeight(70)
        dec_bright.clicked.connect(lambda: self.display_action('decreaseBrightness'))
        layout.addWidget(dec_bright)
        toggle_theme = QPushButton(get_text('toggle_theme'))
        toggle_theme.setObjectName('setting-btn')
        toggle_theme.setFixedHeight(70)
        toggle_theme.clicked.connect(lambda: self.display_action('toggleTheme'))
        layout.addWidget(toggle_theme)
        self.grid_layout.addWidget(panel, 0, 1)

    def add_network_panel(self):
        panel = QWidget(objectName='setting-panel')
        layout = QVBoxLayout(panel)
        layout.setSpacing(30)
        title = QLabel(get_text('network'))
        title.setFont(QFont('Hack', 22, QFont.Bold))
        layout.addWidget(title)
        wifi_settings = QPushButton(get_text('wifi_settings'))
        wifi_settings.setObjectName('setting-btn')
        wifi_settings.setFixedHeight(70)
        wifi_settings.clicked.connect(lambda: self.network_action('showWifiSettings'))
        layout.addWidget(wifi_settings)
        toggle_wifi = QPushButton(get_text('toggle_wifi'))
        toggle_wifi.setObjectName('setting-btn')
        toggle_wifi.setFixedHeight(70)
        toggle_wifi.clicked.connect(lambda: self.network_action('toggleWifi'))
        layout.addWidget(toggle_wifi)
        bluetooth = QPushButton(get_text('bluetooth'))
        bluetooth.setObjectName('setting-btn')
        bluetooth.setFixedHeight(70)
        bluetooth.clicked.connect(lambda: self.network_action('showBluetooth'))
        layout.addWidget(bluetooth)
        self.grid_layout.addWidget(panel, 1, 0)

    def add_power_panel(self):
        panel = QWidget(objectName='setting-panel')
        layout = QVBoxLayout(panel)
        layout.setSpacing(30)
        title = QLabel(get_text('power'))
        title.setFont(QFont('Hack', 22, QFont.Bold))
        layout.addWidget(title)
        power_saver = QPushButton(get_text('power_saving'))
        power_saver.setObjectName('setting-btn')
        power_saver.setFixedHeight(70)
        power_saver.clicked.connect(lambda: self.power_action('power-saver'))
        layout.addWidget(power_saver)
        balanced = QPushButton(get_text('balanced'))
        balanced.setObjectName('setting-btn')
        balanced.setFixedHeight(70)
        balanced.clicked.connect(lambda: self.power_action('balanced'))
        layout.addWidget(balanced)
        performance = QPushButton(get_text('performance'))
        performance.setObjectName('setting-btn')
        performance.setFixedHeight(70)
        performance.clicked.connect(lambda: self.power_action('performance'))
        layout.addWidget(performance)
        self.grid_layout.addWidget(panel, 1, 1)

    def add_general_panel(self):
        panel = QWidget(objectName='setting-panel')
        layout = QVBoxLayout(panel)
        layout.setSpacing(30)
        title = QLabel(get_text('general'))
        title.setFont(QFont('Hack', 22, QFont.Bold))
        layout.addWidget(title)
        self.lang_select = QComboBox()
        self.lang_select.setObjectName('input-field')
        self.lang_select.setFixedHeight(70)
        self.lang_select.addItems(['en', 'pl'])
        self.lang_select.setCurrentText(lang)
        layout.addWidget(self.lang_select)
        apply_lang = QPushButton('Apply Language')
        apply_lang.setObjectName('setting-btn')
        apply_lang.setFixedHeight(70)
        apply_lang.clicked.connect(lambda: self.set_language(self.lang_select.currentText()))
        layout.addWidget(apply_lang)
        self.grid_layout.addWidget(panel, 2, 0)

    def add_gaming_tools_panel(self):
        panel = QWidget(objectName='setting-panel')
        layout = QVBoxLayout(panel)
        layout.setSpacing(30)
        title = QLabel(get_text('gaming_tools'))
        title.setFont(QFont('Hack', 22, QFont.Bold))
        layout.addWidget(title)
        gamescope_check = QCheckBox(get_text('enable_gamescope'))
        gamescope_check.setObjectName('checkbox')
        gamescope_check.stateChanged.connect(lambda state: set_gaming_tool('gamescope', state == Qt.Checked))
        layout.addWidget(gamescope_check)
        mangohud_check = QCheckBox(get_text('enable_mangohud'))
        mangohud_check.setObjectName('checkbox')
        mangohud_check.stateChanged.connect(lambda state: set_gaming_tool('mangohud', state == Qt.Checked))
        layout.addWidget(mangohud_check)
        vkbasalt_check = QCheckBox(get_text('enable_vkbasalt'))
        vkbasalt_check.setObjectName('checkbox')
        vkbasalt_check.stateChanged.connect(lambda state: set_gaming_tool('vkbasalt', state == Qt.Checked))
        layout.addWidget(vkbasalt_check)
        self.grid_layout.addWidget(panel, 2, 1)

    def update_texts(self):
        self.setWindowTitle(get_text('settings'))
        self.title.setText(get_text('settings'))
        self.wifi_title.setText(get_text('wifi_settings'))
        self.bluetooth_title.setText(get_text('bluetooth'))
        self.connect_btn.setText(get_text('connect'))
        self.scan_btn.setText(get_text('scan'))
        self.pair_btn.setText(get_text('pair'))
        self.back_btn.setText(get_text('back'))
        self.close_btn.setText(get_text('close'))
        for i in range(self.grid_layout.count()):
            panel = self.grid_layout.itemAt(i).widget()
            if panel:
                labels = panel.findChildren(QLabel)
                buttons = panel.findChildren(QPushButton)
                checkboxes = panel.findChildren(QCheckBox)
                if labels:
                    labels[0].setText(get_text(labels[0].text().lower().replace(' ', '_')))
                for btn in buttons:
                    key = btn.text().lower().replace(' ', '_')
                    btn.setText(get_text(key))
                for cb in checkboxes:
                    key = cb.text().lower().replace(' ', '_')
                    cb.setText(get_text(key))

    def audio_action(self, action):
        global is_muted
        if action == 'increaseVolume':
            logging.info('Increasing volume')
            is_muted = False
            subprocess.run(['pactl', 'set-sink-volume', '@DEFAULT_SINK@', '+5%'])
        elif action == 'decreaseVolume':
            logging.info('Decreasing volume')
            is_muted = False
            subprocess.run(['pactl', 'set-sink-volume', '@DEFAULT_SINK@', '-5%'])
        elif action == 'toggleMute':
            logging.info('Toggling mute')
            is_muted = not is_muted
            subprocess.run(['pactl', 'set-sink-mute', '@DEFAULT_SINK@', 'toggle'])

    def display_action(self, action):
        global is_dark_mode
        config_path = os.path.join(os.path.expanduser('~'), '.hackeros/Hacker-Mode/wayfire.ini')
        if action == 'increaseBrightness':
            logging.info('Increasing brightness')
            subprocess.run(['brightnessctl', 'set', '+5%'])
        elif action == 'decreaseBrightness':
            logging.info('Decreasing brightness')
            subprocess.run(['brightnessctl', 'set', '5%-'])
        elif action == 'toggleTheme':
            logging.info('Toggling theme')
            is_dark_mode = not is_dark_mode
            theme = 'dark' if is_dark_mode else 'light'
            try:
                with open(config_path, 'r') as f:
                    config = f.read()
                config = config.replace(r'theme=(dark|light)', f'theme={theme}')
                with open(config_path, 'w') as f:
                    f.write(config)
                subprocess.run(['wayfire', '-c', config_path, '--replace'])
            except Exception as e:
                logging.error(f'Error toggling theme: {e}')

    def network_action(self, action):
        global wifi_enabled
        if action == 'showWifiSettings':
            logging.info('Showing Wi-Fi settings')
            self.wifi_panel.setHidden(False)
            self.bluetooth_panel.setHidden(True)
            self.update_wifi_list()
        elif action == 'toggleWifi':
            logging.info('Toggling Wi-Fi')
            wifi_enabled = not wifi_enabled
            state = 'on' if wifi_enabled else 'off'
            try:
                result = subprocess.run(['nmcli', 'radio', 'wifi', state], capture_output=True, text=True)
                if result.returncode != 0:
                    QMessageBox.warning(self, "Warning", get_text('wifi_toggle_failed', {'error': result.stderr}))
                    logging.error(f'Failed to toggle Wi-Fi: {result.stderr}')
                else:
                    QMessageBox.information(self, "Info", get_text('wifi_toggle_success', {'state': state}))
            except Exception as e:
                logging.error(f'Error toggling Wi-Fi: {e}')
        elif action == 'showBluetooth':
            logging.info('Showing Bluetooth')
            self.bluetooth_panel.setHidden(False)
            self.wifi_panel.setHidden(True)

    def power_action(self, profile):
        logging.info(f'Setting power profile to {profile}')
        subprocess.run(['powerprofilesctl', 'set', profile])

    def set_language(self, new_lang):
        set_language(new_lang)
        self.update_texts()
        if self.parent():
            self.parent().update_texts()

    def update_wifi_list(self):
        while self.wifi_list_layout.count():
            child = self.wifi_list_layout.takeAt(0)
            if child.widget():
                child.widget().deleteLater()
        try:
            result = subprocess.run(['nmcli', '-t', '-f', 'SSID,SIGNAL', 'dev', 'wifi'], capture_output=True, text=True)
            networks = [line.split(':') for line in result.stdout.splitlines() if line]
            if not networks:
                label = QLabel(get_text('no_networks'))
                label.setObjectName('list-label')
                self.wifi_list_layout.addWidget(label)
                return
            for ssid, signal in networks:
                btn = QPushButton(f'{ssid} ({signal}%)')
                btn.setObjectName('list-btn')
                btn.setFixedHeight(70)
                btn.clicked.connect(lambda checked, s=ssid: self.select_wifi(s))
                self.wifi_list_layout.addWidget(btn)
        except Exception as e:
            logging.error(f'Error scanning Wi-Fi: {e}')
            label = QLabel(get_text('no_networks'))
            label.setObjectName('list-label')
            self.wifi_list_layout.addWidget(label)

    def select_wifi(self, ssid):
        global selected_wifi
        selected_wifi = ssid
        for i in range(self.wifi_list_layout.count()):
            btn = self.wifi_list_layout.itemAt(i).widget()
            if btn and btn.text().startswith(ssid):
                btn.setStyleSheet("background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #3d3d3d, stop:1 #2a2a2a);")
            else:
                btn.setStyleSheet("")

    def connect_wifi(self):
        global selected_wifi
        if not selected_wifi:
            QMessageBox.warning(self, "Warning", get_text('no_selection'))
            return
        password = self.wifi_password.text()
        cmd = ['nmcli', 'dev', 'wifi', 'connect', selected_wifi]
        if password:
            cmd += ['password', password]
        try:
            result = subprocess.run(cmd, capture_output=True, text=True)
            if result.returncode != 0:
                QMessageBox.warning(self, "Warning", get_text('connection_failed', {'error': result.stderr}))
                logging.error(f'Wi-Fi connection failed: {result.stderr}')
            else:
                QMessageBox.information(self, "Info", get_text('connecting', {'ssid': selected_wifi}))
        except Exception as e:
            QMessageBox.warning(self, "Warning", get_text('connection_failed', {'error': str(e)}))
            logging.error(f'Error connecting to Wi-Fi: {e}')

    def scan_bluetooth(self):
        while self.bluetooth_list_layout.count():
            child = self.bluetooth_list_layout.takeAt(0)
            if child.widget():
                child.widget().deleteLater()
        try:
            subprocess.run(['bluetoothctl', 'power', 'on'])
            subprocess.run(['bluetoothctl', 'scan', 'on'])
            time.sleep(5)
            result = subprocess.run(['bluetoothctl', 'devices'], capture_output=True, text=True)
            subprocess.run(['bluetoothctl', 'scan', 'off'])
            devices = [line.split(' ', 2)[1:] for line in result.stdout.splitlines() if line.startswith('Device')]
            if not devices:
                label = QLabel('No devices found')
                label.setObjectName('list-label')
                self.bluetooth_list_layout.addWidget(label)
                return
            for device_id, name in devices:
                btn = QPushButton(f'{name} ({device_id})')
                btn.setObjectName('list-btn')
                btn.setFixedHeight(70)
                btn.clicked.connect(lambda checked, d=device_id: self.select_bluetooth(d))
                self.bluetooth_list_layout.addWidget(btn)
        except Exception as e:
            logging.error(f'Error scanning Bluetooth: {e}')
            label = QLabel('No devices found')
            label.setObjectName('list-label')
            self.bluetooth_list_layout.addWidget(label)

    def select_bluetooth(self, device_id):
        global selected_bluetooth
        selected_bluetooth = device_id
        for i in range(self.bluetooth_list_layout.count()):
            btn = self.bluetooth_list_layout.itemAt(i).widget()
            if btn and btn.text().endswith(f'({device_id})'):
                btn.setStyleSheet("background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #3d3d3d, stop:1 #2a2a2a);")
            else:
                btn.setStyleSheet("")

    def pair_bluetooth(self):
        global selected_bluetooth
        if not selected_bluetooth:
            QMessageBox.warning(self, "Warning", get_text('no_selection'))
            return
        try:
            result_pair = subprocess.run(['bluetoothctl', 'pair', selected_bluetooth], capture_output=True, text=True)
            if result_pair.returncode != 0:
                QMessageBox.warning(self, "Warning", get_text('pairing_failed', {'error': result_pair.stderr}))
                logging.error(f'Bluetooth pairing failed: {result_pair.stderr}')
                return
            result_connect = subprocess.run(['bluetoothctl', 'connect', selected_bluetooth], capture_output=True, text=True)
            if result_connect.returncode != 0:
                QMessageBox.warning(self, "Warning", get_text('pairing_failed', {'error': result_connect.stderr}))
                logging.error(f'Bluetooth connection failed: {result_connect.stderr}')
            else:
                QMessageBox.information(self, "Info", get_text('pairing', {'device': selected_bluetooth}))
        except Exception as e:
            QMessageBox.warning(self, "Warning", get_text('pairing_failed', {'error': str(e)}))
            logging.error(f'Error pairing Bluetooth: {e}')

    def closeEvent(self, event):
        try:
            if self.parent():
                self.parent().showFullScreen()
                logging.info('Restoring main window in fullscreen mode on close')
            super().closeEvent(event)
        except Exception as e:
            logging.error(f'Error during close event: {e}')
