import sys
import os
import subprocess
from PySide6.QtWidgets import QApplication, QMainWindow, QLabel, QPushButton, QGridLayout, QVBoxLayout, QWidget, QHBoxLayout, QMenu, QGraphicsOpacityEffect
from PySide6.QtGui import QPixmap, QFont
from PySide6.QtCore import Qt, QPropertyAnimation, QRect, QUrl, QTimer, QEasingCurve
from PySide6.QtNetwork import QNetworkAccessManager, QNetworkRequest, QNetworkReply
from utils import setup_language, get_text, logging
from launchers import launch_app, system_action
from settings import SettingsWindow
class IconLoader:
    def __init__(self):
        self.manager = QNetworkAccessManager()
        self.icons = {}
    def load_icon(self, url, callback):
        if not url:
            logging.error("Icon URL is empty")
            callback(QPixmap()) # Return empty pixmap as fallback
            return
        request = QNetworkRequest(QUrl(url))
        reply = self.manager.get(request)
        if reply:
            reply.finished.connect(lambda: self.icon_loaded(reply, callback))
        else:
            logging.error(f"Failed to create network reply for URL: {url}")
            callback(QPixmap()) # Return empty pixmap as fallback
    def icon_loaded(self, reply, callback):
        if reply.error() == QNetworkReply.NoError:
            pixmap = QPixmap()
            pixmap.loadFromData(reply.readAll())
            if not pixmap.isNull():
                callback(pixmap.scaled(140, 140, Qt.KeepAspectRatio, Qt.SmoothTransformation))
            else:
                logging.error(f"Failed to load pixmap from URL: {reply.url().toString()}")
                callback(QPixmap()) # Fallback empty pixmap
        else:
            logging.error(f"Failed to load icon: {reply.errorString()}")
            callback(QPixmap()) # Fallback empty pixmap
        reply.deleteLater()
