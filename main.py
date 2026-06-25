#!/usr/bin/env python3
import sys
import os
import json
import bisect
import tempfile
import threading
from datetime import datetime
from PyQt6.QtWidgets import QApplication, QMainWindow, QMessageBox, QFileDialog, QDialog
from PyQt6.QtCore import Qt, QThread
from PyQt6.QtGui import QIcon

from core.core import KeyMapper, TempoMap
from core.translator import FormatRegistry
import core.session_cache as session_cache
from managers.HotkeyManager import HotkeyManager
import webbrowser
from managers.UpdateManager import UpdateChecker
from controllers.PlaybackController import PlaybackController
from controllers.midi_parse_worker import MidiParseWorker
from controllers.app_state import AppState
from managers.ConfigManager import ConfigManager
from ui.MainWindowUI import MainWindowUI
from ui.dialogs.TrackSelectionDialog import TrackSelectionDialog
from ui.dialogs.LoadSaveDialog import LoadSaveDialog
from ui.widgets import StatusIndicator
from ui.theme import ThemeManager

APP_VERSION = "2.1"


def _validate_save_data(data: dict) -> tuple[bool, str]:
    """Return (ok, reason). reason is empty when ok is True."""
    metadata = data.get('metadata')
    events   = data.get('compiled_events')
    if not isinstance(metadata, dict):
        return False, "Missing or invalid metadata block."
    if not isinstance(events, list) or len(events) == 0:
        return False, "Missing or empty compiled_events list."
    ev0 = events[0]
    try:
        float(ev0['time']); int(ev0['priority']); str(ev0['action']); str(ev0['key_char'])
    except (KeyError, TypeError, ValueError):
        return False, "Compiled events have an unrecognised format."
    if 'track_details' not in metadata or 'compiled_pedal_count' not in metadata:
        return False, (
            "This save was created with an older version of HuMidi and is missing "
            "required fields (track_details, compiled_pedal_count).\n\n"
            "Please re-save your MIDI file with the current version."
        )
    return True, ''


def _write_json_atomic(filepath: str, data: dict) -> None:
    """Serialize `data` to `filepath` via a temp file + atomic os.replace.

    The original file is never truncated in place, so an interrupted write (for
    example a daemon thread killed at application exit) cannot corrupt an
    existing save: the replace either happened in full or not at all.
    """
    target_dir = os.path.dirname(filepath) or '.'
    fd, tmp_path = tempfile.mkstemp(dir=target_dir, suffix='.tmp')
    try:
        with os.fdopen(fd, 'w') as f:
            json.dump(data, f, indent=4)
        os.replace(tmp_path, filepath)
    except Exception:
        try:
            os.remove(tmp_path)
        except OSError:
            pass


