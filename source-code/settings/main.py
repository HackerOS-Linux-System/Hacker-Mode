#!/usr/bin/env python3
import sys
import subprocess
import os
from pathlib import Path
from PyQt6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QGridLayout, QPushButton, QLabel, QCheckBox, QComboBox,
    QLineEdit, QListWidget, QMessageBox, QFrame
)
from PyQt6.QtCore import Qt, QLocale
from PyQt6.QtGui import QFont

TRANSLATIONS = {
    "en": {
        "title": "Quick Settings",
        "audio": "Audio",
        "increase_volume": "Volume Up",
        "decrease_volume": "Volume Down",
        "toggle_mute": "Mute / Unmute",
        "display": "Display",
        "increase_brightness": "Brightness Up",
        "decrease_brightness": "Brightness Down",
        "toggle_theme": "Toggle Theme",
        "network": "Network",
        "wifi_settings": "Wi-Fi Networks",
        "toggle_wifi": "Toggle Wi-Fi",
        "bluetooth": "Bluetooth",
        "power": "Power Profile",
        "power_saving": "Power Saver",
        "balanced": "Balanced",
        "performance": "Performance",
        "general": "General",
        "gaming_tools": "Gaming Tools",
        "enable_gamescope": "Enable Gamescope",
        "enable_mangohud": "Enable MangoHUD",
        "enable_vkbasalt": "Enable vkBasalt",
        "connect": "Connect",
        "scan": "Scan Again",
        "close": "Close",
        "back": "Back to Hacker Mode",
        "no_networks": "No networks found",
        "no_selection": "Please select a network",
        "password": "Password (if required)",
        "wifi_on": "on",
        "wifi_off": "off",
        "language": "Language"
    },
    "pl": {
        "title": "Szybkie Ustawienia",
        "audio": "Dźwięk",
        "increase_volume": "Głośniej",
        "decrease_volume": "Ciszej",
        "toggle_mute": "Wycisz / Włącz",
        "display": "Ekran",
        "increase_brightness": "Jaśniej",
        "decrease_brightness": "Ciemniej",
        "toggle_theme": "Przełącz motyw",
        "network": "Sieć",
        "wifi_settings": "Sieci Wi-Fi",
        "toggle_wifi": "Włącz/Wyłącz Wi-Fi",
        "bluetooth": "Bluetooth",
        "power": "Profil zasilania",
        "power_saving": "Oszczędzanie",
        "balanced": "Zrównoważony",
        "performance": "Wydajność",
        "general": "Ogólne",
        "gaming_tools": "Narzędzia do gier",
        "enable_gamescope": "Włącz Gamescope",
        "enable_mangohud": "Włącz MangoHUD",
        "enable_vkbasalt": "Włącz vkBasalt",
        "connect": "Połącz",
        "scan": "Skanuj ponownie",
        "close": "Zamknij",
        "back": "Powrót do Hacker Mode",
        "no_networks": "Nie znaleziono sieci",
        "no_selection": "Wybierz sieć",
        "password": "Hasło (jeśli wymagane)",
        "wifi_on": "włączone",
        "wifi_off": "wyłączone",
        "language": "Język"
    }
}