icon_loader = IconLoader()
class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle(get_text('title'))
        self.showFullScreen()
        self.central_widget = QWidget()
        self.setCentralWidget(self.central_widget)
        self.layout = QVBoxLayout(self.central_widget)
        self.layout.setAlignment(Qt.AlignCenter)
        self.layout.setSpacing(60)
        self.layout.setContentsMargins(80, 80, 80, 80)
        self.top_layout = QHBoxLayout()
        self.hackeros_logo = QLabel()
        self.hackeros_logo.setObjectName('logo')
        pixmap_hackeros = QPixmap('/usr/share/HackerOS/ICONS/HackerOS.png')
        if pixmap_hackeros.isNull():
            logging.error("Failed to load HackerOS.png")
            pixmap_hackeros = QPixmap(120, 120) # Fallback empty pixmap
            pixmap_hackeros.fill(Qt.transparent)
        self.hackeros_logo.setPixmap(pixmap_hackeros.scaled(120, 120, Qt.KeepAspectRatio, Qt.SmoothTransformation))
        self.hackeros_logo_opacity = QGraphicsOpacityEffect()
        self.hackeros_logo.setGraphicsEffect(self.hackeros_logo_opacity)
        self.top_layout.addWidget(self.hackeros_logo, alignment=Qt.AlignLeft)
        self.top_layout.addStretch()
        self.hackermode_logo = QLabel()
        self.hackermode_logo.setObjectName('logo')
        pixmap_hackermode = QPixmap('/usr/share/HackerOS/ICONS/Hacker-Mode.png')
        if pixmap_hackermode.isNull():
            logging.error("Failed to load Hacker-Mode.png")
            pixmap_hackermode = QPixmap(120, 120) # Fallback empty pixmap
            pixmap_hackermode.fill(Qt.transparent)
        self.hackermode_logo.setPixmap(pixmap_hackermode.scaled(120, 120, Qt.KeepAspectRatio, Qt.SmoothTransformation))
        self.hackermode_logo_opacity = QGraphicsOpacityEffect()
        self.hackermode_logo.setGraphicsEffect(self.hackermode_logo_opacity)
        self.top_layout.addWidget(self.hackermode_logo, alignment=Qt.AlignRight)
        self.layout.addLayout(self.top_layout)
        self.title = QLabel(get_text('title'))
        self.title.setFont(QFont('Hack', 70, QFont.Bold))
        self.title.setObjectName('neon-text')
        self.layout.addWidget(self.title, alignment=Qt.AlignCenter)
        self.grid_layout = QGridLayout()
        self.grid_layout.setSpacing(80)
        self.grid_layout.setContentsMargins(60, 60, 60, 60)
        self.layout.addLayout(self.grid_layout)
        self.launcher_buttons = []
        self.add_launcher_button('steam', 'https://store.steampowered.com/favicon.ico', 0, 0)
        self.add_launcher_button('heroic', 'https://www.heroicgameslauncher.com/favicon.ico', 0, 1)
        self.add_launcher_button('hyperplay', 'https://www.hyperplaygaming.com/favicon.ico', 0, 2)
        self.add_launcher_button('lutris', 'https://lutris.net/static/images/logo.png', 0, 3)
        self.bottom_layout = QHBoxLayout()
        self.hacker_menu_btn = QPushButton(get_text('hacker_menu'))
        self.hacker_menu_btn.setObjectName('action-btn')
        self.hacker_menu_btn.setFixedSize(250, 80)
        self.hacker_menu_btn.clicked.connect(self.show_hacker_menu)
        self.bottom_layout.addWidget(self.hacker_menu_btn, alignment=Qt.AlignLeft)
        self.bottom_layout.addStretch()
        self.settings_btn = QPushButton(get_text('settings'))
        self.settings_btn.setObjectName('action-btn')
        self.settings_btn.setFixedSize(250, 80)
        self.settings_btn.clicked.connect(self.launch_settings)
        self.bottom_layout.addWidget(self.settings_btn, alignment=Qt.AlignRight)
        self.layout.addLayout(self.bottom_layout)
        self.update_texts()
        QTimer.singleShot(100, self.animate_elements)
    def animate_elements(self):
        for i, btn in enumerate(self.launcher_buttons):
            anim = QPropertyAnimation(btn, b"geometry")
            rect = btn.geometry()
            anim.setStartValue(QRect(rect.x(), rect.y() + 300, rect.width(), rect.height()))
            anim.setEndValue(rect)
            anim.setDuration(2000)
            anim.setEasingCurve(QEasingCurve.OutQuint)
            QTimer.singleShot(i * 400, lambda: anim.start())
        logo_anim1 = QPropertyAnimation(self.hackeros_logo_opacity, b"opacity")
        logo_anim1.setStartValue(0.0)
        logo_anim1.setEndValue(1.0)
        logo_anim1.setDuration(2000)
        logo_anim1.setEasingCurve(QEasingCurve.OutQuint)
        QTimer.singleShot(400, lambda: logo_anim1.start())
        logo_anim2 = QPropertyAnimation(self.hackermode_logo_opacity, b"opacity")
        logo_anim2.setStartValue(0.0)
        logo_anim2.setEndValue(1.0)
        logo_anim2.setDuration(2000)
        logo_anim2.setEasingCurve(QEasingCurve.OutQuint)
        QTimer.singleShot(800, lambda: logo_anim2.start())
    def add_launcher_button(self, app_name, icon_url, row, col):
        btn = QPushButton(objectName='launcher-btn')
        btn.setFixedSize(280, 280)
        layout = QVBoxLayout(btn)
        layout.setContentsMargins(30, 30, 30, 30)
        icon_label = QLabel()
        icon_label.setAlignment(Qt.AlignCenter)
        layout.addWidget(icon_label)
        text_label = QLabel(get_text(app_name))
        text_label.setFont(QFont('Hack', 24))
        text_label.setAlignment(Qt.AlignCenter)
        layout.addWidget(text_label)
        icon_loader.load_icon(icon_url, icon_label.setPixmap)
        btn.clicked.connect(lambda: launch_app(app_name, self))
        self.grid_layout.addWidget(btn, row, col, alignment=Qt.AlignCenter)
        self.launcher_buttons.append(btn)
    def update_texts(self):
        self.setWindowTitle(get_text('title'))
        self.title.setText(get_text('title'))
        self.settings_btn.setText(get_text('settings'))
        self.hacker_menu_btn.setText(get_text('hacker_menu'))
        for btn in self.launcher_buttons:
            label = btn.findChildren(QLabel)[1]
            key = label.text().lower()
            label.setText(get_text(key))
    def launch_settings(self):
        self.hide()
        self.settings_window = SettingsWindow(self)
        self.settings_window.showFullScreen()
    def show_hacker_menu(self):
        menu = QMenu(self)
        menu.setObjectName('hacker-menu')
        actions = [
            ('switch_to_plasma', lambda: system_action('switchToPlasma', self)),
            ('shutdown', lambda: system_action('shutdown')),
            ('restart', lambda: system_action('restart')),
            ('sleep', lambda: system_action('sleep')),
            ('restart_apps', lambda: system_action('restartApps', self)),
            ('restart_app', lambda: system_action('restartApps', self))
        ]
        for text, callback in actions:
            action = menu.addAction(get_text(text))
            action.triggered.connect(callback)
        menu.exec(self.hacker_menu_btn.mapToGlobal(self.hacker_menu_btn.rect().bottomLeft()))
