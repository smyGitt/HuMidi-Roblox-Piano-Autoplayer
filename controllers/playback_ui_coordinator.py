import bisect
import os

from PySide6.QtWidgets import QMessageBox

from ui.widgets import StatusIndicator


class PlaybackUICoordinator:
    """Bridges MainWindowUI/PlaybackController for the play/pause/stop/save flow.

    Owns the status indicator sync, timeline scrubbing, two-phase compilation
    slots, the Apply/Discard toast flow, and the play button's authoritative
    text/tooltip state.
    """

    _STATUS_SHORT = [
        ("Preparing playback",                          "PREPPING"),
        ("Analyzing musical structure",                 "ANALYZING"),
        ("Compiling note events",                       "COMPILING"),
        ("Generating pedal events",                     "GEN. PEDAL"),
        ("Preparing playback from imported sheet",      "PREPPING"),
        ("Initializing playback from pre-compiled",     "LOADING SAVE"),
        ("Compiling data for serialization",            "SAVING"),
    ]

    def __init__(self, window, ui, playback_controller, hotkey_manager, config_manager, state):
        self.window = window
        self.ui = ui
        self.playback_controller = playback_controller
        self.hotkey_manager = hotkey_manager
        self.config_manager = config_manager
        self.state = state
        self._auto_compile_pedal_after_notes = False

    def bind_signals(self) -> None:
        self.ui.play_button.clicked.connect(self.handle_play)
        self.ui.stop_button.clicked.connect(self.handle_stop)
        self.ui.save_button.clicked.connect(self.handle_save)
        self.ui.collapse_btn.clicked.connect(self.sync_play_button)

        self.ui.timeline_widget.seek_requested.connect(self._on_timeline_seek)
        self.ui.timeline_widget.scrub_position_changed.connect(self._on_visual_scrub)

        self.hotkey_manager.toggle_requested.connect(self.toggle_playback_state)
        self.hotkey_manager.save_requested.connect(self.handle_save)

        self.playback_controller.status_updated.connect(self.ui.debug_tab.append_log)
        self.playback_controller.status_updated.connect(self._on_status_for_indicator)
        self.playback_controller.progress_updated.connect(self.update_progress)
        self.playback_controller.playback_finished.connect(self.on_playback_finished)
        self.playback_controller.visualizer_updated.connect(
            lambda p: self.ui.piano_widget.set_active_pitches(p)
        )
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
        self.playback_controller.notes_phase_done.connect(self._on_notes_phase_done)
        self.playback_controller.pedal_phase_done.connect(self._on_pedal_phase_done)
        self.playback_controller.session_ready.connect(self._on_session_ready)

        self.ui.playback_tab.tab_shown.connect(self._on_playback_tab_shown)
        self.ui.playback_tab.config_changed.connect(self._on_playback_tab_shown)
        self.ui.playback_tab.apply_requested.connect(self._on_apply_requested)
        self.ui.playback_tab.discard_requested.connect(self._on_discard_requested)
        self.ui.playback_tab.generate_pedal_requested.connect(self._on_generate_pedal_requested)

    def sync_play_button(self) -> None:
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

    def toggle_playback_state(self) -> None:
        if self.playback_controller.is_preparing():
            return  # no-op while the prepare worker is running
        if not self.playback_controller.is_paused():
            self.ui.piano_widget.clear()

        if self.playback_controller.is_playing() or self.playback_controller.is_paused():
            self.playback_controller.toggle_pause()
            self.sync_play_button()
            if not self.playback_controller.is_paused():
                current_t = self.ui.timeline_widget.current_time
                self._on_visual_scrub(current_t)
        elif self.ui.play_button.isEnabled():
            self.handle_play()

    def _on_auto_paused(self) -> None:
        self.sync_play_button()
        self.ui.piano_widget.clear()
        self.ui.stop_button.setEnabled(True)

    def _on_timeline_seek(self, time) -> None:
        self.ui.debug_tab.append_log(f"Seeking to {time:.2f}s...")
        self.playback_controller.seek(time)

    def _on_visual_scrub(self, time) -> None:
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

    def _on_timeline_data_ready(self, notes, total_dur, tempo_map) -> None:
        self.state.current_notes = notes
        self.state.note_start_times = [n.start_time for n in notes]
        self.state.max_note_duration = max((n.duration for n in notes), default=0.0)
        self.state.total_song_duration_sec = total_dur
        self.ui.timeline_widget.set_data(notes, total_dur, tempo_map)
        self.ui.reset_timeline_position()
        self.ui.debug_tab.update_snapshot({
            'notes': len(notes),
            'duration': f"{int(total_dur // 60)}:{int(total_dur % 60):02d}",
        })
        # Status is set by notes_phase_done / pedal_phase_done / session_ready signals.

    def _on_pedal_data_ready(self, intervals: list) -> None:
        self.state.current_pedal_intervals = intervals
        self.state.pedal_interval_starts = [s for s, _ in intervals]
        self.ui.timeline_widget.set_pedal_intervals(intervals)
        self.ui.debug_tab.update_snapshot({'pedal': f"{len(intervals)} presses"})

    def update_progress(self, current_time) -> None:
        self.ui.update_progress(current_time, self.state.total_song_duration_sec)

    def show_error_dialog(self, error_message: str) -> None:
        self.ui.debug_tab.append_log("ERROR: Playback halted by an execution failure.")
        QMessageBox.critical(self.window, "Hardware/Execution Failure", error_message)

    def _on_preparation_started(self) -> None:
        self.ui._status_indicator.set_state(StatusIndicator.LOADING, "PREPPING")
        self.ui.playback_tab.set_generate_pedal_enabled(False)

    def _on_playback_started(self) -> None:
        self.ui.play_button.setEnabled(True)
        self.ui.stop_button.setEnabled(True)
        self.sync_play_button()

    def _on_status_for_indicator(self, text: str) -> None:
        if self.ui._status_indicator.state != StatusIndicator.LOADING:
            return
        for prefix, short in self._STATUS_SHORT:
            if text.startswith(prefix):
                self.ui._status_indicator.set_text(short)
                return

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
        restore = self.playback_controller.get_restore_config()
        if restore and restore.get('pedal_style'):
            self.ui.debug_tab.update_snapshot({'pedal_style': restore['pedal_style']})

    def _on_session_ready(self) -> None:
        """Translator monolithic path done. Status -> READY."""
        self.ui._status_indicator.set_state(StatusIndicator.READY, "READY")

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
        if config.get('debug_mode'):
            self.ui.debug_tab.append_log(
                f"[APPLY] Recompile requested | notes_dirty={notes_dirty}"
            )
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

    def handle_save(self) -> None:
        config = self.ui.gather_playback_config()
        if not self.state.selected_tracks_info:
            QMessageBox.warning(self.window, "No Tracks", "Please select a MIDI file and choose tracks first.")
            return

        self.config_manager.save(self.ui.gather_app_config())
        original_filename = os.path.basename(self.ui.playback_tab.file_path_label.toolTip())
        self.playback_controller.save(
            config, self.state.selected_tracks_info, self.config_manager.save_dir, original_filename
        )

    def _on_save_successful(self, filepath: str, message: str) -> None:
        self.ui.playback_tab.refresh_saved_songs(self.config_manager.save_dir)
        QMessageBox.information(self.window, "Save Successful", f"{message}\n{filepath}")

    def _on_save_failed(self, error_message: str) -> None:
        QMessageBox.critical(self.window, "Save Error", error_message)

    def _prepare_ui_for_playback(self) -> None:
        """Disable controls and switch to the Visualizer tab for any play path."""
        self.ui.set_controls_enabled(False, bool(self.state.loaded_save_data))
        self.ui.stop_button.setEnabled(False)
        self.ui.play_button.setEnabled(False)
        if self.ui._nav_btns[1].isEnabled():
            self.ui.tabs.setCurrentIndex(1)

    def handle_play(self) -> None:
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
                QMessageBox.critical(self.window, "Incompatible Save", f"This save file could not be played:\n{e}")
                self.state.loaded_save_data = None
                self.state.loaded_save_filename = None
                self.ui.play_button.setEnabled(False)
            return

        # MIDI path.
        if not self.state.selected_tracks_info:
            QMessageBox.warning(self.window, "No Tracks", "Please select a MIDI file and choose tracks first.")
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
        if config.get('debug_mode'):
            self.ui.debug_tab.append_log(
                f"[PLAY] Blocked: compilation is stale | notes_dirty={notes_dirty} | "
                f"pedal_dirty={pedal_dirty_independent}"
            )
        self.ui.playback_tab.show_toast(notes_dirty, pedal_dirty_independent)
        self.ui.playback_tab.shake_toast()

    def handle_stop(self) -> None:
        self.playback_controller.stop()

    def on_playback_finished(self) -> None:
        self.ui.debug_tab.append_log("Playback finished.")
        self.ui.set_controls_enabled(True, bool(self.state.loaded_save_data))
        self.ui.stop_button.setEnabled(False)
        self.sync_play_button()
        self.ui.piano_widget.set_pedal_active(False)
        self.ui._status_indicator.set_state(StatusIndicator.READY, "READY")
        self.ui.playback_tab.set_generate_pedal_enabled(
            self.playback_controller.has_compiled_notes()
        )
