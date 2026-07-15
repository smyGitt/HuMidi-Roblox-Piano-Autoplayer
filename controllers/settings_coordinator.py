import os
import subprocess
import webbrowser

from PySide6.QtCore import QObject, Qt
from PySide6.QtWidgets import QMessageBox, QFileDialog

from managers.UpdateManager import UpdateChecker, REQUEST_TIMEOUT_SECONDS
from ui.theme import ThemeManager


def _auto_check_enabled(loaded_cfg: dict) -> bool:
    """Whether the automatic launch-time update check runs for this config.

    Defaults to True when the key is absent (config predating this setting) or
    the config is empty (no saved config yet).
    """
    return loaded_cfg.get('auto_check_updates', True)


class SettingsCoordinator(QObject):
    """Bridges SettingsTab/ConfigManager/HotkeyManager for directories, hotkeys,
    window chrome toggles, config persistence, and the update-check flow.

    Must be a QObject (not a plain Python object): `UpdateChecker` (a QThread
    subclass) emits its signals from inside its own overridden run(), i.e. on
    the checker's thread. Qt can only auto-queue that connection back to the
    GUI thread when it can read the receiver's thread affinity off a QObject;
    a plain-object receiver has no thread affinity, so Qt falls back to a
    direct connection and runs the slot (which builds a QMessageBox) on the
    update-check thread. See the identical fix and explanation on
    LoadCoordinator.
    """

    def __init__(self, window, ui, config_manager, hotkey_manager, playback_coordinator, app_version):
        super().__init__()
        self.window = window
        self.ui = ui
        self.config_manager = config_manager
        self.hotkey_manager = hotkey_manager
        self.playback_coordinator = playback_coordinator
        self.app_version = app_version
        # Must exist before bind_signals(): load_config_to_ui fires settings
        # toggle signals that call save_config, which reads this attribute.
        self._skipped_update_version = ''
        self._update_checker = None
        self._manual_checker = None

    def bind_signals(self) -> None:
        self.ui.settings_tab.reset_all_btn.clicked.connect(self.ui.reset_controls_to_default)
        self.ui.settings_tab.save_browse_btn.clicked.connect(self._browse_save_dir)
        self.ui.settings_tab.save_edit_btn.clicked.connect(self._open_save_dir)
        self.ui.settings_tab.midi_browse_btn.clicked.connect(self._browse_midi_dir)
        self.ui.settings_tab.midi_edit_btn.clicked.connect(self._open_midi_dir)
        self.ui.settings_tab.themes_browse_btn.clicked.connect(self._browse_themes_dir)
        self.ui.settings_tab.themes_edit_btn.clicked.connect(self._open_themes_dir)
        self.ui.settings_tab.hk_btn.clicked.connect(self._change_hotkey)
        self.ui.settings_tab.save_hk_btn.clicked.connect(self._change_save_hotkey)
        self.ui.settings_tab.check_update_btn.clicked.connect(self._manual_check_update)

        self.ui.settings_tab.always_top_check.toggled.connect(self._toggle_always_on_top)
        self.ui.settings_tab.opacity_slider.valueChanged.connect(self._change_opacity)

        # Settings-tab persistence: save immediately on change so closing without playing doesn't lose them
        self.ui.settings_tab.always_top_check.toggled.connect(self.save_config)
        self.ui.settings_tab.opacity_slider.valueChanged.connect(self.save_config)
        self.ui.settings_tab.timeline_vis_check.toggled.connect(self.save_config)
        self.ui.settings_tab.piano_vis_check.toggled.connect(self.save_config)
        self.ui.settings_tab.pedal_prompt_threshold_spinbox.valueChanged.connect(self.save_config)
        self.ui.settings_tab.redact_paths_check.toggled.connect(self.save_config)

        # Privacy: mirror the redact-paths toggle onto the DebugTab immediately
        self.ui.settings_tab.redact_paths_check.toggled.connect(self.ui.debug_tab.set_redact_paths)

        self.hotkey_manager.bound_updated.connect(self._on_hotkey_bound)
        self.hotkey_manager.bound_save_updated.connect(self._on_save_hotkey_bound)

    def start_auto_update_check(self, loaded_cfg: dict) -> None:
        self._skipped_update_version = loaded_cfg.get('skipped_update_version', '')
        if _auto_check_enabled(loaded_cfg):
            self._update_checker = UpdateChecker(self.app_version)
            self._update_checker.update_available.connect(
                lambda tag, url: self._on_update_available(tag, url, suppress_skipped=True)
            )
            self._update_checker.start()

    def join_update_threads(self) -> None:
        # Join every background update-check thread (bounded) so none outlives
        # the window and later emits a signal into a destroyed MainWindow.
        # UpdateChecker overrides run() with a single blocking network call and
        # never starts an event loop, so quit() would be a no-op for it; the
        # wait() bound must cover the request's own timeout (see
        # UpdateManager.REQUEST_TIMEOUT_SECONDS) to actually join rather than
        # merely give up after an arbitrary duration.
        update_join_ms = int(REQUEST_TIMEOUT_SECONDS * 1000) + 500
        if self._update_checker is not None:
            self._update_checker.wait(update_join_ms)
        if self._manual_checker is not None:
            self._manual_checker.wait(update_join_ms)

    def save_config(self) -> None:
        config_data = self.ui.gather_app_config()
        config_data['skipped_update_version'] = self._skipped_update_version
        self.config_manager.save(config_data)

    def _toggle_always_on_top(self, checked) -> None:
        flags = self.window.windowFlags()
        if checked:
            self.window.setWindowFlags(flags | Qt.WindowType.WindowStaysOnTopHint)
        else:
            self.window.setWindowFlags(flags & ~Qt.WindowType.WindowStaysOnTopHint)
        self.window.show()

    def _change_opacity(self, value) -> None:
        self.window.setWindowOpacity(value / 100.0)

    def _browse_save_dir(self) -> None:
        path = QFileDialog.getExistingDirectory(self.window, "Select Save Directory", self.config_manager.save_dir)
        if path:
            self.config_manager.set_save_dir(path)
            self.ui.settings_tab.save_path_input.setText(path)
            self.save_config()

    def _open_save_dir(self) -> None:
        path = self.config_manager.save_dir
        if os.path.isdir(path):
            subprocess.Popen(["explorer", os.path.normpath(path)])

    def _browse_midi_dir(self) -> None:
        path = QFileDialog.getExistingDirectory(self.window, "Select MIDI Directory", self.config_manager.midi_dir)
        if path:
            self.config_manager.set_midi_dir(path)
            self.ui.settings_tab.midi_path_input.setText(path)
            self.save_config()

    def _open_midi_dir(self) -> None:
        path = self.config_manager.midi_dir
        if os.path.isdir(path):
            subprocess.Popen(["explorer", os.path.normpath(path)])

    def _browse_themes_dir(self) -> None:
        current = str(ThemeManager._themes_dir)
        path = QFileDialog.getExistingDirectory(self.window, "Select Themes Directory", current)
        if path:
            ThemeManager.set_themes_dir(path)
            self.ui.settings_tab.themes_path_input.setText(str(ThemeManager._themes_file))

    def _open_themes_dir(self) -> None:
        themes_dir = ThemeManager._themes_dir
        themes_dir.mkdir(parents=True, exist_ok=True)
        subprocess.Popen(["explorer", os.path.normpath(str(themes_dir))])

    def _change_hotkey(self) -> None:
        QMessageBox.information(self.window, "Bind Key", "Press the key you want to bind now.")
        self.ui.settings_tab.hk_btn.setText("Listening...")
        self.ui.settings_tab.hk_btn.setEnabled(False)
        self.hotkey_manager.start_binding()

    def _on_hotkey_bound(self, key_str) -> None:
        self.ui.settings_tab.hk_label.setText(f"Hotkey: {key_str}")
        self.ui.settings_tab.hk_btn.setText("Change")
        self.ui.settings_tab.hk_btn.setEnabled(True)
        self.playback_coordinator.sync_play_button()

    def _change_save_hotkey(self) -> None:
        QMessageBox.information(self.window, "Bind Key", "Press the key you want to bind now.")
        self.ui.settings_tab.save_hk_btn.setText("Listening...")
        self.ui.settings_tab.save_hk_btn.setEnabled(False)
        self.hotkey_manager.start_save_binding()

    def _on_save_hotkey_bound(self, key_str) -> None:
        self.ui.settings_tab.save_hk_label.setText(f"Hotkey: {key_str}")
        self.ui.settings_tab.save_hk_btn.setText("Change")
        self.ui.settings_tab.save_hk_btn.setEnabled(True)

    def _manual_check_update(self) -> None:
        btn = self.ui.settings_tab.check_update_btn
        btn.setEnabled(False)
        btn.setText("Checking...")
        self._manual_checker = UpdateChecker(self.app_version, force=True)
        self._manual_checker.update_available.connect(self._on_update_available)
        self._manual_checker.update_available.connect(lambda *_: self._reset_update_btn())
        self._manual_checker.no_update.connect(self._on_no_update)
        self._manual_checker.check_failed.connect(self._on_check_failed)
        self._manual_checker.start()

    def _reset_update_btn(self) -> None:
        btn = self.ui.settings_tab.check_update_btn
        btn.setEnabled(True)
        btn.setText("Check for updates")

    def _on_no_update(self) -> None:
        self._reset_update_btn()
        QMessageBox.information(self.window, "Up to Date",
            f"HuMidi v{self.app_version} is the latest version.")

    def _on_check_failed(self) -> None:
        self._reset_update_btn()
        QMessageBox.warning(self.window, "Update Check Failed",
            "Could not reach GitHub.\nPlease check your internet connection.")

    def _on_update_available(self, latest_tag: str, releases_url: str, suppress_skipped: bool = False) -> None:
        # The automatic launch check passes suppress_skipped=True so a version the
        # user chose to skip does not re-prompt on every start. The manual "Check
        # for updates" button passes False, so an explicit check always shows the
        # dialog even for a previously skipped version.
        if suppress_skipped and latest_tag == self._skipped_update_version:
            return

        msg = QMessageBox(self.window)
        msg.setIcon(QMessageBox.Icon.Question)
        msg.setWindowTitle("Update Available")
        msg.setText(f"Update available to {latest_tag}. Would you like to open the download page?")
        yes_btn = msg.addButton(QMessageBox.StandardButton.Yes)
        msg.addButton(QMessageBox.StandardButton.No)
        skip_btn = msg.addButton("Skip this version", QMessageBox.ButtonRole.DestructiveRole)
        msg.setDefaultButton(yes_btn)
        msg.exec()

        clicked = msg.clickedButton()
        if clicked is yes_btn:
            webbrowser.open(releases_url)
        elif clicked is skip_btn:
            self._skipped_update_version = latest_tag
            self.save_config()