if __name__ == '__main__':
    app = QApplication(sys.argv)
    stylesheet = """
    QMainWindow, QWidget {
        background: qradialgradient(cx:0.5, cy:0.5, radius:1.5, stop:0 #1a1a1a, stop:1 #000000);
        color: white;
        font-family: 'Hack', 'Courier New', monospace;
    }
    #neon-text {
        color: #ffffff;
        font-weight: bold;
    }
    #logo {
        border: 3px solid #ffffff;
        border-radius: 15px;
        background: rgba(255, 255, 255, 0.1);
    }
    QPushButton#launcher-btn {
        background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #2a2a2a, stop:1 #151515);
        border-radius: 40px;
        padding: 35px;
        border: 3px solid #ffffff;
        background: rgba(255, 255, 255, 0.1);
    }
    QPushButton#launcher-btn:hover {
        background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #3d3d3d, stop:1 #252525);
    }
    QWidget#setting-panel {
        background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #2a2a2a, stop:1 #151515);
        border-radius: 40px;
        padding: 35px;
        border: 3px solid #ffffff;
        background: rgba(255, 255, 255, 0.1);
    }
    QPushButton#action-btn, QPushButton#setting-btn, QPushButton#list-btn {
        background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #2a2a2a, stop:1 #151515);
        border-radius: 20px;
        padding: 18px;
        color: white;
        font-size: 24px;
        border: 3px solid #ffffff;
        background: rgba(255, 255, 255, 0.1);
    }
    QPushButton#action-btn:hover, QPushButton#setting-btn:hover, QPushButton#list-btn:hover {
        background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #3d3d3d, stop:1 #252525);
    }
    QLabel {
        color: white;
        font-size: 22px;
    }
    QLabel#list-label {
        font-size: 20px;
        color: #dddddd;
    }
    QComboBox, QLineEdit#input-field {
        background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #2a2a2a, stop:1 #151515);
        color: white;
        border-radius: 15px;
        padding: 15px;
        border: 2px solid #ffffff;
        font-size: 22px;
        background: rgba(255, 255, 255, 0.1);
    }
    QScrollArea {
        background: transparent;
        border: none;
    }
    QCheckBox#checkbox {
        color: white;
        font-size: 22px;
    }
    QMenu#hacker-menu {
        background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #2a2a2a, stop:1 #151515);
        border-radius: 20px;
        border: 3px solid #ffffff;
        color: white;
        padding: 15px;
        background: rgba(255, 255, 255, 0.1);
    }
    QMenu#hacker-menu::item {
        padding: 15px 30px;
        border-radius: 12px;
        font-size: 22px;
    }
    QMenu#hacker-menu::item:hover {
        background: #3d3d3d;
        color: #ffffff;
    }
    QScrollArea > QWidget > QWidget {
        background: transparent;
    }
    """
    app.setStyleSheet(stylesheet)
    setup_language()
    main_window = MainWindow()
    main_window.show()
    try:
        subprocess.run(['wf-shell', 'fullscreen', 'enable'])
    except Exception as e:
        logging.error(f'Error setting fullscreen in Wayfire: {e}')
    sys.exit(app.exec())
