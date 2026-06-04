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
import core.pedal_generator as pedal_generator


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


def _prepare_notes(config: Dict, selected_tracks_info: List, log=None):
    """Parse MIDI, apply track role assignments, and run hand simulation.

    Shared by PlaybackController.play() and _SaveWorker.run() to eliminate the
    duplicated note-preparation pipeline that previously existed in both places.

    Returns (final_notes, tempo_map). Raises on MIDI parse failure — callers
    should catch and surface the exception appropriately.
    """
    tempo_scale = config.get('tempo', 100.0) / 100.0
    tracks, tempo_map, _ = MidiParser.parse_structure(config['midi_file'], tempo_scale, None)
    selected_indices = [t.index for t, _ in selected_tracks_info]
    role_map = {t.index: r for t, r in selected_tracks_info}
    final_notes = []

    for track in tracks:
        if track.index in selected_indices:
            role = role_map[track.index]
            if log:
                log(f"Track {track.index} ({track.name}): {len(track.notes)} Notes | Role: {role}")
            for note in track.notes:
                new_note = copy.deepcopy(note)
                if role == "Left Hand": new_note.hand = 'left'
                elif role == "Right Hand": new_note.hand = 'right'
                final_notes.append(new_note)

    final_notes.sort(key=lambda n: n.start_time)

    if config.get('simulate_hands'):
        if log: log("Simulating hands for unassigned notes...")
        assign_hands(final_notes)
    else:
        for note in final_notes:
            if note.hand == 'unknown':
                note.hand = 'left' if note.pitch < 60 else 'right'

    return final_notes, tempo_map


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

        try:
            final_notes, tempo_map = _prepare_notes(self.config, self.selected_tracks_info)
        except Exception as e:
            self.save_failed.emit(f"Error preparing save data:\n{e}")
            self.finished.emit()
            return

        analyzer = SectionAnalyzer(final_notes, tempo_map)
        sections = analyzer.analyze()

        compiler_player = Player(self.config, final_notes, sections, tempo_map)
        compiler_player.status_updated.connect(self.status_updated)
        events_to_serialize = compiler_player.export_compiled_events()

        if not events_to_serialize:
            self.save_failed.emit(
                "Compilation produced zero events — nothing to save.\n"
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

        compiled_pedal_count = sum(
            1 for ev in serialized_events
            if ev['action'] == 'pedal' and ev['key_char'] == 'down'
        )

        debug = self.config.get('debug_mode')
        if debug:
            self.status_updated.emit(
                f"[DEBUG] Serializing {len(track_details)} track(s), "
                f"{compiled_pedal_count} compiled pedal event(s)."
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
            self.status_updated.emit(f"Serialization successful: {output_path}")
            self.save_successful.emit(str(output_path), "Playback sequence serialized and saved successfully.")
        except Exception as e:
            self.status_updated.emit(f"Serialization failed: {e}")
            self.save_failed.emit(f"Failed to serialize playback data to Windows file system:\n{e}")

        self.finished.emit()


class PlaybackController(QObject):
    # Signals to communicate back to the GUI
    status_updated = Signal(str)
    progress_updated = Signal(float)
    playback_finished = Signal()
    visualizer_updated = Signal(list)
    auto_paused = Signal()
    error_occurred = Signal(str)
    
    pedal_updated = Signal(bool)          # Bridged from Player: True=down, False=up
    # Custom signals for specific orchestration events
    timeline_data_ready = Signal(list, float, object) # notes, total_duration, tempo_map
    pedal_data_ready = Signal(list)       # List of (start_sec, end_sec) pedal intervals
    save_successful = Signal(str, str) # filepath, success message
    save_failed = Signal(str) # error message

    def __init__(self):
        super().__init__()
        self.player = None
        self.player_thread = None
        self._save_worker = None
        self._save_thread = None

    def is_playing(self) -> bool:
        return self.player_thread is not None and self.player_thread.isRunning()

    def is_paused(self) -> bool:
        return self.player is not None and self.player.pause_event.is_set()

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
        if self.player and self.player_thread and self.player_thread.isRunning():
            self.player.stop()
            self.player_thread.wait(1000)

    def _on_playback_finished(self):
        if self.player_thread:
            self.player_thread.quit()
            self.player_thread.wait()
        self.player = None
        self.player_thread = None
        self.playback_finished.emit()

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
            self._save_thread.wait()
        self._save_worker = None
        self._save_thread = None

    def play(self, config: Dict, selected_tracks_info: List):
        self.status_updated.emit("Preparing playback...")

        debug_log = self.status_updated.emit if config.get('debug_mode') else None
        if debug_log:
            debug_log("\n" + "=" * 60)
            debug_log("=== PLAYBACK SESSION START (MIDI file) ===")
            debug_log("=" * 60)
            debug_log("[CONFIG] " + " | ".join(
                f"{k}={v}" for k, v in sorted(config.items())
                if k not in ('midi_file',)
            ))
            debug_log(f"[CONFIG] midi_file: {config.get('midi_file', 'N/A')}")
            debug_log(f"[CONFIG] Tracks selected: {len(selected_tracks_info)}")
            for t, role in selected_tracks_info:
                debug_log(f"  Track {t.index} ({t.name}): {t.note_count} notes | Role: {role} | Instrument: {t.instrument_name}")
            debug_log("\n=== RAW MIDI DATA (Selected Tracks) ===")

        try:
            final_notes, tempo_map = _prepare_notes(config, selected_tracks_info, log=debug_log)
        except Exception as e:
            self.error_occurred.emit(f"Error preparing playback:\n{e}")
            return

        self.status_updated.emit("Analyzing musical structure...")
        analyzer = SectionAnalyzer(final_notes, tempo_map, debug_log=debug_log)
        sections = analyzer.analyze()

        total_dur = max(n.end_time for n in final_notes) if final_notes else 1.0

        # Pass the processed timeline metrics back to the GUI
        self.timeline_data_ready.emit(final_notes, total_dur, tempo_map)

        try:
            _pedal_evs = pedal_generator.generate_events(config, final_notes, sections)
            self.pedal_data_ready.emit(_extract_pedal_intervals(_pedal_evs))
        except Exception:
            self.pedal_data_ready.emit([])

        self.player_thread = QThread()
        self.player = Player(config, final_notes, sections, tempo_map)
        self.player.moveToThread(self.player_thread)

        self.player_thread.started.connect(self.player.play)

        # Bridge Player signals through the Orchestrator
        self.player.playback_finished.connect(self._on_playback_finished)
        self.player.status_updated.connect(self.status_updated.emit)
        self.player.progress_updated.connect(self.progress_updated.emit)
        self.player.visualizer_updated.connect(self.visualizer_updated.emit)
        self.player.pedal_updated.connect(self.pedal_updated.emit)
        self.player.auto_paused.connect(self.auto_paused.emit)
        self.player.error_occurred.connect(self.error_occurred.emit)

        self.player_thread.start()

    def play_from_notes(self, config: Dict, notes: List[Note], tempo_map: TempoMap):
        """Start playback from pre-built Note objects, bypassing MIDI file parsing.

        Used by the Translator tab to play imported sheet text directly through
        the normal humanization and playback pipeline.
        """
        self.status_updated.emit("Preparing playback from imported sheet...")
        debug_log = self.status_updated.emit if config.get('debug_mode') else None

        if config.get('simulate_hands'):
            assign_hands(notes)
        else:
            for note in notes:
                if note.hand == 'unknown':
                    note.hand = 'left' if note.pitch < 60 else 'right'

        analyzer = SectionAnalyzer(notes, tempo_map, debug_log=debug_log)
        sections = analyzer.analyze()

        total_dur = max(n.end_time for n in notes) if notes else 1.0
        self.timeline_data_ready.emit(notes, total_dur, tempo_map)

        try:
            _pedal_evs = pedal_generator.generate_events(config, notes, sections)
            self.pedal_data_ready.emit(_extract_pedal_intervals(_pedal_evs))
        except Exception:
            self.pedal_data_ready.emit([])

        self.player_thread = QThread()
        self.player = Player(config, notes, sections, tempo_map)
        self.player.moveToThread(self.player_thread)
        self.player_thread.started.connect(self.player.play)

        self.player.playback_finished.connect(self._on_playback_finished)
        self.player.status_updated.connect(self.status_updated.emit)
        self.player.progress_updated.connect(self.progress_updated.emit)
        self.player.visualizer_updated.connect(self.visualizer_updated.emit)
        self.player.pedal_updated.connect(self.pedal_updated.emit)
        self.player.auto_paused.connect(self.auto_paused.emit)
        self.player.error_occurred.connect(self.error_occurred.emit)

        self.player_thread.start()

    def play_from_save(self, loaded_save_data: Dict):
        self.status_updated.emit("Initializing playback from pre-compiled serialization...")
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
            
            # Reconstruct basic note bounds for visualizer tracking.
            if ev['action'] == 'press' and pitch_val is not None:
                active_presses[pitch_val] = float(ev['time'])
            elif ev['action'] == 'release' and pitch_val is not None:
                if pitch_val in active_presses:
                    start = active_presses.pop(pitch_val)
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

        self.player_thread = QThread()
        self.player = Player(config, [], [], dummy_tempo)
        self.player.load_compiled_events(reconstructed_events, total_dur)

        self.player.moveToThread(self.player_thread)
        self.player_thread.started.connect(self.player.play_saved_events)

        self.player.playback_finished.connect(self._on_playback_finished)
        self.player.status_updated.connect(self.status_updated.emit)
        self.player.progress_updated.connect(self.progress_updated.emit)
        self.player.visualizer_updated.connect(self.visualizer_updated.emit)
        self.player.pedal_updated.connect(self.pedal_updated.emit)
        self.player.auto_paused.connect(self.auto_paused.emit)
        self.player.error_occurred.connect(self.error_occurred.emit)

        self.player_thread.start()