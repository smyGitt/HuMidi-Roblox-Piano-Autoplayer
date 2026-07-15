#!/usr/bin/env python3
import sys
import os
import pynput  # noqa: F401  (must precede PySide6: shiboken's signature loader crashes on six.moves.queue if pynput imports it first)
from PySide6.QtWidgets import QApplication, QMainWindow
from PySide6.QtGui import QIcon

import core.session_cache as session_cache
from managers.HotkeyManager import HotkeyManager
from controllers.PlaybackController import PlaybackController
from controllers.app_state import AppState
from controllers.playback_ui_coordinator import PlaybackUICoordinator
from controllers.load_coordinator import LoadCoordinator
from controllers.settings_coordinator import SettingsCoordinator
from controllers.translator_coordinator import TranslatorCoordinator
from managers.ConfigManager import ConfigManager
from ui.MainWindowUI import MainWindowUI

APP_VERSION = "2.1"


class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle(f"HuMidi v{APP_VERSION}")
        # Set specific Icon base execution path (Required for OS Contexts)
        base_path = sys._MEIPASS if getattr(sys, 'frozen', False) else os.path.dirname(os.path.abspath(__file__))
        icon_path = os.path.join(base_path, 'icon.ico')
        if os.path.exists(icon_path):
            self.setWindowIcon(QIcon(icon_path))

        # Instantiate Domains
        self.config_manager = ConfigManager()
        self.ui = MainWindowUI(self)
        self.playback_controller = PlaybackController()
        self.hotkey_manager = HotkeyManager()

        # Global Application State
        self.state = AppState()

        # Coordinators: each owns one domain's UI-event-to-subsystem wiring.
        # settings/translator take playback_coordinator to reach sync_play_button.
        self.playback_coordinator = PlaybackUICoordinator(
            self, self.ui, self.playback_controller, self.hotkey_manager,
            self.config_manager, self.state,
        )
        self.load_coordinator = LoadCoordinator(
            self, self.ui, self.config_manager, self.playback_controller, self.state,
        )
        self.settings_coordinator = SettingsCoordinator(
            self, self.ui, self.config_manager, self.hotkey_manager,
            self.playback_coordinator, APP_VERSION,
        )
        self.translator_coordinator = TranslatorCoordinator(
            self, self.ui, self.playback_controller, self.state, self.playback_coordinator,
        )

        self._bind_signals()

        # Load initialization data
        loaded_cfg = self.config_manager.load()
        if loaded_cfg:
            self.ui.load_config_to_ui(loaded_cfg, self.config_manager.save_dir, self.config_manager.midi_dir)
        else:
            self.ui.reset_controls_to_default()
        # Sync explicitly: load_config_to_ui only fires the toggled signal when
        # the loaded value differs from the widget's built-in default.
        self.ui.debug_tab.set_redact_paths(self.ui.settings_tab.redact_paths_check.isChecked())
        self.ui.settings_tab.hk_label.setText(
            f"Hotkey: {self.hotkey_manager.format_hotkey_string()}"
        )
        self.ui.settings_tab.save_hk_label.setText(
            f"Hotkey: {self.hotkey_manager.format_save_hotkey_string()}"
        )

        self.ui.playback_tab.refresh_saved_songs(self.config_manager.save_dir)

        self.settings_coordinator.start_auto_update_check(loaded_cfg)

        self.resize(self.ui._expanded_size)

    def _bind_signals(self):
        self.playback_coordinator.bind_signals()
        self.load_coordinator.bind_signals()
        self.settings_coordinator.bind_signals()
        self.translator_coordinator.bind_signals()

    def closeEvent(self, event):
        # Join every background thread (bounded) so none outlives the window and
        # later emits a signal into a destroyed MainWindow.
        self.settings_coordinator.join_update_threads()
        self.load_coordinator.join_parse_thread()
        self.settings_coordinator.save_config()
        self.playback_controller.shutdown()
        session_cache.clear_cache()
        event.accept()


if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec())
