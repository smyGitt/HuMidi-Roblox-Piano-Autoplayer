import copy
import json
from datetime import datetime
from pathlib import Path
from typing import Dict, List

from PyQt6.QtCore import QObject, QThread, pyqtSignal as Signal

from core.models import Note, KeyEvent
from core.core import MidiParser, TempoMap
from core.section_analyzer import SectionAnalyzer, assign_hands
from core.player import Player
from core.compiler import compile_events


def _extract_pedal_intervals(pedal_events) -> list:
    """Convert a flat list of pedal KeyEvents into (start_sec, end_sec) interval tuples."""
    intervals = []
    down_time = None
    for ev in pedal_events:
        if ev.action != 'pedal':
            continue
        if ev.key_char == 'down':
            down_time = ev.time
        elif ev.key_char == 'up' and down_time is not None:
            intervals.append((down_time, ev.time))
            down_time = None
    return intervals


def _apply_hand_assignment(notes: List[Note], config: Dict, log=None) -> None:
    """Resolve still-unknown note hands in place.

    Shared by _prepare_notes (MIDI path) and the pre-built-notes (translator)
    path so the simulate-hands-vs-pitch-threshold rule lives in exactly one
    place.
    """
    if config.get('simulate_hands'):
        if log:
            log(f"[PREP] Simulating hands for {sum(1 for n in notes if n.hand == 'unknown')} unassigned notes")
        assign_hands(notes)
    else:
        defaulted = sum(1 for n in notes if n.hand == 'unknown')
        for note in notes:
            if note.hand == 'unknown':
                note.hand = 'left' if note.pitch < 60 else 'right'
        if log and defaulted:
            log(f"[PREP] Hand simulation off: defaulted {defaulted} unassigned notes by pitch threshold (pitch < 60 = left)")


def _prepare_notes(config: Dict, selected_tracks_info: List, log=None):
    """Parse MIDI, apply track role assignments, and run hand simulation.

    Shared by _PrepareWorker.run() and _SaveWorker.run() to eliminate the
    duplicated note-preparation pipeline that previously existed in both places.

    Returns (final_notes, tempo_map). Raises on MIDI parse failure -- callers
    should catch and surface the exception appropriately.
    """
    tempo_scale = config.get('tempo', 100.0) / 100.0
    if log:
        log(f"[PREP] Parsing MIDI: tempo_scale={tempo_scale:.3f} ({config.get('tempo', 100.0):.1f}%)")
    tracks, tempo_map, pedal_cc_count = MidiParser.parse_structure(config['midi_file'], tempo_scale, log)
    selected_indices = [t.index for t, _ in selected_tracks_info]
    role_map = {t.index: r for t, r in selected_tracks_info}
    final_notes = []

    for track in tracks:
        if track.index in selected_indices:
            role = role_map[track.index]
            if log:
                log(f"[PREP] Track {track.index} ({track.name}): {len(track.notes)} notes | Role: {role}")
            for note in track.notes:
                new_note = copy.deepcopy(note)
                if role == "Left Hand": new_note.hand = 'left'
                elif role == "Right Hand": new_note.hand = 'right'
                final_notes.append(new_note)

    final_notes.sort(key=lambda n: n.start_time)

    _apply_hand_assignment(final_notes, config, log)

    if log:
        l_count = sum(1 for n in final_notes if n.hand == 'left')
        r_count = sum(1 for n in final_notes if n.hand == 'right')
        log(f"[PREP] Final note set: {len(final_notes)} notes | L={l_count} R={r_count} | source pedal CC events={pedal_cc_count}")

    return final_notes, tempo_map


