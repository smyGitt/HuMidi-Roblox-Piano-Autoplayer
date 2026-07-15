import json
import os
import tempfile
import threading
from datetime import datetime

from PySide6.QtCore import QObject, QThread
from PySide6.QtWidgets import QMessageBox, QFileDialog, QDialog

from controllers.midi_parse_worker import MidiParseWorker
from ui.dialogs.TrackSelectionDialog import TrackSelectionDialog
from ui.dialogs.LoadSaveDialog import LoadSaveDialog
from ui.widgets import StatusIndicator


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


class LoadCoordinator(QObject):
    """Bridges MainWindowUI/ConfigManager/PlaybackController for MIDI-open and save-load flows.

    Owns the threaded MIDI structure parse (MidiParseWorker on its own QThread)
    and the track-selection / save-file dialogs.

    Must be a QObject (not a plain Python object): MidiParseWorker's signals
    are connected directly to this class's bound methods (_on_midi_parsed,
    _on_midi_parse_failed, _on_parse_cleanup), which build/exec GUI dialogs.
    Qt can only auto-queue a cross-thread signal connection to the GUI thread
    when it can determine the receiver's thread affinity from a QObject; a
    plain-object receiver has no thread affinity, so Qt falls back to a
    direct connection and runs the slot on the emitting worker thread. That
    manifested as "QObject::setParent: Cannot set parent, new parent is in a
    different thread" plus recursive-repaint/backing-store errors whenever
    TrackSelectionDialog was built and exec'd from the parse thread instead
    of the GUI thread.
    """

    def __init__(self, window, ui, config_manager, playback_controller, state):
        super().__init__()
        self.window = window
        self.ui = ui
        self.config_manager = config_manager
        self.playback_controller = playback_controller
        self.state = state
        self._parse_thread = None
        self._parse_worker = None

    def bind_signals(self) -> None:
        self.ui.playback_tab.browse_button.clicked.connect(self.select_file)
        self.ui.playback_tab.load_saved_btn.clicked.connect(self.open_load_dialog)
        self.ui.playback_tab.all_saves_btn.clicked.connect(self.open_load_dialog)
        self.ui.playback_tab.drop_zone.file_dropped.connect(self._open_midi)
        self.ui._collapsed_load_btn.clicked.connect(self.select_file)
        self.ui._collapsed_load_saved_btn.clicked.connect(self.open_load_dialog)
        self.ui.playback_tab.file_strip.reveal_requested.connect(self._reveal_in_explorer)
        self.ui.playback_tab.edit_selection_requested.connect(self._edit_track_selection)
        self.ui.playback_tab.save_card_clicked.connect(self._on_save_card_quick_load)

    def join_parse_thread(self) -> None:
        """Bounded-join the MIDI-parse QThread so it cannot outlive the window."""
        if self._parse_thread and self._parse_thread.isRunning():
            self._parse_thread.quit()
            self._parse_thread.wait(2000)

    def _reveal_in_explorer(self) -> None:
        path = self.ui.playback_tab.file_path_label.toolTip()
        if path and os.path.exists(path):
            import subprocess
            subprocess.Popen(["explorer", "/select,", os.path.normpath(path)])

    def select_file(self) -> None:
        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            return
        filepath, _ = QFileDialog.getOpenFileName(
            self.window, "Select MIDI File", self.config_manager.midi_dir, "MIDI Files (*.mid *.midi)"
        )
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
        self.ui.debug_tab.append_log(f"Selected file: {filepath}")
        self.ui.debug_tab.clear_snapshot()
        self.ui.debug_tab.update_snapshot({
            'file': os.path.basename(filepath),
            'source': 'MIDI file',
        })
        self._parse_and_select_tracks(filepath)

    def _apply_save(self, filepath: str, data: dict) -> None:
        """Apply a loaded save dict to UI state and stamp last_accessed on disk."""
        ok, reason = _validate_save_data(data)
        if not ok:
            QMessageBox.critical(
                self.window,
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
            self.ui.debug_tab.append_log(
                f"[SAVE] Loaded metadata: {len(track_details)} track(s) | "
                f"{compiled_pedal_count} compiled pedal event(s)"
            )
        self.ui.debug_tab.clear_snapshot()
        self.ui.debug_tab.update_snapshot({
            'file': self.state.loaded_save_filename,
            'source': 'Save file',
            'tracks': len(track_details),
            'pedal': f"{compiled_pedal_count} presses",
        })
        self.ui.update_file_label(self.state.loaded_save_filename, filepath)
        self.ui.playback_tab.set_groups_enabled(False)
        self.ui._set_save_enabled(False)
        self.ui.play_button.setEnabled(True)
        self.ui.scrubber_slider.setEnabled(True)
        self.ui.debug_tab.append_log(f"Loaded save file: {self.state.loaded_save_filename}")
        self.ui.playback_tab.refresh_saved_songs(self.config_manager.save_dir)
        self.ui._status_indicator.set_state(StatusIndicator.READY, "READY")

    def open_load_dialog(self) -> None:
        dialog = LoadSaveDialog(self.config_manager.save_dir, self.window)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            selected_file, data = dialog.get_selected_data()
            if selected_file and data:
                self._apply_save(selected_file, data)

    def _on_save_card_quick_load(self, filepath: str, save_name: str, song_name: str) -> None:
        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            return
        reply = QMessageBox.question(
            self.window,
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
            QMessageBox.critical(self.window, "Error", f"Failed to read save file:\n{e}")
            return
        self._apply_save(filepath, data)

    def _parse_and_select_tracks(self, filepath) -> None:
        """Parse the MIDI structure off the GUI thread, then open the dialog.

        MidiParser.parse_structure can take a noticeable amount of time on large
        files; running it on a MidiParseWorker QThread keeps the window (and the
        status-indicator animation) responsive. The TrackSelectionDialog is
        opened from the _on_midi_parsed slot once results arrive.
        """
        if self._parse_thread and self._parse_thread.isRunning():
            return
        self.ui.debug_tab.append_log("Parsing MIDI structure...")
        self.ui._status_indicator.set_state(StatusIndicator.LOADING, "LOADING MIDI")

        self._parse_thread = QThread()
        self._parse_worker = MidiParseWorker(filepath)
        self._parse_worker.moveToThread(self._parse_thread)
        self._parse_thread.started.connect(self._parse_worker.run)
        self._parse_worker.parsed.connect(self._on_midi_parsed)
        self._parse_worker.failed.connect(self._on_midi_parse_failed)
        self._parse_worker.finished.connect(self._on_parse_cleanup)
        self._parse_thread.start()

    def _on_midi_parsed(self, tracks, tempo_map, pedal_count, midi_pedal_events) -> None:
        self.state.parsed_tracks = tracks
        self.state.loaded_pedal_count = pedal_count
        self.state.midi_pedal_events = midi_pedal_events
        self.state.parsed_tempo_map = tempo_map
        self.ui.playback_tab.set_midi_pedal_available(pedal_count > 0)
        pedal_prompt_threshold = self.ui.settings_tab.pedal_prompt_threshold_spinbox.value()
        if pedal_count >= pedal_prompt_threshold:
            reply = QMessageBox.question(
                self.window, "Use MIDI Pedal?",
                f"This MIDI file contains {pedal_count} sustain pedal events from the "
                "source performance. Use these directly instead of generating new pedal "
                "events?",
                QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
                QMessageBox.StandardButton.No,
            )
            self.ui.playback_tab.use_midi_pedal_check.setChecked(
                reply == QMessageBox.StandardButton.Yes
            )

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
        _initial_bpm = 60_000_000 / _initial_tempo_us
        self.ui.playback_tab.update_bpm_display(_initial_bpm)
        self.ui.debug_tab.update_snapshot({'tempo': f"{_initial_bpm:.0f} BPM"})

        dialog = TrackSelectionDialog(tracks, self.window)
        if dialog.exec() == QDialog.DialogCode.Accepted:
            self.state.selected_tracks_info = dialog.get_selection()
            self.ui.debug_tab.append_log(f"Tracks selected: {len(self.state.selected_tracks_info)}")
            self.ui.debug_tab.update_snapshot({'tracks': len(self.state.selected_tracks_info)})
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
            self.ui.debug_tab.append_log("Track selection cancelled.")
            self.state.selected_tracks_info = None
            self.ui.playback_tab.clear_loaded_summary()
            self.ui.play_button.setEnabled(False)
            self.ui.scrubber_slider.setEnabled(False)
            self.ui._set_save_enabled(False)
            self.ui._status_indicator.set_state(StatusIndicator.UNLOADED, "NO FILE")

    def _on_midi_parse_failed(self, error_msg: str) -> None:
        QMessageBox.critical(self.window, "Error", f"Failed to parse MIDI:\n{error_msg}")
        self.ui._status_indicator.set_state(StatusIndicator.UNLOADED, "NO FILE")

    def _on_parse_cleanup(self) -> None:
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
        dialog = TrackSelectionDialog(self.state.parsed_tracks, self.window)
        if dialog.exec() != QDialog.DialogCode.Accepted:
            return
        self.state.selected_tracks_info = dialog.get_selection()
        self.ui.debug_tab.append_log(
            f"Track selection updated: {len(self.state.selected_tracks_info)}"
        )
        self.ui.debug_tab.update_snapshot({'tracks': len(self.state.selected_tracks_info)})
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
