import copy
import json
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

from PySide6.QtCore import QObject, QThread, Signal

from core.models import Note, KeyEvent
from core.core import MidiParser, TempoMap
from core.section_analyzer import SectionAnalyzer, assign_hands
from core.player import Player
from core.compiler import compile_events, compile_note_events, compile_pedal_events, merge_compiled
from core.session_cache import (
    extract_notes_config, extract_pedal_config,
    notes_config_matches, pedal_config_matches,
    write_cache, tempo_map_to_dict,
)


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

    Shared by _NotesCompileWorker.run() and _SaveWorker.run() to eliminate the
    duplicated note-preparation pipeline that previously existed in both places.

    Returns (final_notes, tempo_map, midi_pedal_events). Raises on MIDI parse
    failure -- callers should catch and surface the exception appropriately.
    """
    tempo_scale = config.get('tempo', 100.0) / 100.0
    if log:
        log(f"[PREP] Parsing MIDI: tempo_scale={tempo_scale:.3f} ({config.get('tempo', 100.0):.1f}%)")
    tracks, tempo_map, pedal_cc_count, midi_pedal_events = MidiParser.parse_structure(
        config['midi_file'], tempo_scale, log
    )
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

    return final_notes, tempo_map, midi_pedal_events


# ---------------------------------------------------------------------------
# Phase-1 worker: note preparation + humanization (MIDI path)
# ---------------------------------------------------------------------------

class _NotesCompileWorker(QObject):
    """Runs phase 1 off the GUI thread: note prep + SectionAnalyzer + compile_note_events.

    Handles both the MIDI-file path (via _prepare_notes) and the translator
    pre-built-notes path. Does NOT generate pedal events -- that is phase 2.

    Emits notes_compiled with everything needed to cache the intermediate state
    and populate the timeline visualizer. Always emits finished last.
    """
    status_updated = Signal(str)
    notes_compiled = Signal(object, object, object, object, float, object)
    # (final_notes, humanized_notes, note_events, tempo_map, total_dur, midi_pedal_events)
    error_occurred = Signal(str)
    finished       = Signal()

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
            debug_log("=== NOTES COMPILE SESSION START (MIDI file) ===")
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

        midi_pedal_events = []
        try:
            if self._prebuilt_notes is not None:
                final_notes = self._prebuilt_notes
                tempo_map = self._prebuilt_tempo_map
                _apply_hand_assignment(final_notes, self.config, debug_log)
            else:
                final_notes, tempo_map, midi_pedal_events = _prepare_notes(
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

        self.status_updated.emit("Compiling note events...")
        try:
            note_events, humanized_notes = compile_note_events(
                self.config, final_notes, sections, log=debug_log
            )
        except Exception as e:
            self.error_occurred.emit(f"Error compiling note events:\n{e}")
            self.finished.emit()
            return

        if self._cancelled:
            self.finished.emit()
            return

        self.notes_compiled.emit(
            final_notes, humanized_notes, note_events, tempo_map, total_dur, midi_pedal_events
        )
        self.finished.emit()


# ---------------------------------------------------------------------------
# Phase-2 worker: pedal generation + merge (MIDI path)
# ---------------------------------------------------------------------------

class _PedalCompileWorker(QObject):
    """Runs phase 2 off the GUI thread: re-run SectionAnalyzer + compile_pedal_events + merge.

    Takes the cached intermediate state from phase 1. Does not re-parse MIDI or
    re-humanize notes; the humanized_notes list is re-used as-is so the pedal
    aligns with what will actually be played.

    auto_play -- when True, the controller starts the Player automatically after
                 emitting pedal_compiled (the compile-then-play path triggered
                 by pressing Play from the LOADED state for the first time).
                 When False (Apply button path), playback is NOT started.
    """
    status_updated    = Signal(str)
    pedal_compiled    = Signal(object, object, bool)  # (merged_events, pedal_intervals, auto_play)
    ai_thresholds_ready = Signal(float, float)
    pedal_stats_ready   = Signal(float, float, float, float)
    error_occurred    = Signal(str)
    finished          = Signal()

    def __init__(
        self,
        config: Dict,
        humanized_notes: List[Note],
        note_events: List[KeyEvent],
        final_notes: List[Note],
        tempo_map: TempoMap,
        total_dur: float,
        auto_play: bool = False,
        midi_pedal_events: Optional[List] = None,
    ):
        super().__init__()
        self.config = config
        self._humanized_notes = humanized_notes
        self._note_events = note_events
        self._final_notes = final_notes
        self._tempo_map = tempo_map
        self._total_dur = total_dur
        self._auto_play = auto_play
        self._midi_pedal_events = midi_pedal_events or []
        self._cancelled = False

    def cancel(self):
        self._cancelled = True

    def run(self):
        debug_log = self.status_updated.emit if self.config.get('debug_mode') else None

        self.status_updated.emit("Generating pedal events...")

        analyzer = SectionAnalyzer(self._final_notes, self._tempo_map, debug_log=debug_log)
        sections = analyzer.analyze()

        if self._cancelled:
            self.finished.emit()
            return

        out_meta: dict = {}
        try:
            pedal_events = compile_pedal_events(
                self.config, self._humanized_notes, sections,
                log=debug_log, out_meta=out_meta,
                midi_pedal_events=self._midi_pedal_events,
            )
        except Exception as e:
            self.error_occurred.emit(f"Error generating pedal events:\n{e}")
            self.finished.emit()
            return

        if self._cancelled:
            self.finished.emit()
            return

        merged_events = merge_compiled(self._note_events, pedal_events, log=debug_log)
        pedal_intervals = _extract_pedal_intervals(
            [ev for ev in merged_events if ev.action == 'pedal']
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
                presses_per_min = (
                    len(pedal_intervals) / (self._total_dur / 60.0)
                    if self._total_dur > 0 else 0.0
                )
                self.pedal_stats_ready.emit(avg_dur, min_dur, max_dur, presses_per_min)

        self.pedal_compiled.emit(merged_events, pedal_intervals, self._auto_play)
        self.finished.emit()


# ---------------------------------------------------------------------------
# Legacy monolithic worker: kept for the translator (play_from_notes) path
# ---------------------------------------------------------------------------

class _PrepareWorker(QObject):
    """Runs the full off-thread preparation pipeline on a QThread.

    Used only by the translator (play_from_notes) path. The MIDI play path
    uses _NotesCompileWorker + _PedalCompileWorker instead.
    """
    status_updated      = Signal(str)
    prepare_finished    = Signal(object, object, object, float, object)
    ai_thresholds_ready = Signal(float, float)
    pedal_stats_ready   = Signal(float, float, float, float)
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

        midi_pedal_events = []
        try:
            if self._prebuilt_notes is not None:
                final_notes = self._prebuilt_notes
                tempo_map = self._prebuilt_tempo_map
                _apply_hand_assignment(final_notes, self.config, debug_log)
            else:
                final_notes, tempo_map, midi_pedal_events = _prepare_notes(
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
                self.config, final_notes, sections, log=debug_log, out_meta=out_meta,
                midi_pedal_events=midi_pedal_events,
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


# ---------------------------------------------------------------------------
# Save worker (unchanged)
# ---------------------------------------------------------------------------

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
            final_notes, tempo_map, midi_pedal_events = _prepare_notes(
                self.config, self.selected_tracks_info, log=debug_log
            )
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
            midi_pedal_events=midi_pedal_events,
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


# ---------------------------------------------------------------------------
# PlaybackController
# ---------------------------------------------------------------------------

class PlaybackController(QObject):
    # Player signals (re-emitted)
    status_updated     = Signal(str)
    progress_updated   = Signal(float)
    playback_finished  = Signal()
    visualizer_updated = Signal(list)
    auto_paused        = Signal()
    error_occurred     = Signal(str)
    pedal_updated      = Signal(bool)

    # Timeline data
    timeline_data_ready = Signal(list, float, object)
    pedal_data_ready    = Signal(list)

    # Save signals
    save_successful = Signal(str, str)
    save_failed     = Signal(str)

    # Lifecycle signals
    preparation_started = Signal()
    playback_started    = Signal()

    # Phase-1 done: note events compiled and cached
    notes_phase_done = Signal()

    # Phase-2 done: pedal events compiled and cached; full session ready
    pedal_phase_done = Signal()

    # Translator-path: full monolithic compile done (sets READY like pedal_phase_done)
    session_ready = Signal()

    # AI pedal feedback
    ai_pedal_thresholds_ready = Signal(float, float)
    ai_pedal_stats_ready      = Signal(float, float, float, float)

    def __init__(self):
        super().__init__()
        # Player / prepare / save thread refs
        self.player          = None
        self.player_thread   = None
        self._save_worker    = None
        self._save_thread    = None
        self._prepare_worker = None   # translator _PrepareWorker
        self._prepare_thread = None
        self._pending_config: Optional[Dict] = None

        # Phase-1 worker refs
        self._notes_worker: Optional[_NotesCompileWorker] = None
        self._notes_thread: Optional[QThread] = None

        # Phase-2 worker refs
        self._pedal_worker: Optional[_PedalCompileWorker] = None
        self._pedal_thread: Optional[QThread] = None

        # In-memory compiled state cache (MIDI path only)
        self._cached_final_notes: Optional[List[Note]] = None
        self._cached_humanized_notes: Optional[List[Note]] = None
        self._cached_note_events: Optional[List[KeyEvent]] = None
        self._cached_pedal_events: Optional[List[KeyEvent]] = None
        self._cached_merged_events: Optional[List[KeyEvent]] = None
        self._cached_tempo_map: Optional[TempoMap] = None
        self._cached_total_dur: float = 1.0
        self._cached_midi_pedal_events: Optional[List] = None

        # Config snapshots for freshness checking
        self._notes_config_snapshot: Optional[dict] = None
        self._pedal_config_snapshot: Optional[dict] = None

        # Holds AI-computed thresholds between ai_thresholds_ready and _on_pedal_compiled
        self._pending_ai_thresholds: Optional[tuple] = None

    # -- State queries --------------------------------------------------------

    def is_compiling_notes(self) -> bool:
        return self._notes_thread is not None and self._notes_thread.isRunning()

    def is_compiling_pedal(self) -> bool:
        return self._pedal_thread is not None and self._pedal_thread.isRunning()

    def is_preparing(self) -> bool:
        """True while any compilation worker or the translator prepare thread is running."""
        return (
            self.is_compiling_notes()
            or self.is_compiling_pedal()
            or (self._prepare_thread is not None and self._prepare_thread.isRunning())
        )

    def is_playing(self) -> bool:
        return self.player_thread is not None and self.player_thread.isRunning()

    def is_paused(self) -> bool:
        return self.player is not None and self.player.pause_event.is_set()

    def has_compiled_notes(self) -> bool:
        """True when phase-1 has completed at least once for the current file."""
        return self._notes_config_snapshot is not None

    def pedal_ever_compiled(self) -> bool:
        """True when phase-2 has completed at least once for the current notes."""
        return self._pedal_config_snapshot is not None

    def notes_match_config(self, config: dict) -> bool:
        """True when the cached note events match the given config's notes keys."""
        return notes_config_matches(self._notes_config_snapshot, config)

    def pedal_match_config(self, config: dict) -> bool:
        """True when the cached pedal events match the given config's pedal keys."""
        return pedal_config_matches(self._pedal_config_snapshot, config)

    def invalidate_notes_cache(self) -> None:
        """Clear all compiled state. Called when the MIDI file or tracks change."""
        self._cached_final_notes      = None
        self._cached_humanized_notes  = None
        self._cached_note_events      = None
        self._cached_pedal_events     = None
        self._cached_merged_events    = None
        self._cached_tempo_map        = None
        self._cached_total_dur        = 1.0
        self._cached_midi_pedal_events = None
        self._notes_config_snapshot   = None
        self._pedal_config_snapshot   = None

    def get_restore_config(self) -> Optional[dict]:
        """Return a merged dict of cached snapshot values for the Discard path.

        Keys use the runtime config format (same as gather_playback_config).
        Returns None if no snapshots exist yet.
        """
        if self._notes_config_snapshot is None and self._pedal_config_snapshot is None:
            return None
        merged: dict = {}
        if self._notes_config_snapshot:
            merged.update(self._notes_config_snapshot)
        if self._pedal_config_snapshot:
            merged.update(self._pedal_config_snapshot)
        return merged

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
        """Best-effort teardown of every worker thread on application close."""
        for worker, thread in [
            (self._notes_worker, self._notes_thread),
            (self._pedal_worker, self._pedal_thread),
            (self._prepare_worker, self._prepare_thread),
        ]:
            if worker is not None:
                worker.cancel()
            if thread and thread.isRunning():
                thread.quit()
                thread.wait(2000)
        if self.player and self.player_thread and self.player_thread.isRunning():
            self.player.stop()
            self.player_thread.wait(2000)
        if self._save_thread and self._save_thread.isRunning():
            self._save_thread.quit()
            self._save_thread.wait(2000)

    # -- Player finished ------------------------------------------------------

    def _on_playback_finished(self):
        if self.player_thread:
            self.player_thread.quit()
            self.player_thread.wait(2000)
        self.player       = None
        self.player_thread = None
        self.playback_finished.emit()

    # -- Thread wiring --------------------------------------------------------

    def _wire_and_start_player(self, player: Player, entry_point) -> None:
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

    # -- Phase-1: compile notes (MIDI path) -----------------------------------

    def compile_notes(self, config: Dict, selected_tracks_info: List) -> None:
        """Start phase-1 off the GUI thread: note prep + humanization.

        Invalidates the pedal snapshot so a stale pedal is not played against
        freshly humanized notes. Emits notes_phase_done on success.
        """
        if self.is_compiling_notes() or self.is_compiling_pedal() or self.is_playing():
            return

        self._pedal_config_snapshot = None   # notes will change; pedal must follow
        self._cached_pedal_events   = None
        self._cached_merged_events  = None
        self.preparation_started.emit()
        self.status_updated.emit("Preparing playback...")

        worker = _NotesCompileWorker(config, selected_tracks_info)
        self._start_notes_worker(config, worker)

    def _start_notes_worker(self, config: Dict, worker: _NotesCompileWorker) -> None:
        self._pending_config = config
        self._notes_thread   = QThread()
        self._notes_worker   = worker
        worker.moveToThread(self._notes_thread)

        self._notes_thread.started.connect(worker.run)
        worker.status_updated.connect(self.status_updated)
        worker.notes_compiled.connect(self._on_notes_compiled)
        worker.error_occurred.connect(self._on_notes_error)
        worker.finished.connect(self._on_notes_cleanup)

        self._notes_thread.start()

    def _on_notes_compiled(
        self,
        final_notes: List[Note],
        humanized_notes: List[Note],
        note_events: List[KeyEvent],
        tempo_map: TempoMap,
        total_dur: float,
        midi_pedal_events: list,
    ) -> None:
        config = self._pending_config

        self._cached_final_notes       = final_notes
        self._cached_humanized_notes   = humanized_notes
        self._cached_note_events       = note_events
        self._cached_tempo_map         = tempo_map
        self._cached_total_dur         = total_dur
        self._cached_midi_pedal_events = midi_pedal_events
        self._notes_config_snapshot    = extract_notes_config(config)

        self.timeline_data_ready.emit(final_notes, total_dur, tempo_map)

    def _on_notes_error(self, error_msg: str) -> None:
        self.error_occurred.emit(error_msg)

    def _on_notes_cleanup(self) -> None:
        if self._notes_thread:
            self._notes_thread.quit()
            self._notes_thread.wait(2000)
        self._notes_worker   = None
        self._notes_thread   = None
        self._pending_config = None
        # Emit after the thread is fully torn down so is_compiling_notes() returns
        # False when _on_notes_phase_done attempts to chain compile_pedal.
        self.notes_phase_done.emit()

    # -- Phase-2: compile pedal (MIDI path) -----------------------------------

    def compile_pedal(self, config: Dict) -> None:
        """Start phase-2 off the GUI thread: pedal generation only.

        Requires phase-1 to have completed (has_compiled_notes() must be True).
        Does NOT start playback. Emits pedal_phase_done on success.
        """
        self._start_pedal_worker(config, auto_play=False)

    def compile_pedal_and_play(self, config: Dict) -> None:
        """Start phase-2 and automatically begin playback on success.

        Used by handle_play when in LOADED state (notes ready, pedal not yet
        compiled for the first time).
        """
        self._start_pedal_worker(config, auto_play=True)

    def _start_pedal_worker(self, config: Dict, auto_play: bool) -> None:
        if not self.has_compiled_notes():
            return
        if self.is_compiling_notes() or self.is_compiling_pedal() or self.is_playing():
            return

        self.preparation_started.emit()

        worker = _PedalCompileWorker(
            config,
            self._cached_humanized_notes,
            self._cached_note_events,
            self._cached_final_notes,
            self._cached_tempo_map,
            self._cached_total_dur,
            auto_play=auto_play,
            midi_pedal_events=self._cached_midi_pedal_events,
        )
        self._pending_config  = config
        self._pedal_thread    = QThread()
        self._pedal_worker    = worker
        worker.moveToThread(self._pedal_thread)

        self._pedal_thread.started.connect(worker.run)
        worker.status_updated.connect(self.status_updated)
        worker.pedal_compiled.connect(self._on_pedal_compiled)
        worker.ai_thresholds_ready.connect(self._on_ai_thresholds_ready)
        worker.pedal_stats_ready.connect(self.ai_pedal_stats_ready)
        worker.error_occurred.connect(self._on_pedal_error)
        worker.finished.connect(self._on_pedal_cleanup)

        self._pedal_thread.start()

    def _on_ai_thresholds_ready(self, on: float, off: float) -> None:
        self._pending_ai_thresholds = (on, off)
        self.ai_pedal_thresholds_ready.emit(on, off)

    def _on_pedal_compiled(
        self,
        merged_events: List[KeyEvent],
        pedal_intervals: list,
        auto_play: bool,
    ) -> None:
        config = self._pending_config

        pedal_only = [ev for ev in merged_events if ev.action == 'pedal']
        self._cached_pedal_events  = pedal_only
        self._cached_merged_events = merged_events
        self._pedal_config_snapshot = extract_pedal_config(config)
        if self._pending_ai_thresholds is not None:
            on, off = self._pending_ai_thresholds
            self._pedal_config_snapshot['pedal_threshold_on'] = on
            self._pedal_config_snapshot['pedal_threshold_off'] = off
            self._pending_ai_thresholds = None

        # Write session cache asynchronously.
        write_cache(
            notes_config=self._notes_config_snapshot,
            pedal_config=self._pedal_config_snapshot,
            note_events=self._cached_note_events,
            pedal_events=self._cached_pedal_events,
            humanized_notes=self._cached_humanized_notes,
            final_notes=self._cached_final_notes,
            tempo_map_data=tempo_map_to_dict(self._cached_tempo_map),
            total_dur=self._cached_total_dur,
        )

        self.pedal_data_ready.emit(pedal_intervals)
        self.pedal_phase_done.emit()

        if auto_play:
            self.start_playback()

    def _on_pedal_error(self, error_msg: str) -> None:
        self.error_occurred.emit(error_msg)

    def _on_pedal_cleanup(self) -> None:
        if self._pedal_thread:
            self._pedal_thread.quit()
            self._pedal_thread.wait(2000)
        self._pedal_worker   = None
        self._pedal_thread   = None
        self._pending_config = None

    # -- Start playback from cached state -------------------------------------

    def start_playback(self, config: Optional[dict] = None) -> None:
        """Start the Player with the current cached compiled events.

        Both phases must have completed (merged_events must exist). Does not
        re-compile anything. Emits playback_started just before the thread
        starts (via _wire_and_start_player).

        config -- if supplied, passed to Player for countdown/debug/auto_pause
                  settings. Falls back to _pending_config, then empty dict.
        """
        if not self._cached_merged_events or not self._cached_tempo_map:
            return
        used_config = config if config is not None else (self._pending_config or {})
        compiled_dur = self._cached_merged_events[-1].time if self._cached_merged_events else self._cached_total_dur
        try:
            self.player = Player(used_config, [], [], self._cached_tempo_map)
        except PermissionError as exc:
            self.error_occurred.emit(str(exc))
            return
        self.player.load_compiled_events(self._cached_merged_events, compiled_dur)
        self._wire_and_start_player(self.player, self.player.play_compiled)

    # -- Play (translator notes -- monolithic path) ---------------------------

    def play_from_notes(self, config: Dict, notes: List[Note], tempo_map: TempoMap):
        """Start playback from pre-built Note objects (Translator tab).

        Uses the legacy _PrepareWorker for the full compile-then-play cycle.
        """
        if self.is_preparing() or self.is_playing():
            return
        self.status_updated.emit("Preparing playback from imported sheet...")
        self.preparation_started.emit()
        self._pending_config = config
        worker = _PrepareWorker(config, notes=notes, tempo_map=tempo_map)
        self._start_prepare_worker(config, worker)

    def _start_prepare_worker(self, config: Dict, worker: '_PrepareWorker') -> None:
        self._pending_config  = config
        self._prepare_thread  = QThread()
        self._prepare_worker  = worker
        worker.moveToThread(self._prepare_thread)

        self._prepare_thread.started.connect(worker.run)
        worker.status_updated.connect(self.status_updated)
        worker.prepare_finished.connect(self._on_prepare_finished)
        worker.ai_thresholds_ready.connect(self.ai_pedal_thresholds_ready)
        worker.pedal_stats_ready.connect(self.ai_pedal_stats_ready)
        worker.error_occurred.connect(self._on_prepare_error)
        worker.finished.connect(self._on_prepare_cleanup)

        self._prepare_thread.start()

    def _on_prepare_finished(self, final_notes, compiled_events, tempo_map, total_dur, pedal_intervals):
        config = self._pending_config

        self.timeline_data_ready.emit(final_notes, total_dur, tempo_map)
        self.pedal_data_ready.emit(pedal_intervals)
        self.session_ready.emit()

        compiled_dur = compiled_events[-1].time if compiled_events else total_dur
        try:
            self.player = Player(config, [], [], tempo_map)
        except PermissionError as exc:
            self.error_occurred.emit(str(exc))
            return
        self.player.load_compiled_events(compiled_events, compiled_dur)
        self._wire_and_start_player(self.player, self.player.play_compiled)

    def _on_prepare_error(self, error_msg: str):
        self.error_occurred.emit(error_msg)

    def _on_prepare_cleanup(self):
        if self._prepare_thread:
            self._prepare_thread.quit()
            self._prepare_thread.wait(2000)
        self._prepare_worker = None
        self._prepare_thread = None
        self._pending_config = None

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
        reconstructed_notes  = []
        active_presses = {}
        note_id_counter = 0

        for ev in events_data:
            pitch_val = ev.get('pitch')
            if pitch_val is not None:
                pitch_val = int(pitch_val)

            reconstructed_events.append(KeyEvent(
                time=float(ev['time']),
                priority=int(ev['priority']),
                action=str(ev['action']),
                key_char=str(ev['key_char']),
                pitch=pitch_val
            ))

            if ev['action'] == 'press' and pitch_val is not None:
                active_presses.setdefault(pitch_val, []).append(float(ev['time']))
            elif ev['action'] == 'release' and pitch_val is not None:
                if active_presses.get(pitch_val):
                    start = active_presses[pitch_val].pop(0)
                    dur = max(0.01, float(ev['time']) - start)
                    hand = 'left' if pitch_val < 60 else 'right'
                    reconstructed_notes.append(Note(
                        id=note_id_counter, pitch=pitch_val, velocity=64,
                        start_time=start, duration=dur, hand=hand
                    ))
                    note_id_counter += 1

        reconstructed_notes = sorted(reconstructed_notes, key=lambda n: n.start_time)
        reconstructed_events.sort(key=lambda x: (x.time, x.priority))

        total_dur = reconstructed_events[-1].time if reconstructed_events else 1.0
        dummy_tempo = TempoMap([(0, 500000)], [])

        if debug_log:
            press_ct  = sum(1 for e in reconstructed_events if e.action == 'press')
            release_ct = sum(1 for e in reconstructed_events if e.action == 'release')
            pedal_ct  = sum(1 for e in reconstructed_events if e.action == 'pedal')
            debug_log(
                f"[SAVE] Reconstructed: {len(reconstructed_events)} events "
                f"(press={press_ct} release={release_ct} pedal={pedal_ct}) | "
                f"{len(reconstructed_notes)} visual notes | duration={total_dur:.2f}s"
            )

        self.timeline_data_ready.emit(reconstructed_notes, total_dur, dummy_tempo)

        _pedal_evs = [ev for ev in reconstructed_events if ev.action == 'pedal']
        self.pedal_data_ready.emit(_extract_pedal_intervals(_pedal_evs))

        try:
            self.player = Player(config, [], [], dummy_tempo)
        except PermissionError as exc:
            self.error_occurred.emit(str(exc))
            return
        self.player.load_compiled_events(reconstructed_events, total_dur)
        self._wire_and_start_player(self.player, self.player.play_saved_events)