class _PrepareWorker(QObject):
    """Runs the full off-thread preparation pipeline on a QThread.

    Pipeline: note preparation (MIDI parse + role/hand assignment, OR pre-built
    translator notes) -> SectionAnalyzer.analyze() -> compile_events(). The
    compiled KeyEvent list is emitted so the player thread only has to execute
    it. This is the single place pedal/humanization compilation happens for a
    normal play, which is why the timeline pedal preview now always matches what
    is actually played, and why the AI BiLSTM never runs twice per play.

    Construct with either selected_tracks_info (MIDI file path in config) or
    pre-built notes + tempo_map (translator sheet import). Exactly one is used.

    Emits prepare_finished with everything needed to start the Player, or
    error_occurred on failure. Always emits finished last for thread cleanup.
    Cancellation is cooperative: cancel() sets a flag checked between stages so
    a window close mid-preparation bails out instead of pushing a Player onto a
    torn-down controller.
    """
    status_updated      = Signal(str)
    prepare_finished    = Signal(object, object, object, float, object)  # notes, compiled_events, tempo_map, total_dur, pedal_intervals
    ai_thresholds_ready = Signal(float, float)  # threshold_on, threshold_off (raw sigmoid)
    pedal_stats_ready   = Signal(float, float, float, float)  # avg_dur, min_dur, max_dur, presses_per_min
    error_occurred      = Signal(str)
    finished            = Signal()

    def __init__(self, config: Dict, selected_tracks_info: List = None,
                 notes: List = None, tempo_map: TempoMap = None):
        super().__init__()
        self.config = config
        self.selected_tracks_info = selected_tracks_info
        self._prebuilt_notes = notes
        self._prebuilt_tempo_map = tempo_map
        self._cancelled = False

    def cancel(self):
        self._cancelled = True

    def run(self):
        debug_log = self.status_updated.emit if self.config.get('debug_mode') else None
        if debug_log and self.selected_tracks_info is not None:
            debug_log("\n" + "=" * 60)
            debug_log("=== PLAYBACK SESSION START (MIDI file) ===")
            debug_log("=" * 60)
            debug_log("[CONFIG] " + " | ".join(
                f"{k}={v}" for k, v in sorted(self.config.items())
                if k not in ('midi_file',)
            ))
            debug_log(f"[CONFIG] midi_file: {self.config.get('midi_file', 'N/A')}")
            debug_log(f"[CONFIG] Tracks selected: {len(self.selected_tracks_info)}")
            for t, role in self.selected_tracks_info:
                debug_log(f"  Track {t.index} ({t.name}): {t.note_count} notes | Role: {role} | Instrument: {t.instrument_name}")
            debug_log("\n=== RAW MIDI DATA (Selected Tracks) ===")

        try:
            if self._prebuilt_notes is not None:
                final_notes = self._prebuilt_notes
                tempo_map = self._prebuilt_tempo_map
                _apply_hand_assignment(final_notes, self.config, debug_log)
            else:
                final_notes, tempo_map = _prepare_notes(
                    self.config, self.selected_tracks_info, log=debug_log
                )
        except Exception as e:
            self.error_occurred.emit(f"Error preparing playback:\n{e}")
            self.finished.emit()
            return

        if self._cancelled:
            self.finished.emit()
            return

        self.status_updated.emit("Analyzing musical structure...")
        analyzer = SectionAnalyzer(final_notes, tempo_map, debug_log=debug_log)
        sections = analyzer.analyze()

        if self._cancelled:
            self.finished.emit()
            return

        total_dur = max((n.end_time for n in final_notes), default=1.0) if final_notes else 1.0

        self.status_updated.emit("Compiling playback events...")
        out_meta: dict = {}
        try:
            compiled_events = compile_events(
                self.config, final_notes, sections, log=debug_log, out_meta=out_meta
            )
        except Exception as e:
            self.error_occurred.emit(f"Error compiling playback:\n{e}")
            self.finished.emit()
            return

        if self._cancelled:
            self.finished.emit()
            return

        pedal_intervals = _extract_pedal_intervals(
            [ev for ev in compiled_events if ev.action == 'pedal']
        )

        if 'threshold_on' in out_meta and 'threshold_off' in out_meta:
            self.ai_thresholds_ready.emit(
                float(out_meta['threshold_on']),
                float(out_meta['threshold_off']),
            )
            if pedal_intervals:
                durations = [end - start for start, end in pedal_intervals]
                avg_dur = sum(durations) / len(durations)
                min_dur = min(durations)
                max_dur = max(durations)
                presses_per_min = len(pedal_intervals) / (total_dur / 60.0) if total_dur > 0 else 0.0
                self.pedal_stats_ready.emit(avg_dur, min_dur, max_dur, presses_per_min)

        self.prepare_finished.emit(final_notes, compiled_events, tempo_map, total_dur, pedal_intervals)
        self.finished.emit()


class _SaveWorker(QObject):
    status_updated = Signal(str)
    save_successful = Signal(str, str)
    save_failed = Signal(str)
    finished = Signal()

    def __init__(self, config: Dict, selected_tracks_info: List, save_dir: str, original_filename: str):
        super().__init__()
        self.config = config
        self.selected_tracks_info = selected_tracks_info
        self.save_dir = save_dir
        self.original_filename = original_filename

    def run(self):
        self.status_updated.emit("Compiling data for serialization...")

        debug_log = self.status_updated.emit if self.config.get('debug_mode') else None
        if debug_log:
            debug_log("\n" + "=" * 60)
            debug_log("=== SAVE SESSION START ===")
            debug_log("=" * 60)
            debug_log(f"[SAVE] Source: {self.original_filename}")
            debug_log(f"[SAVE] Target dir: {self.save_dir}")
            debug_log(f"[SAVE] Tracks selected: {len(self.selected_tracks_info)}")
            for t, role in self.selected_tracks_info:
                debug_log(f"  Track {t.index} ({t.name}): {t.note_count} notes | Role: {role}")

        try:
            final_notes, tempo_map = _prepare_notes(self.config, self.selected_tracks_info, log=debug_log)
        except Exception as e:
            if debug_log:
                debug_log(f"[SAVE] FAILED at _prepare_notes: {e}")
            self.save_failed.emit(f"Error preparing save data:\n{e}")
            self.finished.emit()
            return

        analyzer = SectionAnalyzer(final_notes, tempo_map, debug_log=debug_log)
        sections = analyzer.analyze()

        events_to_serialize = compile_events(
            self.config, final_notes, sections,
            log=self.status_updated.emit if self.config.get('debug_mode') else None,
        )

        if not events_to_serialize:
            self.save_failed.emit(
                "Compilation produced zero events -- nothing to save.\n"
                "Verify that the selected tracks contain notes within the keyboard's playable range."
            )
            self.finished.emit()
            return

        serialized_events = [
            {'time': ev.time, 'priority': ev.priority, 'action': ev.action,
             'key_char': ev.key_char, 'pitch': ev.pitch}
            for ev in events_to_serialize
        ]

        track_details = []
        for track, role in self.selected_tracks_info:
            pitches = [n.pitch for n in track.notes]
            track_details.append({
                'name': track.name or '',
                'note_count': len(track.notes),
                'pitch_min': min(pitches) if pitches else None,
                'pitch_max': max(pitches) if pitches else None,
                'role': role,
            })

        # Single pass over the serialized events for every count we need
        # (metadata pedal-down total plus the debug action breakdown), instead
        # of four separate O(n) scans.
        action_counts = {'press': 0, 'release': 0, 'pedal': 0}
        compiled_pedal_count = 0
        for ev in serialized_events:
            action = ev['action']
            if action in action_counts:
                action_counts[action] += 1
            if action == 'pedal' and ev['key_char'] == 'down':
                compiled_pedal_count += 1

        if debug_log:
            debug_log(
                f"[SAVE] Serializing {len(serialized_events)} events "
                f"(press={action_counts['press']} release={action_counts['release']} "
                f"pedal={action_counts['pedal']}) | "
                f"{len(track_details)} track(s) | {compiled_pedal_count} pedal-down(s)"
            )

        # last_accessed mirrors creation_timestamp at save time so a freshly
        # created save shows up at the top of the "recently opened" list
        # before the user ever re-opens it.
        now_iso = datetime.now().isoformat()
        metadata = {
            'creation_timestamp': now_iso,
            'last_accessed':      now_iso,
            'source_midi_filename': self.original_filename,
            'playback_settings': self.config,
            'track_details': track_details,
            'compiled_pedal_count': compiled_pedal_count,
        }

        save_data = {'metadata': metadata, 'compiled_events': serialized_events}

        timestamp_str = datetime.now().strftime("%Y%m%d_%H%M%S")
        output_filename = f"{Path(self.original_filename).stem}_{timestamp_str}.json"
        output_path = Path(self.save_dir) / output_filename

        try:
            with open(output_path, 'w') as f:
                json.dump(save_data, f, indent=4)
            if debug_log:
                debug_log(f"[SAVE] Write successful: {output_path}")
            self.status_updated.emit(f"Serialization successful: {output_path}")
            self.save_successful.emit(str(output_path), "Playback sequence serialized and saved successfully.")
        except Exception as e:
            if debug_log:
                debug_log(f"[SAVE] Write FAILED: {e}")
            self.status_updated.emit(f"Serialization failed: {e}")
            self.save_failed.emit(f"Failed to serialize playback data to Windows file system:\n{e}")

        self.finished.emit()


class PlaybackController(QObject):
    # Signals to communicate back to the GUI
    status_updated    = Signal(str)
    progress_updated  = Signal(float)
    playback_finished = Signal()
    visualizer_updated = Signal(list)
    auto_paused       = Signal()
    error_occurred    = Signal(str)

    pedal_updated = Signal(bool)          # Bridged from Player: True=down, False=up
    # Custom signals for specific orchestration events
    timeline_data_ready = Signal(list, float, object)  # notes, total_duration, tempo_map
    pedal_data_ready    = Signal(list)                 # List of (start_sec, end_sec) pedal intervals
    save_successful     = Signal(str, str)             # filepath, success message
    save_failed         = Signal(str)                  # error message

    # Status indicator lifecycle signals
    preparation_started = Signal()   # emitted when heavy preparation begins (any play path)
    playback_started    = Signal()   # emitted just before the Player QThread starts

    # AI pedal threshold feedback: emitted after successful AI inference so the
    # UI can show the auto-computed values and allow user adjustment.
    ai_pedal_thresholds_ready = Signal(float, float)        # threshold_on, threshold_off
    ai_pedal_stats_ready      = Signal(float, float, float, float)  # avg_dur, min_dur, max_dur, presses_per_min

    def __init__(self):
        super().__init__()
        self.player         = None
        self.player_thread  = None
        self._save_worker   = None
        self._save_thread   = None
        self._prepare_worker = None
        self._prepare_thread = None
        self._pending_config: Dict | None = None

    # -- State queries --------------------------------------------------------

    def is_preparing(self) -> bool:
        """True while the _PrepareWorker thread is running (before Player starts)."""
        return self._prepare_thread is not None and self._prepare_thread.isRunning()

    def is_playing(self) -> bool:
        """True while the Player QThread is running."""
        return self.player_thread is not None and self.player_thread.isRunning()

    def is_paused(self) -> bool:
        return self.player is not None and self.player.pause_event.is_set()

    # -- Playback controls ----------------------------------------------------

    def toggle_pause(self):
        if self.player:
            self.player.toggle_pause()

    def stop(self):
        if self.player:
            self.player.stop()

    def seek(self, target_time: float):
        if self.player:
            self.player.seek(target_time)

    def shutdown(self):
        """Best-effort teardown of every worker thread on application close.

        Python threads cannot be force-killed, so each worker is asked to stop
        cooperatively (prepare worker via cancel(), player via stop()) and then
        joined with a bounded wait so a hung worker cannot block close forever.
        The save worker is included here so a close mid-save does not leak a
        thread that later emits into a destroyed window.
        """
        if self._prepare_worker is not None:
            self._prepare_worker.cancel()
        if self._prepare_thread and self._prepare_thread.isRunning():
            self._prepare_thread.quit()
            self._prepare_thread.wait(2000)
        if self.player and self.player_thread and self.player_thread.isRunning():
            self.player.stop()
            self.player_thread.wait(2000)
        if self._save_thread and self._save_thread.isRunning():
            self._save_thread.quit()
            self._save_thread.wait(2000)

    # -- Playback finished ----------------------------------------------------

    def _on_playback_finished(self):
        if self.player_thread:
            self.player_thread.quit()
            self.player_thread.wait(2000)
        self.player = None
        self.player_thread = None
        self.playback_finished.emit()

    # -- Thread wiring --------------------------------------------------------

    def _wire_and_start_player(self, player: Player, entry_point) -> None:
        """Move player onto a new QThread, wire all signals, and start playback."""
        self.player_thread = QThread()
        player.moveToThread(self.player_thread)
        self.player_thread.started.connect(entry_point)
        player.playback_finished.connect(self._on_playback_finished)
        player.status_updated.connect(self.status_updated.emit)
        player.progress_updated.connect(self.progress_updated.emit)
        player.visualizer_updated.connect(self.visualizer_updated.emit)
        player.pedal_updated.connect(self.pedal_updated.emit)
        player.auto_paused.connect(self.auto_paused.emit)
        player.error_occurred.connect(self.error_occurred.emit)
        self.playback_started.emit()
        self.player_thread.start()

    # -- Save -----------------------------------------------------------------

    def save(self, config: Dict, selected_tracks_info: List, save_dir: str, original_filename: str):
        if self._save_thread and self._save_thread.isRunning():
            return
        self._save_thread = QThread()
        self._save_worker = _SaveWorker(config, selected_tracks_info, save_dir, original_filename)
        self._save_worker.moveToThread(self._save_thread)

        self._save_thread.started.connect(self._save_worker.run)
        self._save_worker.status_updated.connect(self.status_updated)
        self._save_worker.save_successful.connect(self.save_successful)
        self._save_worker.save_failed.connect(self.save_failed)
        self._save_worker.finished.connect(self._on_save_finished)

        self._save_thread.start()

    def _on_save_finished(self):
        if self._save_thread:
            self._save_thread.quit()
            self._save_thread.wait(2000)
        self._save_worker = None
        self._save_thread = None

    # -- Play (MIDI file) -----------------------------------------------------

    def play(self, config: Dict, selected_tracks_info: List):
        """Start async preparation then playback for a MIDI file.

        _prepare_notes, SectionAnalyzer, and compile_events (which itself runs
        humanization and pedal generation) all run on a _PrepareWorker QThread
        so the main thread (and its animation timers) remain responsive during
        the blocking work.
        """
        if self.is_preparing() or self.is_playing():
            return

        self.status_updated.emit("Preparing playback...")
        self.preparation_started.emit()
        self._start_prepare_worker(config, _PrepareWorker(config, selected_tracks_info))

    def _start_prepare_worker(self, config: Dict, worker: '_PrepareWorker') -> None:
        """Move a configured _PrepareWorker onto a new QThread, wire it, start it.

        Single source of truth for prepare-thread setup; used by both the MIDI
        play path and the translator (pre-built notes) play path.
        """
        self._pending_config = config
        self._prepare_thread = QThread()
        self._prepare_worker = worker
        self._prepare_worker.moveToThread(self._prepare_thread)

        self._prepare_thread.started.connect(self._prepare_worker.run)
        self._prepare_worker.status_updated.connect(self.status_updated)
        self._prepare_worker.prepare_finished.connect(self._on_prepare_finished)
        self._prepare_worker.ai_thresholds_ready.connect(self.ai_pedal_thresholds_ready)
        self._prepare_worker.pedal_stats_ready.connect(self.ai_pedal_stats_ready)
        self._prepare_worker.error_occurred.connect(self._on_prepare_error)
        self._prepare_worker.finished.connect(self._on_prepare_cleanup)

        self._prepare_thread.start()

    def _on_prepare_finished(self, final_notes, compiled_events, tempo_map, total_dur, pedal_intervals):
        config = self._pending_config

        self.timeline_data_ready.emit(final_notes, total_dur, tempo_map)
        self.pedal_data_ready.emit(pedal_intervals)

        # Compilation already happened on the prepare thread; inject the result
        # and run the execution-only entry point. notes/sections are not needed
        # by the player in this path (rubato/humanization are baked into
        # compiled_events), so empty lists are passed.
        compiled_dur = compiled_events[-1].time if compiled_events else total_dur
        self.player = Player(config, [], [], tempo_map)
        self.player.load_compiled_events(compiled_events, compiled_dur)
        self._wire_and_start_player(self.player, self.player.play_compiled)

    def _on_prepare_error(self, error_msg: str):
        self.error_occurred.emit(error_msg)

    def _on_prepare_cleanup(self):
        if self._prepare_thread:
            self._prepare_thread.quit()
            self._prepare_thread.wait()
        self._prepare_worker = None
        self._prepare_thread = None
        self._pending_config = None

    # -- Play (translator notes) ----------------------------------------------

    def play_from_notes(self, config: Dict, notes: List[Note], tempo_map: TempoMap):
        """Start playback from pre-built Note objects, bypassing MIDI file parsing.

        Used by the Translator tab to play imported sheet text. Section
        analysis, pedal generation, and compilation run on a _PrepareWorker
        QThread (same as the MIDI path) so the GUI thread is not blocked for
        large sheets or when AI pedal is enabled.
        """
        if self.is_preparing() or self.is_playing():
            return
        self.status_updated.emit("Preparing playback from imported sheet...")
        self.preparation_started.emit()
        self._start_prepare_worker(
            config, _PrepareWorker(config, notes=notes, tempo_map=tempo_map)
        )

    # -- Play (pre-compiled save) ---------------------------------------------

    def play_from_save(self, loaded_save_data: Dict):
        self.status_updated.emit("Initializing playback from pre-compiled serialization...")
        self.preparation_started.emit()

        config = loaded_save_data.get('metadata', {}).get('playback_settings', {})
        events_data = loaded_save_data.get('compiled_events', [])

        debug_log = self.status_updated.emit if config.get('debug_mode') else None
        if debug_log:
            metadata = loaded_save_data.get('metadata', {})
            debug_log("\n" + "=" * 60)
            debug_log("=== PLAYBACK SESSION START (Saved file) ===")
            debug_log("=" * 60)
            debug_log(f"[SAVE] Source: {metadata.get('source_midi_filename', 'unknown')}")
            debug_log(f"[SAVE] Created: {metadata.get('creation_timestamp', 'unknown')}")
            debug_log(f"[SAVE] Raw events in file: {len(events_data)}")

        reconstructed_events = []
        reconstructed_notes = []
        active_presses = {}
        note_id_counter = 0

        for ev in events_data:
            pitch_val = ev.get('pitch')
            # Strictly typecast properties to prevent silent pynput failure
            if pitch_val is not None:
                pitch_val = int(pitch_val)

            reconstructed_events.append(KeyEvent(
                time=float(ev['time']),
                priority=int(ev['priority']),
                action=str(ev['action']),
                key_char=str(ev['key_char']),
                pitch=pitch_val
            ))

            # Reconstruct basic note bounds for visualizer tracking. A pitch can
            # be pressed again before its prior release (overlapping notes on the
            # same key), so keep a FIFO stack of open press times per pitch
            # rather than a single value that the second press would clobber.
            if ev['action'] == 'press' and pitch_val is not None:
                active_presses.setdefault(pitch_val, []).append(float(ev['time']))
            elif ev['action'] == 'release' and pitch_val is not None:
                if active_presses.get(pitch_val):
                    start = active_presses[pitch_val].pop(0)
                    dur = max(0.01, float(ev['time']) - start)
                    # Assign a basic hand based on pitch threshold so visualizer isn't gray
                    hand = 'left' if pitch_val < 60 else 'right'

                    reconstructed_notes.append(Note(
                        id=note_id_counter, pitch=pitch_val, velocity=64,
                        start_time=start, duration=dur, hand=hand
                    ))
                    note_id_counter += 1

        reconstructed_notes = sorted(reconstructed_notes, key=lambda n: n.start_time)

        # Enforce chronological ordering on the compiled execution events to prevent instant loop exiting
        reconstructed_events.sort(key=lambda x: (x.time, x.priority))

        total_dur = reconstructed_events[-1].time if reconstructed_events else 1.0
        dummy_tempo = TempoMap([(0, 500000)], [])

        if debug_log:
            press_ct = sum(1 for e in reconstructed_events if e.action == 'press')
            release_ct = sum(1 for e in reconstructed_events if e.action == 'release')
            pedal_ct = sum(1 for e in reconstructed_events if e.action == 'pedal')
            debug_log(
                f"[SAVE] Reconstructed: {len(reconstructed_events)} events "
                f"(press={press_ct} release={release_ct} pedal={pedal_ct}) | "
                f"{len(reconstructed_notes)} visual notes | duration={total_dur:.2f}s"
            )

        self.timeline_data_ready.emit(reconstructed_notes, total_dur, dummy_tempo)

        _pedal_evs = [ev for ev in reconstructed_events if ev.action == 'pedal']
        self.pedal_data_ready.emit(_extract_pedal_intervals(_pedal_evs))

        self.player = Player(config, [], [], dummy_tempo)
        self.player.load_compiled_events(reconstructed_events, total_dur)
        self._wire_and_start_player(self.player, self.player.play_saved_events)