def _stamp_last_accessed_async(filepath: str, data: dict) -> None:
    """Persist the already-updated `last_accessed` timestamp off the GUI thread.

    For large saves the JSON re-serialization is non-trivial; doing it on a
    daemon thread keeps the UI responsive right after a save card is clicked.
    The write is atomic (see _write_json_atomic), so backgrounding it is safe.
    """
    threading.Thread(
        target=_write_json_atomic, args=(filepath, data), daemon=True
    ).start()


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
        
        # Global Application States
        self.state = AppState()

        # Threaded track-selection parse (see _parse_and_select_tracks)
        self._parse_thread = None
        self._parse_worker = None

        self._bind_signals()

        # Load initialization data
        loaded_cfg = self.config_manager.load()
        if loaded_cfg:
            self.ui.load_config_to_ui(loaded_cfg, self.config_manager.save_dir)
        else:
            self.ui.reset_controls_to_default()
        self.ui.settings_tab.hk_label.setText(
            f"Hotkey: {self.hotkey_manager.format_hotkey_string()}"
        )
        self.ui.settings_tab.save_hk_label.setText(
            f"Hotkey: {self.hotkey_manager.format_save_hotkey_string()}"
        )

        self.ui.playback_tab.refresh_saved_songs(self.config_manager.save_dir)

        self._update_checker = UpdateChecker(APP_VERSION)
        self._update_checker.update_available.connect(self._on_update_available)
        self._update_checker.start()

        self.resize(self.ui._expanded_size)

    def _bind_signals(self):
        # UI controls bound strictly to Execution/Router logic
        self.ui.play_button.clicked.connect(self.handle_play)
        self.ui.stop_button.clicked.connect(self.handle_stop)
        self.ui.save_button.clicked.connect(self.handle_save)
        self.ui.settings_tab.reset_all_btn.clicked.connect(self.ui.reset_controls_to_default)
        self.ui.playback_tab.browse_button.clicked.connect(self.select_file)
        self.ui.playback_tab.load_saved_btn.clicked.connect(self.open_load_dialog)
        self.ui.playback_tab.all_saves_btn.clicked.connect(self.open_load_dialog)
        self.ui.playback_tab.drop_zone.file_dropped.connect(self._open_midi)
        self.ui.settings_tab.save_browse_btn.clicked.connect(self._browse_save_dir)
        self.ui.settings_tab.save_edit_btn.clicked.connect(self._open_save_dir)
        self.ui.settings_tab.themes_browse_btn.clicked.connect(self._browse_themes_dir)
        self.ui.settings_tab.themes_edit_btn.clicked.connect(self._open_themes_dir)
        self.ui._collapsed_load_btn.clicked.connect(self.select_file)
        self.ui._collapsed_load_saved_btn.clicked.connect(self.open_load_dialog)
        self.ui.settings_tab.hk_btn.clicked.connect(self._change_hotkey)
        self.ui.settings_tab.save_hk_btn.clicked.connect(self._change_save_hotkey)
        self.ui.settings_tab.check_update_btn.clicked.connect(self._manual_check_update)

        # View manipulations bound to Window behavior
        self.ui.collapse_btn.clicked.connect(self._sync_play_button)
        self.ui.settings_tab.always_top_check.toggled.connect(self._toggle_always_on_top)
        self.ui.settings_tab.opacity_slider.valueChanged.connect(self._change_opacity)

        # Settings-tab persistence: save immediately on change so closing without playing doesn't lose them
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
        self.hotkey_manager.save_requested.connect(self.handle_save)
        self.hotkey_manager.bound_save_updated.connect(self._on_save_hotkey_bound)

        # File strip reveal action
        self.ui.playback_tab.file_strip.reveal_requested.connect(self._reveal_in_explorer)

        # Edit Selection button on the LOADED card
        self.ui.playback_tab.edit_selection_requested.connect(self._edit_track_selection)
        self.ui.playback_tab.save_card_clicked.connect(self._on_save_card_quick_load)

        # System Logic bridging to the View representations
        self.playback_controller.status_updated.connect(self.ui.log_output.append)
        self.playback_controller.status_updated.connect(self._on_status_for_indicator)
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
        self.playback_controller.preparation_started.connect(self._on_preparation_started)
        self.playback_controller.playback_started.connect(self._on_playback_started)
        self.playback_controller.ai_pedal_thresholds_ready.connect(
            self.ui.playback_tab.set_ai_thresholds
        )
        self.playback_controller.ai_pedal_stats_ready.connect(
            self.ui.playback_tab.set_ai_pedal_stats
        )
        # Two-phase compilation signals
        self.playback_controller.notes_phase_done.connect(self._on_notes_phase_done)
        self.playback_controller.pedal_phase_done.connect(self._on_pedal_phase_done)
        self.playback_controller.session_ready.connect(self._on_session_ready)

        # Toast signals
        self.ui.playback_tab.tab_shown.connect(self._on_playback_tab_shown)
        self.ui.playback_tab.config_changed.connect(self._on_playback_tab_shown)
        self.ui.playback_tab.apply_requested.connect(self._on_apply_requested)
        self.ui.playback_tab.discard_requested.connect(self._on_discard_requested)
        self.ui.playback_tab.generate_pedal_requested.connect(self._on_generate_pedal_requested)

        # Pending flag for auto-pedal after notes compile (Apply path with notes dirty)
        self._auto_compile_pedal_after_notes: bool = False

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

    def _open_save_dir(self):
        import subprocess
        path = self.config_manager.save_dir
        if os.path.isdir(path):
            subprocess.Popen(["explorer", os.path.normpath(path)])

    def _browse_themes_dir(self):
        current = str(ThemeManager._themes_dir)
        path = QFileDialog.getExistingDirectory(self, "Select Themes Directory", current)
        if path:
            ThemeManager.set_themes_dir(path)
            self.ui.settings_tab.themes_path_input.setText(str(ThemeManager._themes_file))

    def _open_themes_dir(self):
        import subprocess
        themes_dir = ThemeManager._themes_dir
        themes_dir.mkdir(parents=True, exist_ok=True)
        subprocess.Popen(["explorer", os.path.normpath(str(themes_dir))])

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

    def _change_save_hotkey(self):
        QMessageBox.information(self, "Bind Key", "Press the key you want to bind now.")
        self.ui.settings_tab.save_hk_btn.setText("Listening...")
        self.ui.settings_tab.save_hk_btn.setEnabled(False)
        self.hotkey_manager.start_save_binding()

    def _on_save_hotkey_bound(self, key_str):
        self.ui.settings_tab.save_hk_label.setText(f"Hotkey: {key_str}")
        self.ui.settings_tab.save_hk_btn.setText("Change")
        self.ui.settings_tab.save_hk_btn.setEnabled(True)

    def _sync_play_button(self):
        """Single authoritative update for the play button, derived from current playback state."""
        key_str = self.hotkey_manager.format_hotkey_string()
        if self.ui._is_collapsed:
            if self.playback_controller.is_paused():
                self.ui.play_button.set_icon_name("play")
                self.ui.play_button.setToolTip(f"Resume ({key_str})")
            elif self.playback_controller.is_playing():
                self.ui.play_button.set_icon_name("pause")
                self.ui.play_button.setToolTip(f"Pause ({key_str})")
            else:
                self.ui.play_button.set_icon_name("play")
                self.ui.play_button.setToolTip(f"Play ({key_str})")
        else:
            if self.playback_controller.is_paused():
                self.ui.play_button.set_icon_name("play")
                self.ui.play_button.setToolTip("Resume playback.")
            elif self.playback_controller.is_playing():
                self.ui.play_button.set_icon_name("pause")
                self.ui.play_button.setToolTip("Pause playback.")
            else:
                self.ui.play_button.set_icon_name("play")
                self.ui.play_button.setToolTip("Start playback.")

    def toggle_playback_state(self):
        if self.playback_controller.is_preparing():
            return  # no-op while the prepare worker is running
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
        lo = bisect.bisect_left(self.state.note_start_times, time - self.state.max_note_duration)
        hi = bisect.bisect_right(self.state.note_start_times, time)
        for note in self.state.current_notes[lo:hi]:
            if note.end_time > time:
                active_pitches.add(note.pitch)
        self.ui.piano_widget.set_active_pitches(list(active_pitches))
        # Pedal intervals are non-overlapping and sorted by start, so a single
        # bisect locates the only candidate interval instead of scanning all.
        starts = self.state.pedal_interval_starts
        idx = bisect.bisect_right(starts, time) - 1
        pedal_down = idx >= 0 and time < self.state.current_pedal_intervals[idx][1]
        self.ui.piano_widget.set_pedal_active(pedal_down)
        self.ui.update_time_label(time, self.state.total_song_duration_sec)

    def _on_timeline_data_ready(self, notes, total_dur, tempo_map):
        self.state.current_notes = notes
        self.state.note_start_times = [n.start_time for n in notes]
        self.state.max_note_duration = max((n.duration for n in notes), default=0.0)
        self.state.total_song_duration_sec = total_dur
        self.ui.timeline_widget.set_data(notes, total_dur, tempo_map)
        self.ui.reset_timeline_position()
        # Status is set by notes_phase_done / pedal_phase_done / session_ready signals.

    def _on_pedal_data_ready(self, intervals: list):
        self.state.current_pedal_intervals = intervals
        self.state.pedal_interval_starts = [s for s, _ in intervals]
        self.ui.timeline_widget.set_pedal_intervals(intervals)

    def update_progress(self, current_time):
        self.ui.update_progress(current_time, self.state.total_song_duration_sec)

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
        if self._parse_thread and self._parse_thread.isRunning():
            return
        self.state.loaded_save_data = None
        self.state.loaded_save_filename = None
        self.state.parsed_tracks = None
        self.state.loaded_pedal_count = 0
        self.state.midi_pedal_events = []
        self.ui.playback_tab.set_midi_pedal_available(False)
        self.ui.playback_tab.clear_loaded_summary()
        self.ui.playback_tab.reset_pedal_ai_card()
        self.ui.playback_tab.update_bpm_display(0.0)
        self.ui.playback_tab.set_groups_enabled(True)
        self.ui.update_file_label(os.path.basename(filepath), filepath)
        self.ui.log_output.append(f"Selected file: {filepath}")
        self._parse_and_select_tracks(filepath)
            
    def _apply_save(self, filepath: str, data: dict) -> None:
        """Apply a loaded save dict to UI state and stamp last_accessed on disk."""
        ok, reason = _validate_save_data(data)
        if not ok:
            QMessageBox.critical(
                self,
                "Incompatible Save",
                f"This save cannot be loaded.\n\n{reason}\n\nFile: {os.path.basename(filepath)}",
            )
            return

        data.setdefault('metadata', {})['last_accessed'] = datetime.now().isoformat()
        _stamp_last_accessed_async(filepath, data)

        self.state.loaded_save_data = data
        self.state.loaded_save_filename = os.path.basename(filepath)
        self.state.parsed_tracks = None
        self.state.loaded_pedal_count = 0
        self.state.midi_pedal_events = []
        track_details = data.get('metadata', {}).get('track_details', [])
        compiled_pedal_count = data.get('metadata', {}).get('compiled_pedal_count', 0)
        self.ui.playback_tab.set_midi_pedal_available(False)
        self.ui.playback_tab.reset_pedal_ai_card()
        self.ui.playback_tab.update_loaded_summary_from_save(track_details, compiled_pedal_count)
        if self.ui.playback_tab.debug_check.isChecked():
            self.ui.log_output.append(
                f"[DEBUG] Loaded save with {len(track_details)} track(s), "
                f"{compiled_pedal_count} compiled pedal event(s)."
            )
        self.ui.update_file_label(self.state.loaded_save_filename, filepath)
        self.ui.playback_tab.set_groups_enabled(False)
        self.ui._set_save_enabled(False)
        self.ui.play_button.setEnabled(True)
        self.ui.scrubber_slider.setEnabled(True)
        self.ui.log_output.append(f"Loaded save file: {self.state.loaded_save_filename}")
        self.ui.playback_tab.refresh_saved_songs(self.config_manager.save_dir)
        self.ui._status_indicator.set_state(StatusIndicator.READY, "READY")

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
        """Parse the MIDI structure off the GUI thread, then open the dialog.

        MidiParser.parse_structure can take a noticeable amount of time on large
        files; running it on a MidiParseWorker QThread keeps the window (and the
        status-indicator animation) responsive. The TrackSelectionDialog is
        opened from the _on_midi_parsed slot once results arrive.
        """
        if self._parse_thread and self._parse_thread.isRunning():
            return
        self.ui.log_output.append("Parsing MIDI structure...")
        self.ui._status_indicator.set_state(StatusIndicator.LOADING, "LOADING MIDI")

        self._parse_thread = QThread()
        self._parse_worker = MidiParseWorker(filepath)
        self._parse_worker.moveToThread(self._parse_thread)
        self._parse_thread.started.connect(self._parse_worker.run)
        self._parse_worker.parsed.connect(self._on_midi_parsed)
        self._parse_worker.failed.connect(self._on_midi_parse_failed)
        self._parse_worker.finished.connect(self._on_parse_cleanup)
        self._parse_thread.start()

    def _on_midi_parsed(self, tracks, tempo_map, pedal_count, midi_pedal_events):
        self.state.parsed_tracks = tracks
        self.state.loaded_pedal_count = pedal_count
        self.state.midi_pedal_events = midi_pedal_events
        self.state.parsed_tempo_map = tempo_map
        self.ui.playback_tab.set_midi_pedal_available(pedal_count > 0)

        _events = tempo_map.events
        # events[0] is always a synthetic default entry (GlobalTickMap always
        # prepends it at tick 0 with 500000 us). The real first set_tempo event
        # lands at events[1] when the MIDI places it even one tick after tick 0,
        # which gives a tiny positive time_sec that get_tempo_at(0.0) misses.
        # Using events[1] directly (if it starts within 5 s) avoids that gap.
        if len(_events) > 1 and _events[1][0] <= 5.0:
            _initial_tempo_us = _events[1][1]
        else:
            _initial_tempo_us = _events[0][1] if _events else 500000
        self.ui.playback_tab.update_bpm_display(60_000_000 / _initial_tempo_us)

        dialog = TrackSelectionDialog(tracks, self)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            self.state.selected_tracks_info = dialog.get_selection()
            self.ui.log_output.append(f"Tracks selected: {len(self.state.selected_tracks_info)}")
            self.ui.playback_tab.update_loaded_summary(
                self.state.selected_tracks_info, self.state.loaded_pedal_count
            )
            self.ui.play_button.setEnabled(True)
            self.ui.scrubber_slider.setEnabled(True)
            self.ui._set_save_enabled(True)
            # Invalidate any prior compiled state and kick off phase-1 notes compilation.
            self.playback_controller.invalidate_notes_cache()
            config = self.ui.gather_playback_config()
            self.playback_controller.compile_notes(config, self.state.selected_tracks_info)
        else:
            self.ui.log_output.append("Track selection cancelled.")
            self.state.selected_tracks_info = None
            self.ui.playback_tab.clear_loaded_summary()
            self.ui.play_button.setEnabled(False)
            self.ui.scrubber_slider.setEnabled(False)
            self.ui._set_save_enabled(False)
            self.ui._status_indicator.set_state(StatusIndicator.UNLOADED, "NO FILE")

    def _on_midi_parse_failed(self, error_msg: str):
        QMessageBox.critical(self, "Error", f"Failed to parse MIDI:\n{error_msg}")
        self.ui._status_indicator.set_state(StatusIndicator.UNLOADED, "NO FILE")

    def _on_parse_cleanup(self):
        if self._parse_thread:
            self._parse_thread.quit()
            self._parse_thread.wait(2000)
        self._parse_worker = None
        self._parse_thread = None

    def _edit_track_selection(self) -> None:
        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            return
        if not self.state.parsed_tracks:
            return
        dialog = TrackSelectionDialog(self.state.parsed_tracks, self)
        if dialog.exec() != QDialog.DialogCode.Accepted:
            return
        self.state.selected_tracks_info = dialog.get_selection()
        self.ui.log_output.append(
            f"Track selection updated: {len(self.state.selected_tracks_info)}"
        )
        self.ui.playback_tab.update_loaded_summary(
            self.state.selected_tracks_info, self.state.loaded_pedal_count
        )
        self.ui._set_save_enabled(bool(self.state.selected_tracks_info))
        self.ui.play_button.setEnabled(bool(self.state.selected_tracks_info))
        if self.state.selected_tracks_info:
            self.playback_controller.invalidate_notes_cache()
            self.ui.playback_tab.hide_toast()
            config = self.ui.gather_playback_config()
            self.playback_controller.compile_notes(config, self.state.selected_tracks_info)

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
        if not self.state.current_notes:
            QMessageBox.warning(self, "No MIDI Loaded",
                                "Load and prepare a MIDI file on the Playback tab first.")
            return

        fmt = FormatRegistry.get(format_name)
        if not fmt:
            QMessageBox.critical(self, "Unknown Format", f"No handler found for format: {format_name}")
            return

        use_88 = self.ui.playback_tab.use_88_key_check.isChecked()
        key_mapper = KeyMapper(use_88_key_layout=use_88)
        tempo_map = self.state.parsed_tempo_map or TempoMap([(0, 500000)], [])

        try:
            text = fmt.serialize(self.state.current_notes, key_mapper, tempo_map)
        except Exception as e:
            QMessageBox.critical(self, "Export Error", f"Failed to generate sheet:\n{e}")
            return

        self.ui.translator_tab.set_export_text(text)
        self.ui.log_output.append(f"Sheet exported: {format_name} ({len(text.splitlines())} lines)")

    def show_error_dialog(self, error_message: str):
        self.ui.log_output.append("ERROR: Playback thread terminated unexpectedly due to an execution failure.")
        QMessageBox.critical(self, "Hardware/Execution Failure", error_message)

    # --- Status Indicator Slots ---

    def _on_preparation_started(self):
        self.ui._status_indicator.set_state(StatusIndicator.LOADING, "PREPPING")
        self.ui.playback_tab.set_generate_pedal_enabled(False)

    def _on_playback_started(self):
        self.ui.play_button.setEnabled(True)
        self.ui.stop_button.setEnabled(True)
        self._sync_play_button()

    _STATUS_SHORT = [
        ("Preparing playback",                          "PREPPING"),
        ("Analyzing musical structure",                 "ANALYZING"),
        ("Compiling note events",                       "COMPILING"),
        ("Generating pedal events",                     "GEN. PEDAL"),
        ("Preparing playback from imported sheet",      "PREPPING"),
        ("Initializing playback from pre-compiled",     "LOADING SAVE"),
        ("Compiling data for serialization",            "SAVING"),
    ]

    def _on_status_for_indicator(self, text: str) -> None:
        if self.ui._status_indicator.state != StatusIndicator.LOADING:
            return
        for prefix, short in self._STATUS_SHORT:
            if text.startswith(prefix):
                self.ui._status_indicator.set_text(short)
                return

    # --- Two-phase compilation slots ---

    def _on_notes_phase_done(self) -> None:
        """Phase 1 complete: notes humanized and cached. Status -> LOADED."""
        self.ui._status_indicator.set_state(StatusIndicator.LOADED, "NOTES RDY")
        if self._auto_compile_pedal_after_notes:
            self._auto_compile_pedal_after_notes = False
            config = self.ui.gather_playback_config()
            self.playback_controller.compile_pedal(config)
        else:
            self.ui.playback_tab.set_generate_pedal_enabled(True)

    def _on_pedal_phase_done(self) -> None:
        """Phase 2 complete (MIDI path): pedal generated and merged. Status -> READY."""
        self.ui._status_indicator.set_state(StatusIndicator.READY, "READY")
        self.ui.playback_tab.set_generate_pedal_enabled(True)

    def _on_session_ready(self) -> None:
        """Translator monolithic path done. Status -> READY."""
        self.ui._status_indicator.set_state(StatusIndicator.READY, "READY")

    # --- Toast slots ---

    def _on_playback_tab_shown(self) -> None:
        """Check dirty flags when user navigates to PlaybackTab; show toast if stale."""
        if not self.playback_controller.has_compiled_notes():
            return
        config = self.ui.gather_playback_config()
        notes_fresh = self.playback_controller.notes_match_config(config)
        pedal_fresh = (
            self.playback_controller.pedal_ever_compiled()
            and self.playback_controller.pedal_match_config(config)
        )
        if notes_fresh and (not self.playback_controller.pedal_ever_compiled() or pedal_fresh):
            self.ui.playback_tab.hide_toast()
            return
        notes_dirty = not notes_fresh
        pedal_dirty_independent = (
            self.playback_controller.pedal_ever_compiled()
            and not self.playback_controller.pedal_match_config(config)
        )
        self.ui.playback_tab.show_toast(notes_dirty, pedal_dirty_independent)

    def _on_apply_requested(self) -> None:
        """Apply button pressed: recompile whichever phase(s) are stale."""
        if self.playback_controller.is_preparing() or self.playback_controller.is_playing():
            return
        config = self.ui.gather_playback_config()
        notes_dirty = not self.playback_controller.notes_match_config(config)
        if notes_dirty:
            # Notes must be recompiled; pedal must follow automatically.
            self._auto_compile_pedal_after_notes = True
            self.playback_controller.compile_notes(config, self.state.selected_tracks_info)
        else:
            self.playback_controller.compile_pedal(config)

    def _on_generate_pedal_requested(self) -> None:
        """Generate button in PedalAI card pressed: re-run phase-2 pedal compilation."""
        if self.playback_controller.is_preparing() or self.playback_controller.is_playing():
            return
        if not self.playback_controller.has_compiled_notes():
            return
        config = self.ui.gather_playback_config()
        self.playback_controller.compile_pedal(config)

    def _on_discard_requested(self) -> None:
        """Discard button pressed: restore UI to the last compiled snapshot."""
        restore = self.playback_controller.get_restore_config()
        if restore:
            self.ui.playback_tab.restore_from_runtime_config(restore)
        self.ui.playback_tab.hide_toast()

    # --- Core Executions ---
    def handle_save(self):
        config = self.ui.gather_playback_config()
        if not self.state.selected_tracks_info:
            QMessageBox.warning(self, "No Tracks", "Please select a MIDI file and choose tracks first.")
            return

        self._save_config()
        original_filename = os.path.basename(self.ui.playback_tab.file_path_label.toolTip())
        self.playback_controller.save(config, self.state.selected_tracks_info, self.config_manager.save_dir, original_filename)

    def _on_save_successful(self, filepath: str, message: str):
        self.ui.playback_tab.refresh_saved_songs(self.config_manager.save_dir)
        QMessageBox.information(self, "Save Successful", f"{message}\n{filepath}")

    def _on_save_failed(self, error_message: str):
        QMessageBox.critical(self, "Save Error", error_message)

    def _prepare_ui_for_playback(self) -> None:
        """Disable controls and switch to the Visualizer tab for any play path."""
        self.ui.set_controls_enabled(False, bool(self.state.loaded_save_data))
        self.ui.stop_button.setEnabled(False)
        self.ui.play_button.setEnabled(False)
        if self.ui._nav_btns[1].isEnabled():
            self.ui.tabs.setCurrentIndex(1)

    def handle_play(self):
        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            self.toggle_playback_state()
            return
        if self.playback_controller.is_preparing():
            return

        # Save path: unchanged.
        if self.state.loaded_save_data:
            try:
                self._prepare_ui_for_playback()
                self.playback_controller.play_from_save(self.state.loaded_save_data)
            except Exception as e:
                QMessageBox.critical(self, "Incompatible Save", f"This save file could not be played:\n{e}")
                self.state.loaded_save_data = None
                self.state.loaded_save_filename = None
                self.ui.play_button.setEnabled(False)
            return

        # MIDI path.
        if not self.state.selected_tracks_info:
            QMessageBox.warning(self, "No Tracks", "Please select a MIDI file and choose tracks first.")
            return

        config = self.ui.gather_playback_config()
        pc = self.playback_controller
        notes_fresh = pc.has_compiled_notes() and pc.notes_match_config(config)
        pedal_compiled = pc.pedal_ever_compiled()
        pedal_fresh = pedal_compiled and pc.pedal_match_config(config)

        if notes_fresh and not pedal_compiled:
            # Notes ready but pedal never generated: compile pedal then play.
            self._prepare_ui_for_playback()
            pc.compile_pedal_and_play(config)
            return

        if notes_fresh and pedal_fresh:
            # Both compiled and current: play immediately without recompiling.
            self._prepare_ui_for_playback()
            pc.start_playback(config)
            return

        # Events are stale: navigate to Playback sub-tab and show the toast.
        # Do NOT start playback.
        self.ui.tabs.setCurrentIndex(0)
        self.ui.playback_tab.navigate_to_playback_sub_tab()
        notes_dirty = not notes_fresh
        pedal_dirty_independent = pedal_compiled and not pedal_fresh
        self.ui.playback_tab.show_toast(notes_dirty, pedal_dirty_independent)
        self.ui.playback_tab.shake_toast()

    def handle_stop(self):
        self.playback_controller.stop()

    def on_playback_finished(self):
        self.ui.log_output.append("Playback process finished.\n" + "="*50 + "\n")
        self.ui.set_controls_enabled(True, bool(self.state.loaded_save_data))
        self.ui.stop_button.setEnabled(False)
        self._sync_play_button()
        self.ui.piano_widget.set_pedal_active(False)
        self.ui._status_indicator.set_state(StatusIndicator.READY, "READY")
        self.ui.playback_tab.set_generate_pedal_enabled(
            self.playback_controller.has_compiled_notes()
        )

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
        # Join every background thread (bounded) so none outlives the window and
        # later emits a signal into a destroyed MainWindow.
        self._update_checker.quit()
        self._update_checker.wait(2000)
        manual_checker = getattr(self, '_manual_checker', None)
        if manual_checker is not None:
            manual_checker.quit()
            manual_checker.wait(2000)
        if self._parse_thread and self._parse_thread.isRunning():
            self._parse_thread.quit()
            self._parse_thread.wait(2000)
        self._save_config()
        self.playback_controller.shutdown()
        session_cache.clear_cache()
        event.accept()

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = MainWindow()
    window.show()
    sys.exit(app.exec())