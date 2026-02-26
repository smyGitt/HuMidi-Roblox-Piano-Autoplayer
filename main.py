#!/usr/bin/env python3
import sys
import os
from PyQt6.QtWidgets import QApplication, QMainWindow, QMessageBox, QFileDialog, QDialog
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QIcon

from core.core import MidiParser
from managers.HotkeyManager import HotkeyManager
from controllers.PlaybackController import PlaybackController
from managers.ConfigManager import ConfigManager
from ui.MainWindowUI import MainWindowUI
from ui.TrackSelectionDialog import TrackSelectionDialog
from ui.LoadSaveDialog import LoadSaveDialog

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("HuMidi v1.3")
        self.setMinimumWidth(550)
        self.setMinimumHeight(683)

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
        self.current_notes = [] 
        self.total_song_duration_sec = 1.0

        self._bind_signals()
        
        # Load initialization data
        loaded_cfg = self.config_manager.load()
        if loaded_cfg:
            self.ui.load_config_to_ui(loaded_cfg, self.config_manager.save_dir)
            self.ui.hk_label.setText(f"Start/Stop Hotkey: {self.hotkey_manager._format_key_string(self.hotkey_manager.current_key)}")
        else:
            self.ui.reset_controls_to_default()

    def _bind_signals(self):
        # UI controls bound strictly to Execution/Router logic
        self.ui.play_button.clicked.connect(self.handle_play)
        self.ui.stop_button.clicked.connect(self.handle_stop)
        self.ui.save_button.clicked.connect(self.handle_save)
        self.ui.reset_button.clicked.connect(self.ui.reset_controls_to_default)
        self.ui.browse_button.clicked.connect(self.select_file)
        self.ui.load_saved_btn.clicked.connect(self.open_load_dialog)
        self.ui.save_browse_btn.clicked.connect(self._browse_save_dir)
        self.ui.hk_btn.clicked.connect(self._change_hotkey)
        
        # View manipulations bound to Window behavior
        self.ui.always_top_check.toggled.connect(self._toggle_always_on_top)
        self.ui.opacity_slider.valueChanged.connect(self._change_opacity)

        # Timeline logic bridging
        self.ui.timeline_widget.seek_requested.connect(self._on_timeline_seek)
        self.ui.timeline_widget.scrub_position_changed.connect(self._on_visual_scrub)

        # External IO bridging
        self.hotkey_manager.toggle_requested.connect(self.toggle_playback_state)
        self.hotkey_manager.bound_updated.connect(self._on_hotkey_bound)

        # System Logic bridging to the View representations
        self.playback_controller.status_updated.connect(self.ui.log_output.append)
        self.playback_controller.progress_updated.connect(self.update_progress)
        self.playback_controller.playback_finished.connect(self.on_playback_finished)
        self.playback_controller.visualizer_updated.connect(lambda p: self.ui.piano_widget.set_active_pitches(p))
        self.playback_controller.auto_paused.connect(self._on_auto_paused)
        self.playback_controller.error_occurred.connect(self.show_error_dialog)
        self.playback_controller.timeline_data_ready.connect(self._on_timeline_data_ready)
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
            self.ui.save_path_input.setText(path)
            self._save_config()

    def _change_hotkey(self):
        self.ui.hk_btn.setText("Listening...")
        self.ui.hk_btn.setEnabled(False)
        self.hotkey_manager.start_binding()
        QMessageBox.information(self, "Bind Key", "Press the key you want to bind now.")

    def _on_hotkey_bound(self, key_str):
        self.ui.hk_label.setText(f"Start/Stop Hotkey: {key_str}")
        self.ui.hk_btn.setText("Change")
        self.ui.hk_btn.setEnabled(True)
        self._update_play_stop_labels()

    def _update_play_stop_labels(self):
        key_str = self.hotkey_manager._format_key_string(self.hotkey_manager.current_key)
        if not self.playback_controller.is_playing() and not self.playback_controller.is_paused(): 
            self.ui.play_button.setText(f"Play ({key_str})")
        self.ui.stop_button.setText(f"Stop")

    def _update_pause_ui_state(self):
        key_str = self.hotkey_manager._format_key_string(self.hotkey_manager.current_key)
        if self.playback_controller.is_paused():
            self.ui.play_button.setText(f"Resume ({key_str})")
        else:
            self.ui.play_button.setText(f"Pause ({key_str})")

    def toggle_playback_state(self):
        if self.playback_controller.is_paused(): pass 
        else: self.ui.piano_widget.clear()

        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            self.playback_controller.toggle_pause()
            self._update_pause_ui_state()
            if not self.playback_controller.is_paused():
                current_t = self.ui.timeline_widget.current_time
                self._on_visual_scrub(current_t)
        elif self.ui.play_button.isEnabled():
            self.handle_play()

    def _on_auto_paused(self):
        self._update_pause_ui_state()
        self.ui.piano_widget.clear()
        self.ui.stop_button.setEnabled(True)

    def _on_timeline_seek(self, time):
        self.ui.log_output.append(f"Seeking to {time:.2f}s...")
        self.playback_controller.seek(time)
    
    def _on_visual_scrub(self, time):
        active_pitches = set()
        for note in self.current_notes:
            if note.start_time <= time < note.end_time: active_pitches.add(note.pitch)
        self.ui.piano_widget.set_active_pitches(list(active_pitches))
        self.ui.update_time_label(time, self.total_song_duration_sec)

    def _on_timeline_data_ready(self, notes, total_dur, tempo_map):
        self.current_notes = notes
        self.total_song_duration_sec = total_dur
        self.ui.timeline_widget.set_data(notes, total_dur, tempo_map)

    def update_progress(self, current_time):
        self.ui.update_progress(current_time, self.total_song_duration_sec)

    # --- Loading & File State Dialogs ---
    def select_file(self):
        if self.playback_controller.is_playing() or self.playback_controller.is_paused(): return
        filepath, _ = QFileDialog.getOpenFileName(self, "Select MIDI File", "", "MIDI Files (*.mid *.midi)")
        if filepath:
            self.loaded_save_data = None
            self.loaded_save_filename = None
            self.ui.playback_group.setEnabled(True)
            self.ui.humanization_group.setEnabled(True)
            self.ui.file_path_label.setText(os.path.basename(filepath))
            self.ui.file_path_label.setToolTip(filepath)
            self.ui.log_output.append(f"Selected file: {filepath}")
            self._parse_and_select_tracks(filepath)
            
    def open_load_dialog(self):
        dialog = LoadSaveDialog(self.config_manager.save_dir, self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            selected_file, data = dialog.get_selected_data()
            if selected_file and data:
                self.loaded_save_data = data
                self.loaded_save_filename = os.path.basename(selected_file)
                
                self.ui.file_path_label.setText(f"{self.loaded_save_filename}")
                self.ui.file_path_label.setToolTip(selected_file)
                
                self.ui.playback_group.setEnabled(False)
                self.ui.humanization_group.setEnabled(False)
                self.ui.save_button.setEnabled(False)
                self.ui.play_button.setEnabled(True)
                self.ui.log_output.append(f"Loaded save file: {self.loaded_save_filename}")

    def _parse_and_select_tracks(self, filepath):
        self.ui.log_output.append("Parsing MIDI structure...")
        try:
            tracks, tempo_map = MidiParser.parse_structure(filepath, 1.0, None)
        except Exception as e:
            QMessageBox.critical(self, "Error", f"Failed to parse MIDI:\n{e}")
            return
            
        dialog = TrackSelectionDialog(tracks, self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            self.selected_tracks_info = dialog.get_selection()
            self.parsed_tempo_map = tempo_map 
            self.ui.log_output.append(f"Tracks selected: {len(self.selected_tracks_info)}")
            self.ui.play_button.setEnabled(True)
            self.ui.save_button.setEnabled(True)
        else:
            self.ui.log_output.append("Track selection cancelled.")
            self.selected_tracks_info = None
            self.ui.play_button.setEnabled(False)
            self.ui.save_button.setEnabled(False)

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
        original_filename = os.path.basename(self.ui.file_path_label.toolTip())
        self.playback_controller.save(config, self.selected_tracks_info, self.config_manager.save_dir, original_filename)

    def _on_save_successful(self, filepath: str, message: str):
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
            self._save_config()
            self.playback_controller.play(config, self.selected_tracks_info)
            
        self.ui.set_controls_enabled(False, bool(self.loaded_save_data))
        self.ui.play_button.setEnabled(True) 
        self.ui.stop_button.setEnabled(True)
        key_str = self.hotkey_manager._format_key_string(self.hotkey_manager.current_key)
        self.ui.play_button.setText(f"Pause ({key_str})")
        self.ui.tabs.setCurrentIndex(1)

    def handle_stop(self):
        self.playback_controller.stop()

    def on_playback_finished(self):
        self.ui.log_output.append("Playback process finished.\n" + "="*50 + "\n")
        self.ui.set_controls_enabled(True, bool(self.loaded_save_data))
        self.ui.stop_button.setEnabled(False)
        self.ui.play_button.setText(f"Play ({self.hotkey_manager._format_key_string(self.hotkey_manager.current_key)})")

    def closeEvent(self, event):
        self.playback_controller.shutdown()
        event.accept()

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec())