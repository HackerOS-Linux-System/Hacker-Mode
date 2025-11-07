from PySide6.QtWidgets import QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QTableWidget, QTableWidgetItem, QPushButton, QTabWidget, QComboBox, QLineEdit, QLabel, QMessageBox, QFileDialog, QDialog, QGridLayout, QProgressDialog, QCheckBox, QTextEdit, QMenu, QToolButton, QStackedWidget
from PySide6.QtCore import Qt, QThread, Signal
from PySide6.QtGui import QIcon, QCloseEvent, QKeyEvent, QAction
import os
import subprocess
import shutil
import logging
from proton_manager import ProtonManager
from game_manager import GameManager
from config_manager import ConfigManager
class InstallThread(QThread):
    update_progress = Signal(str, int, int)
    finished = Signal(bool, str)
    def __init__(self, manager, version, proton_type, custom_path=None, custom_type=None):
        super().__init__()
        self.manager = manager
        self.version = version
        self.proton_type = proton_type
        self.custom_path = custom_path
        self.custom_type = custom_type
    def run(self):
        def progress_callback(stage, value, total):
            self.update_progress.emit(stage, value, total)
        try:
            if self.proton_type in ['GE', 'Official', 'Experimental']:
                success, message = self.manager.install_proton(self.version, self.proton_type if self.proton_type != 'Experimental' else 'Official', progress_callback)
            else: # Custom
                if self.custom_type == 'Tar.gz File':
                    success, message = self.manager.install_custom_tar(self.custom_path, self.version, progress_callback)
                else:
                    success, message = self.manager.install_custom_folder(self.custom_path, self.version)
            self.finished.emit(success, message)
        except Exception as e:
            self.finished.emit(False, str(e))
class LoadProtonsThread(QThread):
    protons_loaded = Signal(list)
    def __init__(self, manager):
        super().__init__()
        self.manager = manager
    def run(self):
        try:
            protons = self.manager.get_installed_protons()
            self.protons_loaded.emit(protons)
        except Exception as e:
            logging.error(f"Error loading protons: {e}")
            print(f"Error loading protons: {e}")
            self.protons_loaded.emit([])
