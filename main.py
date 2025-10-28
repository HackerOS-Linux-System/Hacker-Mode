import sys
import os
import shutil
from PySide6.QtWidgets import QApplication, QMessageBox
from main_window import MainWindow
import logging

os.environ["QT_QPA_PLATFORM"] = "xcb"  # Force X11 backend

# Define the stylesheet as a string
STYLESHEET = """
/* Hacker Launcher Styles - Black, White, Purple, Blue - Console-like with larger fonts */
QWidget {
    background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #1a1a1a, stop:1 #2a2a2a);
    color: #ffffff;
    font-family: "Courier New", monospace;
    font-size: 16px;
}
QMainWindow, QDialog, QProgressDialog {
    background: #1a1a1a;
}
QPushButton {
    background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #4b0082, stop:1 #6a0dad);
    color: #ffffff;
    border: 2px solid #00b7eb;
    border-radius: 12px;
    padding: 15px 20px;
    font-weight: bold;
    font-size: 18px;
}
QPushButton:hover {
    background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #6a0dad, stop:1 #8b0ece);
    border-color: #00d4ff;
}
QPushButton:pressed {
    background: #3a0062;
}
QTableWidget {
    background: #2a2a2a;
    border: 2px solid #00b7eb;
    border-radius: 12px;
    color: #ffffff;
    gridline-color: #00b7eb;
    font-size: 18px;
}
QTableWidget::item {
    padding: 12px;
}
QTableWidget::item:selected {
    background: #4b0082;
    color: #ffffff;
}
QHeaderView::section {
    background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #4b0082, stop:1 #6a0dad);
    color: #ffffff;
    padding: 12px;
    border: 1px solid #00b7eb;
    font-size: 18px;
}
QComboBox {
    background: #2a2a2a;
    border: 2px solid #00b7eb;
    color: #ffffff;
    padding: 12px;
    border-radius: 12px;
    font-size: 18px;
}
QComboBox::drop-down {
    border: none;
}
QComboBox QAbstractItemView {
    background: #2a2a2a;
    color: #ffffff;
    selection-background-color: #4b0082;
    border: 2px solid #00b7eb;
    font-size: 18px;
}
QLineEdit {
    background: #2a2a2a;
    border: 2px solid #00b7eb;
    color: #ffffff;
    padding: 12px;
    border-radius: 12px;
    font-size: 18px;
}
QLineEdit:focus {
    border-color: #00d4ff;
}
QLabel {
    color: #ffffff;
    font-weight: bold;
    font-size: 18px;
}
QTabWidget::pane {
    border: 2px solid #00b7eb;
    background: #2a2a2a;
    border-radius: 12px;
}
QTabBar::tab {
    background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #4b0082, stop:1 #6a0dad);
    color: #ffffff;
    padding: 18px 32px;
    margin: 6px;
    border-radius: 12px;
    border: 1px solid #00b7eb;
    font-size: 20px;
}
QTabBar::tab:selected {
    background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #00b7eb, stop:1 #00d4ff);
    color: #1a1a1a;
    font-weight: bold;
}
QTabBar::tab:hover {
    background: #8b0ece;
}
QToolTip {
    background: #2a2a2a;
    color: #ffffff;
    border: 1px solid #00b7eb;
    font-size: 16px;
}
QProgressDialog {
    background: #1a1a1a;
    color: #ffffff;
    font-size: 18px;
}
QProgressBar {
    background: #2a2a2a;
    border: 2px solid #00b7eb;
    border-radius: 12px;
    text-align: center;
    color: #ffffff;
    font-size: 18px;
}
QProgressBar::chunk {
    background: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #4b0082, stop:1 #6a0dad);
}
QCheckBox {
    font-size: 18px;
}
QTextEdit {
    font-size: 18px;
}
QMenu {
    background: #2a2a2a;
    color: #ffffff;
    border: 2px solid #00b7eb;
    font-size: 18px;
}
QMenu::item {
    padding: 12px 24px;
}
QMenu::item:selected {
    background: #4b0082;
    color: #ffffff;
}
"""

if __name__ == '__main__':
    try:
        gamescope = '--gamescope' in sys.argv
        if gamescope and not shutil.which('gamescope'):
            app = QApplication(sys.argv)
            QMessageBox.critical(None, 'Error', "Gamescope is not installed. Please install it via your package manager (e.g., apt, dnf, pacman).")
            sys.exit(1)
        app = QApplication(sys.argv)
        app.setStyleSheet(STYLESHEET)
        window = MainWindow()
        window.show()
        sys.exit(app.exec())
    except Exception as e:
        logging.error(f"Application error: {e}")
        QMessageBox.critical(None, 'Error', str(e))
        sys.exit(1)
