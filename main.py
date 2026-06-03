#!/usr/bin/env python3
import sys
import os
import json
import bisect
from datetime import datetime
from PyQt6.QtWidgets import QApplication, QMainWindow, QMessageBox, QFileDialog, QDialog
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QIcon

from core.core import MidiParser, KeyMapper, TempoMap
from core.translator import FormatRegistry
from managers.HotkeyManager import HotkeyManager
import webbrowser
from managers.UpdateManager import UpdateChecker
from controllers.PlaybackController import PlaybackController
from managers.ConfigManager import ConfigManager
from ui.MainWindowUI import MainWindowUI
from ui.TrackSelectionDialog import TrackSelectionDialog
from ui.LoadSaveDialog import LoadSaveDialog

APP_VERSION = "2.0"

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle(f"HuMidi v{APP_VERSION}")
        self.setMinimumWidth(960)
        self.setMinimumHeight(660)

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
        
        # Global Application States
        self.loaded_save_data = None
        self.loaded_save_filename = None
        self.selected_tracks_info = None
        self._parsed_tracks = None
        self._loaded_pedal_count = 0
        self.current_notes = []
        self._note_start_times = []
        self.total_song_duration_sec = 1.0
        self._max_note_duration = 0.0
        self.current_pedal_intervals = []

        self._bind_signals()

        # Load initialization data
        loaded_cfg = self.config_manager.load()
        if loaded_cfg:
            self.ui.load_config_to_ui(loaded_cfg, self.config_manager.save_dir)
            self.ui.settings_tab.hk_label.setText(
                f"Hotkey: {self.hotkey_manager.format_hotkey_string()}"
            )
        else:
            self.ui.reset_controls_to_default()

        self.ui.playback_tab.refresh_saved_songs(self.config_manager.save_dir)

        self._update_checker = UpdateChecker(APP_VERSION)
        self._update_checker.update_available.connect(self._on_update_available)
        self._update_checker.start()

        self.resize(self.minimumWidth(), self.minimumHeight())

    def _bind_signals(self):
        # UI controls bound strictly to Execution/Router logic
        self.ui.play_button.clicked.connect(self.handle_play)
        self.ui.stop_button.clicked.connect(self.handle_stop)
        self.ui.save_button.clicked.connect(self.handle_save)
        self.ui.reset_button.clicked.connect(self.ui.reset_controls_to_default)
        self.ui.playback_tab.browse_button.clicked.connect(self.select_file)
        self.ui.playback_tab.load_saved_btn.clicked.connect(self.open_load_dialog)
        self.ui.playback_tab.all_saves_btn.clicked.connect(self.open_load_dialog)
        self.ui.playback_tab.drop_zone.file_dropped.connect(self._open_midi)
        self.ui.settings_tab.save_browse_btn.clicked.connect(self._browse_save_dir)
        self.ui._collapsed_load_btn.clicked.connect(self.select_file)
        self.ui._collapsed_load_saved_btn.clicked.connect(self.open_load_dialog)
        self.ui.settings_tab.hk_btn.clicked.connect(self._change_hotkey)
        self.ui.settings_tab.check_update_btn.clicked.connect(self._manual_check_update)

        # View manipulations bound to Window behavior
        self.ui.collapse_btn.clicked.connect(self._sync_play_button)
        self.ui.settings_tab.always_top_check.toggled.connect(self._toggle_always_on_top)
        self.ui.settings_tab.opacity_slider.valueChanged.connect(self._change_opacity)

        # Settings-tab persistence — save immediately on change so closing without playing doesn't lose them
        self.ui.settings_tab.always_top_check.toggled.connect(self._save_config)
        self.ui.settings_tab.opacity_slider.valueChanged.connect(self._save_config)
        self.ui.settings_tab.timeline_vis_check.toggled.connect(self._save_config)
        self.ui.settings_tab.piano_vis_check.toggled.connect(self._save_config)

        # Translator tab
        self.ui.translator_tab.play_sheet_requested.connect(self._on_play_sheet)
        self.ui.translator_tab.export_requested.connect(self._on_export_sheet)

        # Timeline logic bridging
        self.ui.timeline_widget.seek_requested.connect(self._on_timeline_seek)
        self.ui.timeline_widget.scrub_position_changed.connect(self._on_visual_scrub)

        # External IO bridging
        self.hotkey_manager.toggle_requested.connect(self.toggle_playback_state)
        self.hotkey_manager.bound_updated.connect(self._on_hotkey_bound)

        # File strip reveal action
        self.ui.playback_tab.file_strip.reveal_requested.connect(self._reveal_in_explorer)

        # Edit Selection button on the LOADED card
        self.ui.playback_tab.edit_selection_requested.connect(self._edit_track_selection)
        self.ui.playback_tab.save_card_clicked.connect(self._on_save_card_quick_load)

        # System Logic bridging to the View representations
        self.playback_controller.status_updated.connect(self.ui.log_output.append)
        self.playback_controller.progress_updated.connect(self.update_progress)
        self.playback_controller.playback_finished.connect(self.on_playback_finished)
        self.playback_controller.visualizer_updated.connect(lambda p: self.ui.piano_widget.set_active_pitches(p))
        self.playback_controller.pedal_updated.connect(self.ui.piano_widget.set_pedal_active)
        self.playback_controller.auto_paused.connect(self._on_auto_paused)
        self.playback_controller.error_occurred.connect(self.show_error_dialog)
        self.playback_controller.timeline_data_ready.connect(self._on_timeline_data_ready)
        self.playback_controller.pedal_data_ready.connect(self._on_pedal_data_ready)
        self.playback_controller.save_successful.connect(self._on_save_successful)
        self.playback_controller.save_failed.connect(self._on_save_failed)

    # --- Windows Specific GUI Modifications ---
    def _toggle_always_on_top(self, checked):
        flags = self.windowFlags()
        if checked: self.setWindowFlags(flags | Qt.WindowType.WindowStaysOnTopHint)
        else: self.setWindowFlags(flags & ~Qt.WindowType.WindowStaysOnTopHint)
        self.show()

    def _change_opacity(self, value):
        self.setWindowOpacity(value / 100.0)

    # --- Standard Execution Behaviors ---
    def _save_config(self):
        config_data = self.ui.gather_app_config()
        self.config_manager.save(config_data)

    def _browse_save_dir(self):
        path = QFileDialog.getExistingDirectory(self, "Select Save Directory", self.config_manager.save_dir)
        if path:
            self.config_manager.set_save_dir(path)
            self.ui.settings_tab.save_path_input.setText(path)
            self._save_config()

    def _change_hotkey(self):
        QMessageBox.information(self, "Bind Key", "Press the key you want to bind now.")
        self.ui.settings_tab.hk_btn.setText("Listening...")
        self.ui.settings_tab.hk_btn.setEnabled(False)
        self.hotkey_manager.start_binding()

    def _on_hotkey_bound(self, key_str):
        self.ui.settings_tab.hk_label.setText(f"Hotkey: {key_str}")
        self.ui.settings_tab.hk_btn.setText("Change")
        self.ui.settings_tab.hk_btn.setEnabled(True)
        self._sync_play_button()

    def _sync_play_button(self):
        """Single authoritative update for the play button, derived from current playback state."""
        key_str = self.hotkey_manager.format_hotkey_string()
        if self.ui._is_collapsed:
            if self.playback_controller.is_paused():
                self.ui.play_button.setIcon(self.ui._icon_play)
                self.ui.play_button.setToolTip(f"Resume ({key_str})")
            elif self.playback_controller.is_playing():
                self.ui.play_button.setIcon(self.ui._icon_pause)
                self.ui.play_button.setToolTip(f"Pause ({key_str})")
            else:
                self.ui.play_button.setIcon(self.ui._icon_play)
                self.ui.play_button.setToolTip(f"Play ({key_str})")
        else:
            if self.playback_controller.is_paused():
                self.ui.play_button.setIcon(self.ui._icon_play)
                self.ui.play_button.setToolTip("Resume playback.")
            elif self.playback_controller.is_playing():
                self.ui.play_button.setIcon(self.ui._icon_pause)
                self.ui.play_button.setToolTip("Pause playback.")
            else:
                self.ui.play_button.setIcon(self.ui._icon_play)
                self.ui.play_button.setToolTip("Start playback.")

    def toggle_playback_state(self):
        if not self.playback_controller.is_paused():
            self.ui.piano_widget.clear()

        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            self.playback_controller.toggle_pause()
            self._sync_play_button()
            if not self.playback_controller.is_paused():
                current_t = self.ui.timeline_widget.current_time
                self._on_visual_scrub(current_t)
        elif self.ui.play_button.isEnabled():
            self.handle_play()

    def _on_auto_paused(self):
        self._sync_play_button()
        self.ui.piano_widget.clear()
        self.ui.stop_button.setEnabled(True)

    def _on_timeline_seek(self, time):
        self.ui.log_output.append(f"Seeking to {time:.2f}s...")
        self.playback_controller.seek(time)
    
    def _on_visual_scrub(self, time):
        active_pitches = set()
        lo = bisect.bisect_left(self._note_start_times, time - self._max_note_duration)
        hi = bisect.bisect_right(self._note_start_times, time)
        for note in self.current_notes[lo:hi]:
            if note.end_time > time:
                active_pitches.add(note.pitch)
        self.ui.piano_widget.set_active_pitches(list(active_pitches))
        pedal_down = any(s <= time < e for s, e in self.current_pedal_intervals)
        self.ui.piano_widget.set_pedal_active(pedal_down)
        self.ui.update_time_label(time, self.total_song_duration_sec)

    def _on_timeline_data_ready(self, notes, total_dur, tempo_map):
        self.current_notes = notes
        self._note_start_times = [n.start_time for n in notes]
        self._max_note_duration = max((n.duration for n in notes), default=0.0)
        self.total_song_duration_sec = total_dur
        self.ui.timeline_widget.set_data(notes, total_dur, tempo_map)
        self.ui.reset_timeline_position()

    def _on_pedal_data_ready(self, intervals: list):
        self.current_pedal_intervals = intervals
        self.ui.timeline_widget.set_pedal_intervals(intervals)

    def update_progress(self, current_time):
        self.ui.update_progress(current_time, self.total_song_duration_sec)

    # --- Loading & File State Dialogs ---
    def _reveal_in_explorer(self) -> None:
        path = self.ui.playback_tab.file_path_label.toolTip()
        if path and os.path.exists(path):
            import subprocess
            subprocess.Popen(["explorer", "/select,", os.path.normpath(path)])

    def select_file(self):
        if self.playback_controller.is_playing() or self.playback_controller.is_paused(): return
        filepath, _ = QFileDialog.getOpenFileName(self, "Select MIDI File", "", "MIDI Files (*.mid *.midi)")
        if filepath:
            self._open_midi(filepath)

    def _open_midi(self, filepath: str) -> None:
        if not filepath:
            return
        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            return
        self.loaded_save_data = None
        self.loaded_save_filename = None
        self._parsed_tracks = None
        self._loaded_pedal_count = 0
        self.ui.playback_tab.clear_loaded_summary()
        self.ui.playback_tab.set_groups_enabled(True)
        self.ui.update_file_label(os.path.basename(filepath), filepath)
        self.ui.log_output.append(f"Selected file: {filepath}")
        self._parse_and_select_tracks(filepath)
            
    def _apply_save(self, filepath: str, data: dict) -> None:
        """Apply a loaded save dict to UI state and stamp last_accessed on disk."""
        try:
            data.setdefault('metadata', {})['last_accessed'] = datetime.now().isoformat()
            with open(filepath, 'w') as f:
                json.dump(data, f, indent=4)
        except Exception:
            pass

        self.loaded_save_data = data
        self.loaded_save_filename = os.path.basename(filepath)
        self._parsed_tracks = None
        self._loaded_pedal_count = 0
        track_details = data.get('metadata', {}).get('track_details', [])
        compiled_pedal_count = data.get('metadata', {}).get('compiled_pedal_count', 0)
        self.ui.playback_tab.update_loaded_summary_from_save(track_details, compiled_pedal_count)
        if self.config.get('debug_mode'):
            self.ui.log_output.append(
                f"[DEBUG] Loaded save with {len(track_details)} track(s), "
                f"{compiled_pedal_count} compiled pedal event(s)."
            )
        self.ui.update_file_label(self.loaded_save_filename, filepath)
        self.ui.playback_tab.set_groups_enabled(False)
        self.ui._set_save_enabled(False)
        self.ui.play_button.setEnabled(True)
        self.ui.scrubber_slider.setEnabled(True)
        self.ui.log_output.append(f"Loaded save file: {self.loaded_save_filename}")
        self.ui.playback_tab.refresh_saved_songs(self.config_manager.save_dir)

    def open_load_dialog(self):
        dialog = LoadSaveDialog(self.config_manager.save_dir, self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            selected_file, data = dialog.get_selected_data()
            if selected_file and data:
                self._apply_save(selected_file, data)

    def _on_save_card_quick_load(self, filepath: str, save_name: str, song_name: str) -> None:
        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            return
        reply = QMessageBox.question(
            self,
            "Load Save",
            f"Load \"{save_name}\"?\n\nMIDI: {song_name}",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.Yes,
        )
        if reply != QMessageBox.StandardButton.Yes:
            return
        try:
            with open(filepath, 'r') as f:
                data = json.load(f)
        except Exception as e:
            QMessageBox.critical(self, "Error", f"Failed to read save file:\n{e}")
            return
        self._apply_save(filepath, data)

    def _parse_and_select_tracks(self, filepath):
        self.ui.log_output.append("Parsing MIDI structure...")
        try:
            tracks, tempo_map, pedal_count = MidiParser.parse_structure(filepath, 1.0, None)
        except Exception as e:
            QMessageBox.critical(self, "Error", f"Failed to parse MIDI:\n{e}")
            return

        self._parsed_tracks = tracks
        self._loaded_pedal_count = pedal_count
        self.parsed_tempo_map = tempo_map

        dialog = TrackSelectionDialog(tracks, self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            self.selected_tracks_info = dialog.get_selection()
            self.ui.log_output.append(f"Tracks selected: {len(self.selected_tracks_info)}")
            self.ui.playback_tab.update_loaded_summary(
                self.selected_tracks_info, self._loaded_pedal_count
            )
            self.ui.play_button.setEnabled(True)
            self.ui.scrubber_slider.setEnabled(True)
            self.ui._set_save_enabled(True)
        else:
            self.ui.log_output.append("Track selection cancelled.")
            self.selected_tracks_info = None
            self.ui.playback_tab.clear_loaded_summary()
            self.ui.play_button.setEnabled(False)
            self.ui.scrubber_slider.setEnabled(False)
            self.ui._set_save_enabled(False)

    def _edit_track_selection(self) -> None:
        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            return
        if not self._parsed_tracks:
            return
        dialog = TrackSelectionDialog(self._parsed_tracks, self)
        if dialog.exec() != QDialog.DialogCode.Accepted:
            return
        self.selected_tracks_info = dialog.get_selection()
        self.ui.log_output.append(
            f"Track selection updated: {len(self.selected_tracks_info)}"
        )
        self.ui.playback_tab.update_loaded_summary(
            self.selected_tracks_info, self._loaded_pedal_count
        )
        self.ui._set_save_enabled(bool(self.selected_tracks_info))
        self.ui.play_button.setEnabled(bool(self.selected_tracks_info))

    # --- Translator ---
    def _on_play_sheet(self, text: str, format_name: str, bpm: int, humanize: bool):
        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            return

        fmt = FormatRegistry.get(format_name)
        if not fmt:
            QMessageBox.critical(self, "Unknown Format", f"No handler found for format: {format_name}")
            return

        use_88 = self.ui.playback_tab.use_88_key_check.isChecked()
        key_mapper = KeyMapper(use_88_key_layout=use_88)

        try:
            notes = fmt.parse(text, float(bpm), key_mapper)
        except Exception as e:
            QMessageBox.critical(self, "Parse Error", f"Failed to parse sheet:\n{e}")
            return

        if not notes:
            QMessageBox.warning(self, "No Notes", "No playable notes were found in the pasted sheet.")
            return

        tempo_us = int(60_000_000 / bpm)
        tempo_map = TempoMap([(0, tempo_us)], [])

        if humanize:
            config = self.ui.gather_playback_config()
        else:
            config = {
                'use_88_key_layout': use_88, 'debug_mode': False, 'countdown': False,
                'pedal_style': 'none', 'simulate_hands': False, 'vary_velocity': False,
                'enable_chord_roll': False, 'vary_timing': False, 'timing_variance': 0.01,
                'vary_articulation': False, 'articulation': 0.95,
                'enable_drift_correction': False, 'drift_decay_factor': 0.25,
                'enable_mistakes': False, 'mistake_chance': 0.0,
                'enable_tempo_sway': False, 'tempo_sway_intensity': 0.0,
                'invert_tempo_sway': False, 'use_ai_pedal': False,
            }

        self.ui.log_output.append(f"Importing sheet: {len(notes)} notes at {bpm} BPM ({format_name})")
        self.playback_controller.play_from_notes(config, notes, tempo_map)
        self.ui.set_controls_enabled(False)
        self.ui.play_button.setEnabled(True)
        self.ui.stop_button.setEnabled(True)
        self.ui.scrubber_slider.setEnabled(True)
        self._sync_play_button()
        if self.ui._nav_btns[1].isEnabled():
            self.ui.tabs.setCurrentIndex(1)  # Switch to Visualizer

    def _on_export_sheet(self, format_name: str):
        if not self.current_notes:
            QMessageBox.warning(self, "No MIDI Loaded",
                                "Load and prepare a MIDI file on the Playback tab first.")
            return

        fmt = FormatRegistry.get(format_name)
        if not fmt:
            QMessageBox.critical(self, "Unknown Format", f"No handler found for format: {format_name}")
            return

        use_88 = self.ui.playback_tab.use_88_key_check.isChecked()
        key_mapper = KeyMapper(use_88_key_layout=use_88)
        tempo_map = getattr(self, 'parsed_tempo_map', TempoMap([(0, 500000)], []))

        try:
            text = fmt.serialize(self.current_notes, key_mapper, tempo_map)
        except Exception as e:
            QMessageBox.critical(self, "Export Error", f"Failed to generate sheet:\n{e}")
            return

        self.ui.translator_tab.set_export_text(text)
        self.ui.log_output.append(f"Sheet exported: {format_name} ({len(text.splitlines())} lines)")

    def show_error_dialog(self, error_message: str):
        self.ui.log_output.append("ERROR: Playback thread terminated unexpectedly due to an execution failure.")
        QMessageBox.critical(self, "Hardware/Execution Failure", error_message)

    # --- Core Executions ---
    def handle_save(self):
        config = self.ui.gather_playback_config()
        if not self.selected_tracks_info:
            QMessageBox.warning(self, "No Tracks", "Please select a MIDI file and choose tracks first.")
            return
            
        self._save_config()
        original_filename = os.path.basename(self.ui.playback_tab.file_path_label.toolTip())
        self.playback_controller.save(config, self.selected_tracks_info, self.config_manager.save_dir, original_filename)

    def _on_save_successful(self, filepath: str, message: str):
        self.ui.playback_tab.refresh_saved_songs(self.config_manager.save_dir)
        QMessageBox.information(self, "Save Successful", f"{message}\n{filepath}")

    def _on_save_failed(self, error_message: str):
        QMessageBox.critical(self, "Save Error", error_message)

    def handle_play(self):
        if self.playback_controller.is_playing() or self.playback_controller.is_paused(): 
            self.toggle_playback_state()
            return
            
        if self.loaded_save_data:
            self.playback_controller.play_from_save(self.loaded_save_data)
        else:
            config = self.ui.gather_playback_config()
            if not self.selected_tracks_info:
                QMessageBox.warning(self, "No Tracks", "Please select a MIDI file and choose tracks first.")
                return
            self.playback_controller.play(config, self.selected_tracks_info)
            
        self.ui.set_controls_enabled(False, bool(self.loaded_save_data))
        self.ui.play_button.setEnabled(True)
        self.ui.stop_button.setEnabled(True)
        self._sync_play_button()
        if self.ui._nav_btns[1].isEnabled():
            self.ui.tabs.setCurrentIndex(1)

    def handle_stop(self):
        self.playback_controller.stop()

    def on_playback_finished(self):
        self.ui.log_output.append("Playback process finished.\n" + "="*50 + "\n")
        self.ui.set_controls_enabled(True, bool(self.loaded_save_data))
        self.ui.stop_button.setEnabled(False)
        self._sync_play_button()
        self.ui.piano_widget.set_pedal_active(False)

    # --- Update ---
    def _manual_check_update(self):
        btn = self.ui.settings_tab.check_update_btn
        btn.setEnabled(False)
        btn.setText("Checking...")
        self._manual_checker = UpdateChecker(APP_VERSION, force=True)
        self._manual_checker.update_available.connect(self._on_update_available)
        self._manual_checker.update_available.connect(lambda *_: self._reset_update_btn())
        self._manual_checker.no_update.connect(self._on_no_update)
        self._manual_checker.check_failed.connect(self._on_check_failed)
        self._manual_checker.start()

    def _reset_update_btn(self):
        btn = self.ui.settings_tab.check_update_btn
        btn.setEnabled(True)
        btn.setText("Check for updates")

    def _on_no_update(self):
        self._reset_update_btn()
        QMessageBox.information(self, "Up to Date",
            f"HuMidi v{APP_VERSION} is the latest version.")

    def _on_check_failed(self):
        self._reset_update_btn()
        QMessageBox.warning(self, "Update Check Failed",
            "Could not reach GitHub.\nPlease check your internet connection.")

    def _on_update_available(self, latest_tag: str, releases_url: str):
        reply = QMessageBox.question(
            self, "Update Available",
            f"Update available to {latest_tag}. Would you like to open the download page?",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.Yes,
        )
        if reply == QMessageBox.StandardButton.Yes:
            webbrowser.open(releases_url)

    def closeEvent(self, event):
        self._update_checker.quit()
        self._save_config()
        self.playback_controller.shutdown()
        event.accept()

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec())