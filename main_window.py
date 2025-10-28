from PySide6.QtWidgets import QMainWindow, QWidget, QVBoxLayout, QHBoxLayout, QTableWidget, QTableWidgetItem, QPushButton, QTabWidget, QComboBox, QLineEdit, QLabel, QMessageBox, QFileDialog, QDialog, QGridLayout, QProgressDialog, QCheckBox, QTextEdit, QMenu
from PySide6.QtCore import Qt, QThread, Signal
from PySide6.QtGui import QIcon, QKeyEvent, QAction
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
            else:  # Custom
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
        self.setWindowTitle('Hacker Launcher')
        self.resize(1000, 800)
        self.config_manager = ConfigManager()
        self.proton_manager = ProtonManager()
        self.game_manager = GameManager(self.proton_manager)
        self.games = []
        self.settings = self.config_manager.settings
        self.settings['fullscreen'] = True  # Always fullscreen
        self.showFullScreen()
        self.setup_ui()
        self.load_games()
        self.start_proton_loading()
        # Set focus for keyboard navigation
        self.setFocusPolicy(Qt.StrongFocus)
        self.centralWidget().setFocus()

    def keyPressEvent(self, event: QKeyEvent):
        # Handle CTRL + SHIFT + M or SUPER + M for Hacker Menu
        if (event.modifiers() & Qt.ControlModifier and event.modifiers() & Qt.ShiftModifier and event.key() == Qt.Key_M) or \
           (event.modifiers() & Qt.MetaModifier and event.key() == Qt.Key_M):
            self.show_hacker_menu()
        super().keyPressEvent(event)

    def show_hacker_menu(self):
        # Show the menu at the button position
        pos = self.hacker_btn.mapToGlobal(self.hacker_btn.rect().bottomLeft())
        self.hacker_menu.exec_(pos)

    def setup_ui(self):
        central = QWidget()
        layout = QVBoxLayout(central)
        # Top bar with Hacker Menu button
        top_layout = QHBoxLayout()
        top_layout.addStretch()
        self.hacker_btn = QPushButton('Hacker Menu')
        self.hacker_btn.setToolTip("Open Hacker Menu (CTRL+SHIFT+M or SUPER+M)")
        self.hacker_btn.setFixedWidth(200)
        top_layout.addWidget(self.hacker_btn)
        layout.addLayout(top_layout)
        # Setup Hacker Menu
        self.hacker_menu = QMenu(self)
        switch_plasma = QAction('Switch to Plasma', self)
        switch_plasma.triggered.connect(self.switch_to_plasma)
        self.hacker_menu.addAction(switch_plasma)
        shutdown_act = QAction('Shutdown', self)
        shutdown_act.triggered.connect(self.shutdown_system)
        self.hacker_menu.addAction(shutdown_act)
        reboot_act = QAction('Reboot', self)
        reboot_act.triggered.connect(self.reboot_system)
        self.hacker_menu.addAction(reboot_act)
        logout_act = QAction('Log Out', self)
        logout_act.triggered.connect(self.log_out)
        self.hacker_menu.addAction(logout_act)
        self.hacker_btn.setMenu(self.hacker_menu)
        # Tabs
        tabs = QTabWidget()
        tabs.setFocusPolicy(Qt.StrongFocus)
        # Games tab
        games_widget = QWidget()
        games_layout = QVBoxLayout(games_widget)
        self.games_list = QTableWidget(0, 3)
        self.games_list.setHorizontalHeaderLabels(['Game Name', 'Runner', 'Launch Options'])
        self.games_list.horizontalHeader().setStretchLastSection(True)
        self.games_list.setSelectionBehavior(QTableWidget.SelectRows)
        self.games_list.setFocusPolicy(Qt.StrongFocus)
        games_layout.addWidget(QLabel("Installed Games"))
        games_layout.addWidget(self.games_list)
        buttons_layout = QHBoxLayout()
        add_btn = QPushButton(QIcon.fromTheme("list-add"), 'Add Game')
        add_btn.clicked.connect(self.add_game)
        add_btn.setToolTip("Add a new game to the launcher")
        launch_btn = QPushButton(QIcon.fromTheme("media-playback-start"), 'Launch Game')
        launch_btn.clicked.connect(self.launch_game)
        launch_btn.setToolTip("Launch the selected game")
        remove_btn = QPushButton(QIcon.fromTheme("edit-delete"), 'Remove Game')
        remove_btn.clicked.connect(self.remove_game)
        remove_btn.setToolTip("Remove the selected game")
        config_btn = QPushButton(QIcon.fromTheme("configure"), 'Configure Game')
        config_btn.clicked.connect(self.configure_game)
        config_btn.setToolTip("Configure Wine/Proton for the selected game")
        buttons_layout.addWidget(add_btn)
        buttons_layout.addWidget(launch_btn)
        buttons_layout.addWidget(remove_btn)
        buttons_layout.addWidget(config_btn)
        games_layout.addLayout(buttons_layout)
        tabs.addTab(games_widget, 'Games')
        # Protons tab
        protons_widget = QWidget()
        protons_layout = QVBoxLayout(protons_widget)
        self.protons_table = QTableWidget(0, 4)
        self.protons_table.setHorizontalHeaderLabels(['Version', 'Type', 'Installed Date', 'Status'])
        self.protons_table.horizontalHeader().setStretchLastSection(True)
        self.protons_table.setSelectionBehavior(QTableWidget.SelectRows)
        self.protons_table.setFocusPolicy(Qt.StrongFocus)
        protons_layout.addWidget(QLabel("Installed Protons"))
        protons_layout.addWidget(self.protons_table)
        buttons_layout = QHBoxLayout()
        install_btn = QPushButton(QIcon.fromTheme("list-add"), 'Install Proton')
        install_btn.clicked.connect(self.install_proton)
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
        stable_check = QCheckBox("Stable Only")
        stable_check.setChecked(True)
        stable_check.stateChanged.connect(self.start_proton_loading)
        buttons_layout.addWidget(install_btn)
        buttons_layout.addWidget(update_btn)
        buttons_layout.addWidget(remove_btn)
        buttons_layout.addWidget(refresh_btn)
        buttons_layout.addWidget(stable_check)
        protons_layout.addLayout(buttons_layout)
        tabs.addTab(protons_widget, 'Protons')
        # Settings tab (removed fullscreen option)
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
        tabs.addTab(settings_widget, 'Settings')
        layout.addWidget(tabs)
        self.setCentralWidget(central)

    def update_settings(self):
        sender = self.sender()
        if sender.objectName() == 'theme_combo':
            self.settings['theme'] = sender.currentText()
        elif sender.objectName() == 'runner_combo':
            self.settings['default_runner'] = sender.currentText()
        elif sender.objectName() == 'update_combo':
            self.settings['auto_update'] = sender.currentText()
        elif sender.objectName() == 'esync_check':
            self.settings['enable_esync'] = sender.isChecked()
        elif sender.objectName() == 'fsync_check':
            self.settings['enable_fsync'] = sender.isChecked()
        elif sender.objectName() == 'dxvk_async_check':
            self.settings['enable_dxvk_async'] = sender.isChecked()
        self.config_manager.save_settings(self.settings)

    def load_games(self):
        self.games = self.config_manager.load_games()
        self.games_list.setRowCount(len(self.games))
        for i, game in enumerate(self.games):
            self.games_list.setItem(i, 0, QTableWidgetItem(game['name']))
            self.games_list.setItem(i, 1, QTableWidgetItem(game['runner']))
            self.games_list.setItem(i, 2, QTableWidgetItem(game.get('launch_options', '')))

    def start_proton_loading(self):
        self.protons_table.clearContents()
        self.protons_table.setRowCount(0)
        thread = LoadProtonsThread(self.proton_manager)
        thread.protons_loaded.connect(self.load_protons)
        thread.start()

    def load_protons(self, protons):
        self.protons_table.setRowCount(len(protons))
        for i, p in enumerate(protons):
            self.protons_table.setItem(i, 0, QTableWidgetItem(p['version']))
            self.protons_table.setItem(i, 1, QTableWidgetItem(p['type']))
            self.protons_table.setItem(i, 2, QTableWidgetItem(p['date']))
            self.protons_table.setItem(i, 3, QTableWidgetItem(p['status']))

    def switch_to_plasma(self):
        try:
            subprocess.run(['startplasma-x11'], check=True)
            self.close()
        except Exception as e:
            QMessageBox.warning(self, 'Error', str(e))

    def shutdown_system(self):
        subprocess.run(['systemctl', 'poweroff'])

    def reboot_system(self):
        subprocess.run(['systemctl', 'reboot'])

    def log_out(self):
        subprocess.run(['loginctl', 'terminate-user', os.getlogin()])

    def add_game(self):
        add_dialog = QDialog(self)
        add_dialog.setWindowTitle("Add Game")
        layout = QGridLayout()
        name_label = QLabel("Game Name:")
        name_edit = QLineEdit()
        layout.addWidget(name_label, 0, 0)
        layout.addWidget(name_edit, 0, 1)
        runner_label = QLabel("Runner:")
        runner_combo = QComboBox()
        runner_combo.addItems(['Native', 'Wine', 'Proton', 'Flatpak', 'Steam'])
        runner_combo.setCurrentText(self.settings['default_runner'])
        layout.addWidget(runner_label, 1, 0)
        layout.addWidget(runner_combo, 1, 1)
        exe_label = QLabel("Executable/App ID:")
        exe_edit = QLineEdit()
        browse_btn = QPushButton(QIcon.fromTheme("folder"), 'Browse')
        exe_hbox = QHBoxLayout()
        exe_hbox.addWidget(exe_edit)
        exe_hbox.addWidget(browse_btn)
        layout.addWidget(exe_label, 2, 0)
        layout.addLayout(exe_hbox, 2, 1)
        prefix_label = QLabel("Prefix:")
        prefix_edit = QLineEdit()
        prefix_browse = QPushButton(QIcon.fromTheme("folder"), 'Browse')
        prefix_hbox = QHBoxLayout()
        prefix_hbox.addWidget(prefix_edit)
        prefix_hbox.addWidget(prefix_browse)
        layout.addWidget(prefix_label, 3, 0)
        layout.addLayout(prefix_hbox, 3, 1)
        options_label = QLabel("Launch Options:")
        options_edit = QLineEdit()
        layout.addWidget(options_label, 4, 0)
        layout.addWidget(options_edit, 4, 1)
        esync_check = QCheckBox("Enable Esync")
        esync_check.setChecked(self.settings['enable_esync'])
        layout.addWidget(esync_check, 5, 0)
        fsync_check = QCheckBox("Enable Fsync")
        fsync_check.setChecked(self.settings['enable_fsync'])
        layout.addWidget(fsync_check, 6, 0)
        dxvk_check = QCheckBox("Enable DXVK")
        layout.addWidget(dxvk_check, 7, 0)
        dxvk_async_check = QCheckBox("Enable DXVK Async")
        dxvk_async_check.setChecked(self.settings['enable_dxvk_async'])
        layout.addWidget(dxvk_async_check, 8, 0)
        def update_ui(text):
            is_steam = text == 'Steam'
            is_proton_wine = text in ['Proton', 'Wine']
            exe_label.setText("App ID:" if is_steam else "Executable:")
            prefix_label.setVisible(is_proton_wine)
            prefix_edit.setVisible(is_proton_wine)
            prefix_browse.setVisible(is_proton_wine)
            esync_check.setVisible(is_proton_wine)
            fsync_check.setVisible(is_proton_wine)
            dxvk_check.setVisible(is_proton_wine)
            dxvk_async_check.setVisible(is_proton_wine)
        runner_combo.currentTextChanged.connect(update_ui)
        update_ui(runner_combo.currentText())
        def browse_exe():
            if runner_combo.currentText() != 'Steam':
                path = QFileDialog.getOpenFileName(self, 'Select Executable', '', 'Executables (*.exe *.bin *.sh);;All Files (*)')[0]
                if path:
                    exe_edit.setText(path)
        browse_btn.clicked.connect(browse_exe)
        def browse_prefix():
            path = QFileDialog.getExistingDirectory(self, 'Select Prefix Folder')
            if path:
                prefix_edit.setText(path)
        prefix_browse.clicked.connect(browse_prefix)
        button_layout = QHBoxLayout()
        ok_btn = QPushButton(QIcon.fromTheme("dialog-ok"), 'Add')
        cancel_btn = QPushButton(QIcon.fromTheme("dialog-cancel"), 'Cancel')
        ok_btn.clicked.connect(add_dialog.accept)
        cancel_btn.clicked.connect(add_dialog.reject)
        button_layout.addStretch()
        button_layout.addWidget(ok_btn)
        button_layout.addWidget(cancel_btn)
        layout.addLayout(button_layout, 9, 0, 1, 2)
        add_dialog.setLayout(layout)
        add_dialog.resize(500, 400)
        if add_dialog.exec() == QDialog.Accepted:
            name = name_edit.text()
            runner = runner_combo.currentText()
            exe_or_id = exe_edit.text()
            prefix = prefix_edit.text() if runner in ['Wine', 'Proton'] else ''
            options = options_edit.text()
            enable_esync = esync_check.isChecked()
            enable_fsync = fsync_check.isChecked()
            enable_dxvk = dxvk_check.isChecked()
            enable_dxvk_async = dxvk_async_check.isChecked()
            if not name or not exe_or_id:
                QMessageBox.warning(self, 'Error', 'Name and Executable/App ID required')
                return
            game = {
                'name': name,
                'runner': runner,
                'prefix': prefix,
                'launch_options': options,
                'enable_esync': enable_esync,
                'enable_fsync': enable_fsync,
                'enable_dxvk': enable_dxvk,
                'enable_dxvk_async': enable_dxvk_async
            }
            if runner == 'Steam':
                game['app_id'] = exe_or_id
            else:
                game['exe'] = exe_or_id
            self.game_manager.add_game(game)
            self.load_games()
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
                gamescope = '--gamescope' in game.get('launch_options', '').split()
                self.game_manager.launch_game(game, gamescope)
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

    def configure_game(self):
        selected = self.games_list.currentRow()
        if selected < 0:
            QMessageBox.warning(self, 'Error', 'No game selected')
            return
        name = self.games_list.item(selected, 0).text()
        game = next((g for g in self.games if g['name'] == name), None)
        if not game or game['runner'] in ['Native', 'Flatpak', 'Steam']:
            QMessageBox.information(self, 'Info', 'No configuration needed for this runner')
            return
        if not shutil.which('winetricks'):
            QMessageBox.warning(self, 'Error', 'Winetricks not installed. Please install it.')
            return
        tricks_dialog = QDialog(self)
        tricks_dialog.setWindowTitle("Install Winetricks Libraries")
        tricks_layout = QVBoxLayout()
        tricks_label = QLabel("Select libraries to install:")
        tricks_list = QComboBox()
        popular_tricks = ['dotnet48', 'vcrun2019', 'dxvk', 'vkd3d', 'corefonts', 'd3dcompiler_47', 'physx', 'msls31']
        tricks_list.addItems(popular_tricks)
        install_btn = QPushButton('Install Selected')
        def install_trick():
            trick = tricks_list.currentText()
            prefix = game['prefix']
            env = os.environ.copy()
            env['WINEPREFIX'] = prefix
            try:
                subprocess.run(['winetricks', '--unattended', trick], env=env, check=True)
                QMessageBox.information(self, 'Success', f'{trick} installed')
            except Exception as e:
                QMessageBox.warning(self, 'Error', str(e))
        install_btn.clicked.connect(install_trick)
        tricks_layout.addWidget(tricks_label)
        tricks_layout.addWidget(tricks_list)
        tricks_layout.addWidget(install_btn)
        tricks_dialog.setLayout(tricks_layout)
        tricks_dialog.exec()
        env = os.environ.copy()
        env['WINEPREFIX'] = game['prefix']
        subprocess.Popen(['winetricks'], env=env)

    def install_proton(self):
        install_dialog = QDialog(self)
        install_dialog.setWindowTitle("Install Proton")
        layout = QVBoxLayout()
        type_label = QLabel("Proton Type:")
        type_combo = QComboBox()
        type_combo.addItems(['GE', 'Official', 'Experimental', 'Custom'])
        layout.addWidget(type_label)
        layout.addWidget(type_combo)
        version_widget = QWidget()
        version_layout = QVBoxLayout()
        version_label = QLabel("Select Version:")
        version_combo = QComboBox()
        version_layout.addWidget(version_label)
        version_layout.addWidget(version_combo)
        version_widget.setLayout(version_layout)
        version_widget.setVisible(False)
        layout.addWidget(version_widget)
        custom_widget = QWidget()
        custom_layout = QVBoxLayout()
        custom_type_label = QLabel("Custom Source:")
        custom_type_combo = QComboBox()
        custom_type_combo.addItems(['Tar.gz File', 'Folder'])
        custom_layout.addWidget(custom_type_label)
        custom_layout.addWidget(custom_type_combo)
        path_label = QLabel("Path:")
        path_edit = QLineEdit()
        browse_btn = QPushButton(QIcon.fromTheme("folder"), 'Browse')
        path_hbox = QHBoxLayout()
        path_hbox.addWidget(path_edit)
        path_hbox.addWidget(browse_btn)
        custom_layout.addWidget(path_label)
        custom_layout.addLayout(path_hbox)
        name_label = QLabel("Version Name:")
        name_edit = QLineEdit()
        custom_layout.addWidget(name_label)
        custom_layout.addWidget(name_edit)
        custom_widget.setLayout(custom_layout)
        custom_widget.setVisible(False)
        layout.addWidget(custom_widget)
        def update_ui(text):
            if text == 'Custom':
                version_widget.setVisible(False)
                custom_widget.setVisible(True)
            else:
                version_widget.setVisible(True)
                custom_widget.setVisible(False)
                if text == 'GE':
                    available = self.proton_manager.get_available_ge()
                elif text == 'Official':
                    available = self.proton_manager.get_available_official(stable=True)
                elif text == 'Experimental':
                    available = self.proton_manager.get_available_official(stable=False)
                version_combo.clear()
                version_combo.addItems(available or ["No versions available"])
        type_combo.currentTextChanged.connect(update_ui)
        update_ui(type_combo.currentText())
        def browse_custom():
            if custom_type_combo.currentText() == 'Tar.gz File':
                path = QFileDialog.getOpenFileName(self, 'Select Tar.gz', '', 'Tar.gz (*.tar.gz)')[0]
            else:
                path = QFileDialog.getExistingDirectory(self, 'Select Folder')
            if path:
                path_edit.setText(path)
                if not name_edit.text():
                    name_edit.setText(os.path.basename(path).replace('.tar.gz', ''))
        browse_btn.clicked.connect(browse_custom)
        button_layout = QHBoxLayout()
        ok_btn = QPushButton(QIcon.fromTheme("dialog-ok"), 'Install')
        cancel_btn = QPushButton(QIcon.fromTheme("dialog-cancel"), 'Cancel')
        ok_btn.clicked.connect(install_dialog.accept)
        cancel_btn.clicked.connect(install_dialog.reject)
        button_layout.addStretch()
        button_layout.addWidget(ok_btn)
        button_layout.addWidget(cancel_btn)
        layout.addLayout(button_layout)
        install_dialog.setLayout(layout)
        install_dialog.resize(500, 350)
        if install_dialog.exec() == QDialog.Accepted:
            proton_type = type_combo.currentText()
            progress = QProgressDialog(f"Installing {proton_type} Proton...", "Cancel", 0, 100, self)
            progress.setWindowModality(Qt.WindowModal)
            progress.setAutoClose(True)
            version = version_combo.currentText() if proton_type != 'Custom' else name_edit.text()
            custom_path = path_edit.text() if proton_type == 'Custom' else None
            custom_type = custom_type_combo.currentText() if proton_type == 'Custom' else None
            if proton_type == 'Custom' and (not version or not custom_path):
                QMessageBox.warning(self, 'Error', 'Name and Path required')
                return
            thread = InstallThread(self.proton_manager, version, proton_type, custom_path, custom_type)
            thread.update_progress.connect(lambda stage, value, total: progress.setLabelText(stage) or progress.setValue(int(value * 100 / total) if total else 0))
            thread.finished.connect(lambda success, message: self.install_finished(success, message, version, progress))
            thread.start()
            progress.canceled.connect(thread.terminate)

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
        thread = InstallThread(self.proton_manager, new_version, new_type)
        thread.update_progress.connect(lambda stage, value, total: progress.setLabelText(stage) or progress.setValue(int(value * 100 / total) if total else 0))
        thread.finished.connect(lambda success, message: self.update_finished(success, message, new_version, version, progress))
        thread.start()
        progress.canceled.connect(thread.terminate)

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