class SettingsApp(QMainWindow):
    def __init__(self):
        super().__init__()
        self.lang = self.detect_language()
        self.t = TRANSLATIONS[self.lang]

        self.setWindowTitle(self.t["title"])
        self.showFullScreen()

        self.setStyleSheet(self.get_stylesheet())

        self.central_widget = QWidget()
        self.setCentralWidget(self.central_widget)
        self.main_layout = QVBoxLayout(self.central_widget)
        self.main_layout.setContentsMargins(40, 40, 40, 40)
        self.main_layout.setSpacing(30)

        self.create_header()
        self.create_main_grid()
        self.create_extra_panels()

        # Główny przycisk Back (na dole ekranu głównego)
        self.create_bottom_buttons()

    def keyPressEvent(self, event):
        if event.key() == Qt.Key.Key_Escape:
            if self.wifi_panel.isVisible() or self.bluetooth_panel.isVisible():
                self.hide_extra_panels()
            else:
                self.launch_hacker_mode_and_close()

    def detect_language(self):
        locale = QLocale.system().name().split('_')[0]
        return "pl" if locale == "pl" else "en"

    def get_stylesheet(self):
        return """
        QMainWindow {
            background: qlineargradient(x1:0, y1:0, x2:1, y2:1,
                                        stop:0 #0d1117, stop:1 #161b22);
        }
        QLabel {
            color: #e6edf3;
            font-family: 'Segoe UI', 'Arial', sans-serif;
        }
        QLabel#header-title {
            color: #58a6ff;
            font-size: 42px;
            font-weight: bold;
        }
        QLabel#card-title {
            color: #58a6ff;
            font-size: 22px;
            font-weight: bold;
        }
        QPushButton {
            background-color: rgba(88, 166, 255, 0.15);
            color: #58a6ff;
            border: 2px solid #58a6ff;
            border-radius: 16px;
            padding: 16px;
            font-size: 18px;
            min-height: 60px;
        }
        QPushButton:hover {
            background-color: #58a6ff;
            color: #0d1117;
        }
        QPushButton:pressed {
            background-color: #3e84db;
        }
        QPushButton#back-button {
            background-color: rgba(255, 100, 100, 0.2);
            color: #ff6b6b;
            border: 2px solid #ff6b6b;
            font-size: 20px;
            font-weight: bold;
        }
        QPushButton#back-button:hover {
            background-color: #ff6b6b;
            color: #0d1117;
        }
        QFrame#card {
            background-color: rgba(33, 41, 51, 0.6);
            border: 1px solid #30363d;
            border-radius: 20px;
        }
        QCheckBox {
            color: #e6edf3;
            font-size: 18px;
            spacing: 15px;
        }
        QCheckBox::indicator {
            width: 28px;
            height: 28px;
            border-radius: 10px;
            border: 2px solid #58a6ff;
        }
        QCheckBox::indicator:checked {
            background-color: #58a6ff;
        }
        QLineEdit {
            background-color: #0d1117;
            color: #e6edf3;
            border: 2px solid #58a6ff;
            border-radius: 12px;
            padding: 14px;
            font-size: 18px;
        }
        QListWidget {
            background-color: #0d1117;
            color: #e6edf3;
            border: 2px solid #58a6ff;
            border-radius: 16px;
            padding: 10px;
            font-size: 18px;
        }
        QListWidget::item {
            padding: 14px;
        }
        QListWidget::item:selected {
            background-color: #58a6ff;
            color: #0d1117;
        }
        QComboBox {
            background-color: #0d1117;
            color: #e6edf3;
            border: 2px solid #58a6ff;
            border-radius: 12px;
            padding: 14px;
            font-size: 18px;
        }
        """

    def create_header(self):
        header = QLabel(self.t["title"])
        header.setObjectName("header-title")
        header.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.main_layout.addWidget(header)

    def create_card(self, title_text):
        card = QFrame()
        card.setObjectName("card")
        card.setMinimumSize(450, 350)
        layout = QVBoxLayout(card)
        layout.setSpacing(25)
        layout.setContentsMargins(35, 35, 35, 35)

        title = QLabel(title_text)
        title.setObjectName("card-title")
        title.setAlignment(Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(title)

        return card, layout

    def create_main_grid(self):
        grid = QGridLayout()
        grid.setSpacing(40)

        # Audio
        audio_card, audio_l = self.create_card("🔊 " + self.t["audio"])
        audio_l.addWidget(self.create_button(self.t["increase_volume"], "pactl set-sink-volume @DEFAULT_SINK@ +5%"))
        audio_l.addWidget(self.create_button(self.t["decrease_volume"], "pactl set-sink-volume @DEFAULT_SINK@ -5%"))
        audio_l.addWidget(self.create_button(self.t["toggle_mute"], "pactl set-sink-mute @DEFAULT_SINK@ toggle"))
        grid.addWidget(audio_card, 0, 0)

        # Display
        disp_card, disp_l = self.create_card("🖥️ " + self.t["display"])
        disp_l.addWidget(self.create_button(self.t["increase_brightness"], "brightnessctl set +10%"))
        disp_l.addWidget(self.create_button(self.t["decrease_brightness"], "brightnessctl set 10%-"))
        disp_l.addWidget(self.create_button(self.t["toggle_theme"], "swaymsg reload"))
        grid.addWidget(disp_card, 0, 1)

        # Network
        net_card, net_l = self.create_card("🌐 " + self.t["network"])
        net_l.addWidget(self.create_button(self.t["wifi_settings"], self.show_wifi_panel))
        net_l.addWidget(self.create_button(self.t["toggle_wifi"], self.toggle_wifi))
        net_l.addWidget(self.create_button(self.t["bluetooth"], self.show_bluetooth_panel))
        grid.addWidget(net_card, 1, 0)

        # Power
        power_card, power_l = self.create_card("🔋 " + self.t["power"])
        power_l.addWidget(self.create_button(self.t["power_saving"], "powerprofilesctl set power-saver"))
        power_l.addWidget(self.create_button(self.t["balanced"], "powerprofilesctl set balanced"))
        power_l.addWidget(self.create_button(self.t["performance"], "powerprofilesctl set performance"))
        grid.addWidget(power_card, 1, 1)

        # General
        gen_card, gen_l = self.create_card("⚙️ " + self.t["general"])
        lang_label = QLabel(self.t["language"])
        lang_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        lang_label.setStyleSheet("font-size: 18px;")
        gen_l.addWidget(lang_label)
        self.lang_combo = QComboBox()
        self.lang_combo.addItems(["English", "Polski"])
        self.lang_combo.setCurrentIndex(1 if self.lang == "pl" else 0)
        self.lang_combo.currentIndexChanged.connect(self.change_language)
        gen_l.addWidget(self.lang_combo)
        grid.addWidget(gen_card, 2, 0)

        # Gaming
        game_card, game_l = self.create_card("🎮 " + self.t["gaming_tools"])
        game_l.addWidget(self.create_checkbox(self.t["enable_gamescope"]))
        game_l.addWidget(self.create_checkbox(self.t["enable_mangohud"]))
        game_l.addWidget(self.create_checkbox(self.t["enable_vkbasalt"]))
        grid.addWidget(game_card, 2, 1)

        self.main_layout.addLayout(grid)

    def create_button(self, text, cmd_or_func):
        btn = QPushButton(text)
        if callable(cmd_or_func):
            btn.clicked.connect(cmd_or_func)
        else:
            btn.clicked.connect(lambda checked=False, c=cmd_or_func: self.run_cmd(c))
        return btn

    def create_checkbox(self, text):
        cb = QCheckBox(text)
        cb.setStyleSheet("QCheckBox { padding: 10px; }")
        return cb

    def run_cmd(self, cmd):
        try:
            subprocess.run(cmd, shell=True, check=False)
        except Exception as e:
            QMessageBox.critical(self, "Error", str(e))

    def run_cmd_with_output(self, cmd):
        try:
            result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=15)
            return result.returncode == 0, result.stdout, result.stderr
        except:
            return False, "", ""

    def change_language(self, index):
        new_lang = "pl" if index == 1 else "en"
        if new_lang != self.lang:
            self.lang = new_lang
            self.t = TRANSLATIONS[self.lang]
            self.rebuild_ui()

    def rebuild_ui(self):
        # Usuń wszystkie widgety oprócz bottom buttons
        for i in reversed(range(self.main_layout.count() - 1)):  # -1 bo ostatni to bottom_layout
            item = self.main_layout.itemAt(i)
            if item:
                widget = item.widget()
                if widget:
                    widget.setParent(None)
        self.create_header()
        self.create_main_grid()
        self.create_extra_panels(rebuild=True)

    def create_extra_panels(self, rebuild=False):
        if rebuild or not hasattr(self, 'wifi_panel'):
            if hasattr(self, 'wifi_panel'):
                self.wifi_panel.deleteLater()
            if hasattr(self, 'bluetooth_panel'):
                self.bluetooth_panel.deleteLater()

            # Wi-Fi Panel
            self.wifi_panel = QFrame()
            self.wifi_panel.setObjectName("card")
            wifi_layout = QVBoxLayout(self.wifi_panel)
            wifi_layout.setContentsMargins(50, 50, 50, 50)
            wifi_layout.setSpacing(25)

            back_btn_wifi = QPushButton("← " + self.t["back"])
            back_btn_wifi.clicked.connect(self.hide_extra_panels)
            back_btn_wifi.setFixedHeight(70)
            wifi_layout.addWidget(back_btn_wifi)

            wifi_layout.addWidget(QLabel("📶 " + self.t["wifi_settings"]))
            self.wifi_list = QListWidget()
            wifi_layout.addWidget(self.wifi_list)

            pass_layout = QHBoxLayout()
            self.wifi_pass = QLineEdit()
            self.wifi_pass.setPlaceholderText(self.t["password"])
            self.wifi_pass.setEchoMode(QLineEdit.EchoMode.Password)
            pass_layout.addWidget(self.wifi_pass)

            connect_btn = QPushButton(self.t["connect"])
            connect_btn.clicked.connect(self.connect_wifi)
            pass_layout.addWidget(connect_btn)
            wifi_layout.addLayout(pass_layout)

            self.main_layout.addWidget(self.wifi_panel)

            # Bluetooth Panel
            self.bluetooth_panel = QFrame()
            self.bluetooth_panel.setObjectName("card")
            bt_layout = QVBoxLayout(self.bluetooth_panel)
            bt_layout.setContentsMargins(50, 50, 50, 50)

            back_btn_bt = QPushButton("← " + self.t["back"])
            back_btn_bt.clicked.connect(self.hide_extra_panels)
            back_btn_bt.setFixedHeight(70)
            bt_layout.addWidget(back_btn_bt)

            bt_layout.addWidget(QLabel("🔵 " + self.t["bluetooth"]))
            self.bt_list = QListWidget()
            bt_layout.addWidget(self.bt_list)

            scan_btn = QPushButton(self.t["scan"])
            scan_btn.clicked.connect(self.scan_bluetooth)
            bt_layout.addWidget(scan_btn)

            self.main_layout.addWidget(self.bluetooth_panel)

        self.hide_extra_panels()

    def show_wifi_panel(self):
        self.scan_wifi()
        self.wifi_panel.setVisible(True)
        self.bluetooth_panel.setVisible(False)

    def show_bluetooth_panel(self):
        self.scan_bluetooth()
        self.bluetooth_panel.setVisible(True)
        self.wifi_panel.setVisible(False)

    def hide_extra_panels(self):
        self.wifi_panel.setVisible(False)
        self.bluetooth_panel.setVisible(False)

    def toggle_wifi(self):
        success, stdout, _ = self.run_cmd_with_output("nmcli radio wifi")
        if success:
            state = stdout.strip().lower()
            new_state = "off" if "enabled" in state else "on"
            self.run_cmd(f"nmcli radio wifi {new_state}")
            status = self.t["wifi_on"] if new_state == "on" else self.t["wifi_off"]
            QMessageBox.information(self, "Wi-Fi", f"Wi-Fi: {status}")

    def scan_wifi(self):
        self.wifi_list.clear()
        success, stdout, _ = self.run_cmd_with_output("nmcli -t -f SSID,SIGNAL dev wifi list")
        if not success or not stdout.strip():
            self.wifi_list.addItem(self.t["no_networks"])
            return
        for line in stdout.splitlines():
            if ':' in line:
                ssid, signal = line.split(':', 1)
                ssid = ssid.strip()
                if ssid:
                    self.wifi_list.addItem(f"{ssid}  ({signal.strip()}%)")

    def connect_wifi(self):
        item = self.wifi_list.currentItem()
        if not item or self.t["no_networks"] in item.text():
            QMessageBox.warning(self, "Error", self.t["no_selection"])
            return
        ssid = item.text().split("  (")[0]
        password = self.wifi_pass.text()
        cmd = f'nmcli dev wifi connect "{ssid}"'
        if password:
            cmd += f' password "{password}"'
        success, _, stderr = self.run_cmd_with_output(cmd)
        if success:
            QMessageBox.information(self, "Success", f"Connecting to {ssid}...")
        else:
            QMessageBox.critical(self, "Error", stderr.strip() or "Connection failed")

    def scan_bluetooth(self):
        self.bt_list.clear()
        success, stdout, _ = self.run_cmd_with_output("bluetoothctl devices")
        if not success or not stdout.strip():
            self.bt_list.addItem("No devices found")
            return
        for line in stdout.splitlines():
            if line.startswith("Device"):
                parts = line.split(" ", 2)
                if len(parts) >= 3:
                    self.bt_list.addItem(f"{parts[2]} ({parts[1]})")

    def create_bottom_buttons(self):
        bottom_layout = QHBoxLayout()
        bottom_layout.addStretch()

        back_button = QPushButton(self.t["back"])
        back_button.setObjectName("back-button")
        back_button.setFixedSize(400, 80)
        back_button.clicked.connect(self.launch_hacker_mode_and_close)
        bottom_layout.addWidget(back_button)

        close_button = QPushButton(self.t["close"])
        close_button.setFixedSize(200, 80)
        close_button.clicked.connect(self.close)
        bottom_layout.addWidget(close_button)

        self.main_layout.addLayout(bottom_layout)

    def launch_hacker_mode_and_close(self):
        appimage_path = Path.home() / ".hackeros" / "Hacker-Mode" / "Hacker-Mode.AppImage"
        if appimage_path.exists():
            try:
                subprocess.Popen([str(appimage_path)], start_new_session=True)
            except Exception as e:
                QMessageBox.critical(self, "Error", f"Failed to launch Hacker-Mode:\n{str(e)}")
        else:
            QMessageBox.warning(self, "Not found", f"AppImage not found:\n{appimage_path}")
        self.close()

if __name__ == "__main__":
    app = QApplication(sys.argv)
    app.setFont(QFont("Segoe UI", 11))
    window = SettingsApp()
    window.show()
    sys.exit(app.exec())