class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle('Hacker Mode')
        self.resize(1000, 800)
        self.config_manager = ConfigManager()
        self.proton_manager = ProtonManager()
        self.game_manager = GameManager(self.proton_manager)
        self.games = []
        self.settings = self.config_manager.settings # Use stored settings
        self.setup_ui()
        self.load_games()
        self.start_proton_loading()
        self.showFullScreen()
    def closeEvent(self, event: QCloseEvent):
        threads = []
        if hasattr(self, 'proton_thread') and self.proton_thread.isRunning():
            threads.append(self.proton_thread)
        if hasattr(self, 'install_thread') and self.install_thread.isRunning():
            threads.append(self.install_thread)
        if hasattr(self, 'update_thread') and self.update_thread.isRunning():
            threads.append(self.update_thread)
        for thread in threads:
            thread.requestInterruption()
            thread.quit()
            thread.wait()
        super().closeEvent(event)
    def keyPressEvent(self, event: QKeyEvent):
        current_index = self.tabs.currentIndex()
        if event.key() in (Qt.Key_Up, Qt.Key_Down):
            if current_index == self.games_tab_index:
                table = self.games_list
            elif current_index == self.protons_tab_index:
                table = self.protons_table
            else:
                return super().keyPressEvent(event)
            row = table.currentRow()
            if event.key() == Qt.Key_Up:
                row -= 1
            else:
                row += 1
            if 0 <= row < table.rowCount():
                table.setCurrentCell(row, table.currentColumn())
        elif event.key() in (Qt.Key_Left, Qt.Key_Right):
            index = self.tabs.currentIndex()
            if event.key() == Qt.Key_Left:
                index -= 1
            else:
                index += 1
            if 0 <= index < self.tabs.count():
                self.tabs.setCurrentIndex(index)
        else:
            super().keyPressEvent(event)
    def setup_ui(self):
        central = QWidget()
        layout = QVBoxLayout(central)
        self.tabs = QTabWidget()
        # Games tab
        games_widget = QWidget()
        games_layout = QVBoxLayout(games_widget)
        self.games_stack = QStackedWidget()
        games_layout.addWidget(self.games_stack)
        self.games_main_page = QWidget()
        games_main_layout = QVBoxLayout(self.games_main_page)
        games_main_layout.addWidget(QLabel("Installed Games"))
        self.games_list = QTableWidget(0, 3)
        self.games_list.setHorizontalHeaderLabels(['Game Name', 'Runner', 'Launch Options'])
        self.games_list.horizontalHeader().setStretchLastSection(True)
        self.games_list.setSelectionBehavior(QTableWidget.SelectRows)
        games_main_layout.addWidget(self.games_list)
        buttons_layout = QHBoxLayout()
        add_btn = QPushButton(QIcon.fromTheme("list-add"), 'Add Game')
        add_btn.clicked.connect(self.show_add_game)
        add_btn.setToolTip("Add a new game to the launcher")
        launch_btn = QPushButton(QIcon.fromTheme("media-playback-start"), 'Launch Game')
        launch_btn.clicked.connect(self.launch_game)
        launch_btn.setToolTip("Launch the selected game")
        remove_btn = QPushButton(QIcon.fromTheme("edit-delete"), 'Remove Game')
        remove_btn.clicked.connect(self.remove_game)
        remove_btn.setToolTip("Remove the selected game")
        config_btn = QPushButton(QIcon.fromTheme("configure"), 'Configure Game')
        config_btn.clicked.connect(self.show_configure_game)
        config_btn.setToolTip("Edit the selected game's configuration")
        buttons_layout.addWidget(add_btn)
        buttons_layout.addWidget(launch_btn)
        buttons_layout.addWidget(remove_btn)
        buttons_layout.addWidget(config_btn)
        games_main_layout.addLayout(buttons_layout)
        self.games_stack.addWidget(self.games_main_page)
        self.games_tab_index = self.tabs.addTab(games_widget, 'Games')
        # Protons tab
        protons_widget = QWidget()
        protons_layout = QVBoxLayout(protons_widget)
        self.protons_stack = QStackedWidget()
        protons_layout.addWidget(self.protons_stack)
        self.protons_main_page = QWidget()
        protons_main_layout = QVBoxLayout(self.protons_main_page)
        protons_main_layout.addWidget(QLabel("Installed Protons"))
        self.protons_table = QTableWidget(0, 4)
        self.protons_table.setHorizontalHeaderLabels(['Version', 'Type', 'Installed Date', 'Status'])
        self.protons_table.horizontalHeader().setStretchLastSection(True)
        self.protons_table.setSelectionBehavior(QTableWidget.SelectRows)
        protons_main_layout.addWidget(self.protons_table)
        buttons_layout = QHBoxLayout()
        install_btn = QPushButton(QIcon.fromTheme("list-add"), 'Install Proton')
        install_btn.clicked.connect(self.show_install_proton)
        install_btn.setToolTip("Install a new Proton version")
        update_btn = QPushButton(QIcon.fromTheme("view-refresh"), 'Update Selected')
        update_btn.clicked.connect(self.update_proton)
        update_btn.setToolTip("Check and update the selected Proton")
        remove_btn = QPushButton(QIcon.fromTheme("edit-delete"), 'Remove Selected')
        remove_btn.clicked.connect(self.remove_proton)
        remove_btn.setToolTip("Remove the selected Proton")
        refresh_btn = QPushButton(QIcon.fromTheme("view-refresh"), 'Refresh')
        refresh_btn.clicked.connect(self.start_proton_loading)
        refresh_btn.setToolTip("Refresh the list of installed Protons")
        self.stable_check = QCheckBox("Stable Only")
        self.stable_check.setChecked(True)
        self.stable_check.stateChanged.connect(self.start_proton_loading)
        buttons_layout.addWidget(install_btn)
        buttons_layout.addWidget(update_btn)
        buttons_layout.addWidget(remove_btn)
        buttons_layout.addWidget(refresh_btn)
        buttons_layout.addWidget(self.stable_check)
        protons_main_layout.addLayout(buttons_layout)
        self.protons_stack.addWidget(self.protons_main_page)
        self.protons_tab_index = self.tabs.addTab(protons_widget, 'Protons')
        # Settings tab
        settings_widget = QWidget()
        settings_layout = QGridLayout(settings_widget)
        settings_layout.addWidget(QLabel("Settings"), 0, 0, 1, 2)
        row = 1
        theme_label = QLabel("Theme:")
        self.theme_combo = QComboBox()
        self.theme_combo.addItems(['Dark (Default)', 'Light'])
        self.theme_combo.setCurrentText(self.settings.get('theme', 'Dark (Default)'))
        self.theme_combo.setObjectName('theme_combo')
        self.theme_combo.currentTextChanged.connect(self.update_settings)
        settings_layout.addWidget(theme_label, row, 0)
        settings_layout.addWidget(self.theme_combo, row, 1)
        row += 1
        runner_label = QLabel("Default Runner:")
        self.runner_combo = QComboBox()
        self.runner_combo.addItems(['Native', 'Wine', 'Proton', 'Flatpak', 'Steam'])
        self.runner_combo.setCurrentText(self.settings['default_runner'])
        self.runner_combo.setObjectName('runner_combo')
        self.runner_combo.currentTextChanged.connect(self.update_settings)
        settings_layout.addWidget(runner_label, row, 0)
        settings_layout.addWidget(self.runner_combo, row, 1)
        row += 1
        update_label = QLabel("Auto-check Updates:")
        self.update_combo = QComboBox()
        self.update_combo.addItems(['Enabled', 'Disabled'])
        self.update_combo.setCurrentText(self.settings['auto_update'])
        self.update_combo.setObjectName('update_combo')
        self.update_combo.currentTextChanged.connect(self.update_settings)
        settings_layout.addWidget(update_label, row, 0)
        settings_layout.addWidget(self.update_combo, row, 1)
        row += 1
        esync_check = QCheckBox("Enable Esync (Global)")
        esync_check.setChecked(self.settings['enable_esync'])
        esync_check.setObjectName('esync_check')
        esync_check.stateChanged.connect(self.update_settings)
        settings_layout.addWidget(esync_check, row, 0)
        row += 1
        fsync_check = QCheckBox("Enable Fsync (Global)")
        fsync_check.setChecked(self.settings['enable_fsync'])
        fsync_check.setObjectName('fsync_check')
        fsync_check.stateChanged.connect(self.update_settings)
        settings_layout.addWidget(fsync_check, row, 0)
        row += 1
        dxvk_async_check = QCheckBox("Enable DXVK Async (Global)")
        dxvk_async_check.setChecked(self.settings['enable_dxvk_async'])
        dxvk_async_check.setObjectName('dxvk_async_check')
        dxvk_async_check.stateChanged.connect(self.update_settings)
        settings_layout.addWidget(dxvk_async_check, row, 0)
        row += 1
        prefix_label = QLabel("Prefixes Location:")
        prefix_value = QLabel(self.config_manager.prefixes_dir)
        prefix_value.setStyleSheet("color: #888888;")
        settings_layout.addWidget(prefix_label, row, 0)
        settings_layout.addWidget(prefix_value, row, 1)
        row += 1
        protons_label = QLabel("Protons Location:")
        protons_value = QLabel(self.config_manager.protons_dir)
        protons_value.setStyleSheet("color: #888888;")
        settings_layout.addWidget(protons_label, row, 0)
        settings_layout.addWidget(protons_value, row, 1)
        settings_layout.setRowStretch(row, 1)
        self.tabs.addTab(settings_widget, 'Settings')
        # About tab
        about_widget = QWidget()
        about_layout = QVBoxLayout(about_widget)
        about_text = QTextEdit()
        about_text.setReadOnly(True)
        about_text.setText("Hacker Mode v1.0\nGitHub: https://github.com/your-repo\nA launcher for running games with Proton/Wine easily.")
        about_layout.addWidget(about_text)
        self.tabs.addTab(about_widget, 'About')
        # Hacker Mode button
        menu = QMenu(self)
        action_plasma = QAction('Switch to Plasma', self)
        action_plasma.triggered.connect(lambda: subprocess.run(['sudo', '/usr/share/HackerOS/Scripts/Bin/revert_to_plasma.sh']))
        action_shutdown = QAction('Wyłącz', self)
        action_shutdown.triggered.connect(lambda: subprocess.run(['sudo', 'shutdown', '-h', 'now']))
        action_reboot = QAction('Uruchom ponownie', self)
        action_reboot.triggered.connect(lambda: subprocess.run(['sudo', 'reboot']))
        action_logout = QAction('Log out', self)
        action_logout.triggered.connect(lambda: subprocess.run(['qdbus', 'org.kde.ksmserver', '/KSMServer', 'logout', '0', '0', '0']))
        menu.addAction(action_plasma)
        menu.addAction(action_shutdown)
        menu.addAction(action_reboot)
        menu.addAction(action_logout)
        button = QToolButton(self)
        button.setText('Hacker Mode')
        button.setPopupMode(QToolButton.InstantPopup)
        button.setMenu(menu)
        self.tabs.setCornerWidget(button, Qt.TopRightCorner)
        layout.addWidget(self.tabs)
        self.setCentralWidget(central)
    def start_proton_loading(self):
        self.protons_table.setRowCount(0) # Clear table while loading
        self.proton_thread = LoadProtonsThread(self.proton_manager)
        self.proton_thread.protons_loaded.connect(self.load_protons)
        self.proton_thread.start()
    def update_settings(self):
        sender = self.sender()
        obj_name = sender.objectName()
        if obj_name == 'theme_combo':
            self.settings['theme'] = sender.currentText()
        elif obj_name == 'runner_combo':
            self.settings['default_runner'] = sender.currentText()
        elif obj_name == 'update_combo':
            self.settings['auto_update'] = sender.currentText()
        elif obj_name == 'esync_check':
            self.settings['enable_esync'] = sender.isChecked()
        elif obj_name == 'fsync_check':
            self.settings['enable_fsync'] = sender.isChecked()
        elif obj_name == 'dxvk_async_check':
            self.settings['enable_dxvk_async'] = sender.isChecked()
        self.config_manager.save_settings(self.settings)
    def load_games(self):
        self.games_list.setRowCount(0)
        self.games = self.config_manager.load_games()
        self.games_list.setRowCount(len(self.games))
        for i, game in enumerate(self.games):
            self.games_list.setItem(i, 0, QTableWidgetItem(game['name']))
            self.games_list.setItem(i, 1, QTableWidgetItem(game['runner']))
            self.games_list.setItem(i, 2, QTableWidgetItem(game.get('launch_options', '')))
        self.games_list.resizeColumnsToContents()
    def load_protons(self, protons):
        if self.stable_check.isChecked():
            protons = [p for p in protons if p['type'] != 'Experimental']
        self.protons_table.setRowCount(0)
        self.protons_table.setRowCount(len(protons))
        for i, proton in enumerate(protons):
            self.protons_table.setItem(i, 0, QTableWidgetItem(proton['version']))
            self.protons_table.setItem(i, 1, QTableWidgetItem(proton['type']))
            self.protons_table.setItem(i, 2, QTableWidgetItem(proton['date']))
            self.protons_table.setItem(i, 3, QTableWidgetItem(proton['status']))
        self.protons_table.resizeColumnsToContents()
    def show_add_game(self):
        if not hasattr(self, 'add_game_page'):
            self.add_game_page = QWidget()
            dlg_layout = QGridLayout(self.add_game_page)
            row = 0
            name_label = QLabel('Game Name:')
            self.add_name_edit = QLineEdit()
            self.add_name_edit.setPlaceholderText("Enter game name")
            dlg_layout.addWidget(name_label, row, 0)
            dlg_layout.addWidget(self.add_name_edit, row, 1, 1, 2)
            row += 1
            exe_label = QLabel('Executable / App ID:')
            self.add_exe_edit = QLineEdit()
            self.add_exe_edit.setPlaceholderText("Select game executable or enter Steam App ID")
            self.add_browse_btn = QPushButton(QIcon.fromTheme("folder"), 'Browse')
            self.add_browse_btn.clicked.connect(lambda: self.add_exe_edit.setText(QFileDialog.getOpenFileName(self, 'Select Executable', '/', 'Executables (*.exe *.bat);;All Files (*)')[0]))
            dlg_layout.addWidget(exe_label, row, 0)
            dlg_layout.addWidget(self.add_exe_edit, row, 1)
            dlg_layout.addWidget(self.add_browse_btn, row, 2)
            row += 1
            runner_label = QLabel('Runner:')
            self.add_runner_combo = QComboBox()
            self.add_runner_combo.addItems(['Native', 'Wine', 'Proton', 'Flatpak', 'Steam'])
            self.add_runner_combo.setCurrentText(self.settings['default_runner'])
            dlg_layout.addWidget(runner_label, row, 0)
            dlg_layout.addWidget(self.add_runner_combo, row, 1, 1, 2)
            row += 1
            proton_label = QLabel('Proton Version:')
            self.add_proton_combo = QComboBox()
            self.add_proton_combo.addItems([p['version'] for p in self.proton_manager.get_installed_protons()])
            self.add_proton_widget = QWidget()
            proton_layout = QHBoxLayout()
            proton_layout.addWidget(proton_label)
            proton_layout.addWidget(self.add_proton_combo)
            self.add_proton_widget.setLayout(proton_layout)
            self.add_proton_widget.setVisible(self.add_runner_combo.currentText() == 'Proton')
            dlg_layout.addWidget(self.add_proton_widget, row, 0, 1, 3)
            row += 1
            prefix_label = QLabel('Wine/Proton Prefix:')
            self.add_prefix_edit = QLineEdit()
            self.add_prefix_edit.setPlaceholderText("Select or enter prefix path")
            prefix_browse_btn = QPushButton(QIcon.fromTheme("folder"), 'Browse')
            prefix_browse_btn.clicked.connect(lambda: self.add_prefix_edit.setText(QFileDialog.getExistingDirectory(self, 'Select Prefix Directory')))
            self.add_prefix_widget = QWidget()
            prefix_layout = QHBoxLayout()
            prefix_layout.addWidget(prefix_label)
            prefix_layout.addWidget(self.add_prefix_edit)
            prefix_layout.addWidget(prefix_browse_btn)
            self.add_prefix_widget.setLayout(prefix_layout)
            self.add_prefix_widget.setVisible(self.add_runner_combo.currentText() in ['Wine', 'Proton'])
            dlg_layout.addWidget(self.add_prefix_widget, row, 0, 1, 3)
            row += 1
            launch_label = QLabel('Launch Options:')
            self.add_launch_edit = QLineEdit()
            self.add_launch_edit.setPlaceholderText("e.g., --fullscreen --bigpicture --gamescope --adaptive-sync --width=1920 --height=1080")
            dlg_layout.addWidget(launch_label, row, 0)
            dlg_layout.addWidget(self.add_launch_edit, row, 1, 1, 2)
            row += 1
            fps_label = QLabel('FPS Limit (for Gamescope):')
            self.add_fps_edit = QLineEdit()
            self.add_fps_edit.setPlaceholderText("e.g., 60 (only if --gamescope in launch options)")
            dlg_layout.addWidget(fps_label, row, 0)
            dlg_layout.addWidget(self.add_fps_edit, row, 1, 1, 2)
            row += 1
            self.add_dxvk_check = QCheckBox("Enable DXVK/VKD3D")
            self.add_dxvk_check.setVisible(self.add_runner_combo.currentText() in ['Wine', 'Proton'])
            dlg_layout.addWidget(self.add_dxvk_check, row, 0)
            row += 1
            self.add_esync_check = QCheckBox("Enable Esync (Override)")
            self.add_esync_check.setVisible(self.add_runner_combo.currentText() in ['Wine', 'Proton'])
            dlg_layout.addWidget(self.add_esync_check, row, 0)
            row += 1
            self.add_fsync_check = QCheckBox("Enable Fsync (Override)")
            self.add_fsync_check.setVisible(self.add_runner_combo.currentText() in ['Wine', 'Proton'])
            dlg_layout.addWidget(self.add_fsync_check, row, 0)
            row += 1
            self.add_dxvk_async_check = QCheckBox("Enable DXVK Async (Override)")
            self.add_dxvk_async_check.setVisible(self.add_runner_combo.currentText() in ['Wine', 'Proton'])
            dlg_layout.addWidget(self.add_dxvk_async_check, row, 0)
            row += 1
            self.add_app_id_widget = QWidget()
            app_id_layout = QHBoxLayout()
            app_id_label = QLabel('Steam App ID:')
            self.add_app_id_edit = QLineEdit()
            app_id_layout.addWidget(app_id_label)
            app_id_layout.addWidget(self.add_app_id_edit)
            self.add_app_id_widget.setLayout(app_id_layout)
            self.add_app_id_widget.setVisible(self.add_runner_combo.currentText() == 'Steam')
            dlg_layout.addWidget(self.add_app_id_widget, row, 0, 1, 3)
            row += 1
            def update_visibility(text):
                self.add_proton_widget.setVisible(text == 'Proton')
                self.add_prefix_widget.setVisible(text in ['Wine', 'Proton'])
                self.add_app_id_widget.setVisible(text == 'Steam')
                self.add_browse_btn.setVisible(text != 'Steam')
                self.add_dxvk_check.setVisible(text in ['Wine', 'Proton'])
                self.add_esync_check.setVisible(text in ['Wine', 'Proton'])
                self.add_fsync_check.setVisible(text in ['Wine', 'Proton'])
                self.add_dxvk_async_check.setVisible(text in ['Wine', 'Proton'])
            self.add_runner_combo.currentTextChanged.connect(update_visibility)
            button_layout = QHBoxLayout()
            ok_btn = QPushButton(QIcon.fromTheme("dialog-ok"), 'Add Game')
            cancel_btn = QPushButton(QIcon.fromTheme("dialog-cancel"), 'Cancel')
            ok_btn.clicked.connect(self.save_add_game)
            cancel_btn.clicked.connect(lambda: self.games_stack.setCurrentWidget(self.games_main_page))
            button_layout.addStretch()
            button_layout.addWidget(ok_btn)
            button_layout.addWidget(cancel_btn)
            dlg_layout.addLayout(button_layout, row, 0, 1, 3)
            self.games_stack.addWidget(self.add_game_page)
        self.games_stack.setCurrentWidget(self.add_game_page)
    def save_add_game(self):
        name = self.add_name_edit.text()
        exe = self.add_exe_edit.text()
        runner = self.add_runner_combo.currentText()
        launch_options = self.add_launch_edit.text()
        fps_limit = self.add_fps_edit.text()
        enable_dxvk = self.add_dxvk_check.isChecked()
        enable_esync = self.add_esync_check.isChecked()
        enable_fsync = self.add_fsync_check.isChecked()
        enable_dxvk_async = self.add_dxvk_async_check.isChecked()
        app_id = self.add_app_id_edit.text() if runner == 'Steam' else ''
        prefix = self.add_prefix_edit.text() if runner in ['Wine', 'Proton'] else ''
        if runner == 'Proton':
            runner = self.add_proton_combo.currentText()
        if not name or (runner != 'Steam' and not exe) or (runner == 'Steam' and not app_id):
            QMessageBox.warning(self, 'Error', 'Name and Executable/App ID required')
            return
        if runner in ['Wine', 'Proton'] and not prefix:
            prefix = os.path.join(self.config_manager.prefixes_dir, name.replace(' ', '_'))
        if prefix:
            os.makedirs(prefix, exist_ok=True)
        if os.name == 'posix' and ':' in exe:
            exe = exe.replace('\\', '/').replace('C:', '/drive_c')
        game = {
            'name': name,
            'exe': exe,
            'runner': runner,
            'prefix': prefix,
            'launch_options': launch_options,
            'fps_limit': int(fps_limit) if fps_limit.isdigit() else '',
            'enable_dxvk': enable_dxvk,
            'enable_esync': enable_esync,
            'enable_fsync': enable_fsync,
            'enable_dxvk_async': enable_dxvk_async,
            'app_id': app_id
        }
        self.game_manager.add_game(game)
        self.load_games()
        self.games_stack.setCurrentWidget(self.games_main_page)
        QMessageBox.information(self, 'Success', 'Game added successfully!')
    def launch_game(self):
        selected = self.games_list.currentRow()
        if selected < 0:
            QMessageBox.warning(self, 'Error', 'No game selected')
            return
        name = self.games_list.item(selected, 0).text()
        game = next((g for g in self.games if g['name'] == name), None)
        if game:
            try:
                gamescope_flag = '--gamescope' in game.get('launch_options', '')
                self.game_manager.launch_game(game, gamescope_flag)
            except Exception as e:
                logging.error(f"Error launching {name}: {e}")
                print(f"Error launching {name}: {e}")
                QMessageBox.warning(self, 'Error', str(e))
    def remove_game(self):
        selected = self.games_list.currentRow()
        if selected < 0:
            QMessageBox.warning(self, 'Error', 'No game selected')
            return
        name = self.games_list.item(selected, 0).text()
        self.game_manager.remove_game(name)
        self.load_games()
    def show_configure_game(self):
        selected = self.games_list.currentRow()
        if selected < 0:
            QMessageBox.warning(self, 'Error', 'No game selected')
            return
        name = self.games_list.item(selected, 0).text()
        self.configure_game_data = next((g for g in self.games if g['name'] == name), None)
        if not self.configure_game_data:
            return
        if not hasattr(self, 'configure_game_page'):
            self.configure_game_page = QWidget()
            dlg_layout = QGridLayout(self.configure_game_page)
            row = 0
            name_label = QLabel('Game Name:')
            self.configure_name_edit = QLineEdit()
            dlg_layout.addWidget(name_label, row, 0)
            dlg_layout.addWidget(self.configure_name_edit, row, 1, 1, 2)
            row += 1
            runner_label = QLabel('Runner:')
            self.configure_runner_combo = QComboBox()
            self.configure_runner_combo.addItems(['Native', 'Wine', 'Proton', 'Flatpak', 'Steam'])
            dlg_layout.addWidget(runner_label, row, 0)
            dlg_layout.addWidget(self.configure_runner_combo, row, 1, 1, 2)
            row += 1
            proton_label = QLabel('Proton Version:')
            self.configure_proton_combo = QComboBox()
            self.configure_proton_combo.addItems([p['version'] for p in self.proton_manager.get_installed_protons()])
            self.configure_proton_widget = QWidget()
            proton_layout = QHBoxLayout()
            proton_layout.addWidget(proton_label)
            proton_layout.addWidget(self.configure_proton_combo)
            self.configure_proton_widget.setLayout(proton_layout)
            dlg_layout.addWidget(self.configure_proton_widget, row, 0, 1, 3)
            row += 1
            fps_label = QLabel('FPS Limit (for Gamescope):')
            self.configure_fps_edit = QLineEdit()
            dlg_layout.addWidget(fps_label, row, 0)
            dlg_layout.addWidget(self.configure_fps_edit, row, 1, 1, 2)
            row += 1
            def update_visibility(text):
                self.configure_proton_widget.setVisible(text == 'Proton')
            self.configure_runner_combo.currentTextChanged.connect(update_visibility)
            button_layout = QHBoxLayout()
            ok_btn = QPushButton(QIcon.fromTheme("dialog-ok"), 'Save Changes')
            cancel_btn = QPushButton(QIcon.fromTheme("dialog-cancel"), 'Cancel')
            ok_btn.clicked.connect(self.save_configure_game)
            cancel_btn.clicked.connect(lambda: self.games_stack.setCurrentWidget(self.games_main_page))
            button_layout.addStretch()
            button_layout.addWidget(ok_btn)
            button_layout.addWidget(cancel_btn)
            dlg_layout.addLayout(button_layout, row, 0, 1, 3)
            self.games_stack.addWidget(self.configure_game_page)
        # Populate fields
        game = self.configure_game_data
        self.configure_name_edit.setText(game['name'])
        current_runner = game['runner']
        if 'Proton' in current_runner:
            self.configure_runner_combo.setCurrentText('Proton')
            self.configure_proton_combo.setCurrentText(current_runner)
        else:
            self.configure_runner_combo.setCurrentText(current_runner)
        self.configure_proton_widget.setVisible(self.configure_runner_combo.currentText() == 'Proton')
        self.configure_fps_edit.setText(str(game.get('fps_limit', '')))
        self.games_stack.setCurrentWidget(self.configure_game_page)
    def save_configure_game(self):
        new_name = self.configure_name_edit.text()
        new_runner = self.configure_runner_combo.currentText()
        new_fps = self.configure_fps_edit.text()
        if new_runner == 'Proton':
            new_runner = self.configure_proton_combo.currentText()
        if not new_name:
            QMessageBox.warning(self, 'Error', 'Name required')
            return
        game = self.configure_game_data
        game['name'] = new_name
        game['runner'] = new_runner
        game['fps_limit'] = int(new_fps) if new_fps.isdigit() else ''
        self.config_manager.save_games(self.games)
        self.load_games()
        self.games_stack.setCurrentWidget(self.games_main_page)
        QMessageBox.information(self, 'Success', 'Game configuration updated!')
    def show_install_proton(self):
        if not hasattr(self, 'install_proton_page'):
            self.install_proton_page = QWidget()
            layout = QVBoxLayout(self.install_proton_page)
            type_label = QLabel("Proton Type:")
            self.install_type_combo = QComboBox()
            self.install_type_combo.addItems(['GE', 'Official', 'Experimental', 'Custom'])
            layout.addWidget(type_label)
            layout.addWidget(self.install_type_combo)
            self.install_version_widget = QWidget()
            version_layout = QVBoxLayout()
            version_label = QLabel("Select Version:")
            self.install_version_combo = QComboBox()
            version_layout.addWidget(version_label)
            version_layout.addWidget(self.install_version_combo)
            self.install_version_widget.setLayout(version_layout)
            self.install_version_widget.setVisible(False)
            layout.addWidget(self.install_version_widget)
            self.install_custom_widget = QWidget()
            custom_layout = QVBoxLayout()
            custom_type_label = QLabel("Custom Source:")
            self.install_custom_type_combo = QComboBox()
            self.install_custom_type_combo.addItems(['Tar.gz File', 'Folder'])
            custom_layout.addWidget(custom_type_label)
            custom_layout.addWidget(self.install_custom_type_combo)
            path_label = QLabel("Path:")
            self.install_path_edit = QLineEdit()
            self.install_browse_btn = QPushButton(QIcon.fromTheme("folder"), 'Browse')
            path_hbox = QHBoxLayout()
            path_hbox.addWidget(self.install_path_edit)
            path_hbox.addWidget(self.install_browse_btn)
            custom_layout.addWidget(path_label)
            custom_layout.addLayout(path_hbox)
            name_label = QLabel("Version Name:")
            self.install_name_edit = QLineEdit()
            custom_layout.addWidget(name_label)
            custom_layout.addWidget(self.install_name_edit)
            self.install_custom_widget.setLayout(custom_layout)
            self.install_custom_widget.setVisible(False)
            layout.addWidget(self.install_custom_widget)
            def update_ui(text):
                if text == 'Custom':
                    self.install_version_widget.setVisible(False)
                    self.install_custom_widget.setVisible(True)
                else:
                    self.install_version_widget.setVisible(True)
                    self.install_custom_widget.setVisible(False)
                    if text == 'GE':
                        available = self.proton_manager.get_available_ge()
                    elif text == 'Official':
                        available = self.proton_manager.get_available_official(stable=True)
                    elif text == 'Experimental':
                        available = self.proton_manager.get_available_official(stable=False)
                    self.install_version_combo.clear()
                    self.install_version_combo.addItems(available or ["No versions available"])
            self.install_type_combo.currentTextChanged.connect(update_ui)
            update_ui(self.install_type_combo.currentText())
            def browse_custom():
                if self.install_custom_type_combo.currentText() == 'Tar.gz File':
                    path = QFileDialog.getOpenFileName(self, 'Select Tar.gz', '', 'Tar.gz (*.tar.gz)')[0]
                else:
                    path = QFileDialog.getExistingDirectory(self, 'Select Folder')
                if path:
                    self.install_path_edit.setText(path)
                    if not self.install_name_edit.text():
                        self.install_name_edit.setText(os.path.basename(path).replace('.tar.gz', ''))
            self.install_browse_btn.clicked.connect(browse_custom)
            button_layout = QHBoxLayout()
            ok_btn = QPushButton(QIcon.fromTheme("dialog-ok"), 'Install')
            cancel_btn = QPushButton(QIcon.fromTheme("dialog-cancel"), 'Cancel')
            ok_btn.clicked.connect(self.start_install_proton)
            cancel_btn.clicked.connect(lambda: self.protons_stack.setCurrentWidget(self.protons_main_page))
            button_layout.addStretch()
            button_layout.addWidget(ok_btn)
            button_layout.addWidget(cancel_btn)
            layout.addLayout(button_layout)
            self.protons_stack.addWidget(self.install_proton_page)
        self.protons_stack.setCurrentWidget(self.install_proton_page)
    def start_install_proton(self):
        proton_type = self.install_type_combo.currentText()
        version = self.install_version_combo.currentText() if proton_type != 'Custom' else self.install_name_edit.text()
        custom_path = self.install_path_edit.text() if proton_type == 'Custom' else None
        custom_type = self.install_custom_type_combo.currentText() if proton_type == 'Custom' else None
        if proton_type == 'Custom' and (not version or not custom_path):
            QMessageBox.warning(self, 'Error', 'Name and Path required')
            return
        self.protons_stack.setCurrentWidget(self.protons_main_page)
        progress = QProgressDialog(f"Installing {proton_type} Proton...", "Cancel", 0, 100, self)
        progress.setWindowModality(Qt.WindowModal)
        progress.setAutoClose(True)
        self.install_thread = InstallThread(self.proton_manager, version, proton_type, custom_path, custom_type)
        self.install_thread.update_progress.connect(lambda stage, value, total: progress.setLabelText(stage) or progress.setValue(int(value * 100 / total) if total else 0))
        self.install_thread.finished.connect(lambda success, message: self.install_finished(success, message, version, progress))
        self.install_thread.start()
        progress.canceled.connect(self.install_thread.requestInterruption)
    def install_finished(self, success, message, version, progress):
        progress.setValue(100)
        if success:
            QMessageBox.information(self, 'Success', f'Proton {version} installed')
        else:
            QMessageBox.warning(self, 'Error', f'Failed to install Proton {version}: {message}')
        self.start_proton_loading()
    def update_proton(self):
        selected = self.protons_table.currentRow()
        if selected < 0:
            QMessageBox.warning(self, 'Error', 'No Proton selected')
            return
        version = self.protons_table.item(selected, 0).text()
        proton_type = self.protons_table.item(selected, 1).text()
        update_info = self.proton_manager.check_update(version, proton_type)
        if not update_info:
            QMessageBox.information(self, 'Info', 'No update available or not supported')
            return
        new_type, new_version = update_info
        reply = QMessageBox.question(self, 'Update', f'Update to {new_version} ({new_type})?', QMessageBox.Yes | QMessageBox.No)
        if reply == QMessageBox.No:
            return
        progress = QProgressDialog(f"Updating {proton_type} Proton to {new_version}...", "Cancel", 0, 100, self)
        progress.setWindowModality(Qt.WindowModal)
        progress.setAutoClose(True)
        self.update_thread = InstallThread(self.proton_manager, new_version, new_type)
        self.update_thread.update_progress.connect(lambda stage, value, total: progress.setLabelText(stage) or progress.setValue(int(value * 100 / total) if total else 0))
        self.update_thread.finished.connect(lambda success, message: self.update_finished(success, message, new_version, version, progress))
        self.update_thread.start()
        progress.canceled.connect(self.update_thread.requestInterruption)
    def update_finished(self, success, message, new_version, old_version, progress):
        progress.setValue(100)
        if success:
            self.proton_manager.remove_proton(old_version)
            QMessageBox.information(self, 'Success', f'Updated to {new_version}')
        else:
            QMessageBox.warning(self, 'Error', f'Failed to update to {new_version}: {message}')
        self.start_proton_loading()
    def remove_proton(self):
        selected = self.protons_table.currentRow()
        if selected < 0:
            QMessageBox.warning(self, 'Error', 'No Proton selected')
            return
        version = self.protons_table.item(selected, 0).text()
        reply = QMessageBox.question(self, 'Confirm', f'Remove {version}?', QMessageBox.Yes | QMessageBox.No)
        if reply == QMessageBox.Yes:
            if self.proton_manager.remove_proton(version):
                QMessageBox.information(self, 'Success', f'{version} removed')
            else:
                QMessageBox.warning(self, 'Error', f'Failed to remove {version}')
            self.start_proton_loading()
