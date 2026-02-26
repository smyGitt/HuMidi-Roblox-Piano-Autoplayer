import copy
import json
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Any

from PyQt6.QtCore import QObject, QThread, pyqtSignal as Signal

from core.models import Note, KeyEvent
from core.core import MidiParser, TempoMap
from core.analysis import SectionAnalyzer, FingeringEngine
from core.player import Player

class PlaybackController(QObject):
    # Signals to communicate back to the GUI
    status_updated = Signal(str)
    progress_updated = Signal(float)
    playback_finished = Signal()
    visualizer_updated = Signal(list)
    auto_paused = Signal()
    error_occurred = Signal(str)
    
    # Custom signals for specific orchestration events
    timeline_data_ready = Signal(list, float, object) # notes, total_duration, tempo_map
    save_successful = Signal(str, str) # filepath, success message
    save_failed = Signal(str) # error message

    def __init__(self):
        super().__init__()
        self.player = None
        self.player_thread = None

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
        self.status_updated.emit("Compiling data for serialization...")
        tempo_scale = config.get('tempo', 100.0) / 100.0
        
        try:
            tracks, tempo_map = MidiParser.parse_structure(config['midi_file'], tempo_scale, None)
            selected_indices = [t.index for t, _ in selected_tracks_info]
            role_map = {t.index: r for t, r in selected_tracks_info}
            final_notes = []
            
            for track in tracks:
                if track.index in selected_indices:
                    role = role_map[track.index]
                    for note in track.notes:
                        new_note = copy.deepcopy(note)
                        if role == "Left Hand": new_note.hand = 'left'
                        elif role == "Right Hand": new_note.hand = 'right'
                        final_notes.append(new_note)
        except Exception as e:
            self.save_failed.emit(f"Error preparing save data:\n{e}")
            return

        final_notes.sort(key=lambda n: n.start_time)
        
        if config.get('simulate_hands'):
            engine = FingeringEngine()
            engine.assign_hands(final_notes)
        else:
            for note in final_notes:
                if note.hand == 'unknown':
                    note.hand = 'left' if note.pitch < 60 else 'right'

        analyzer = SectionAnalyzer(final_notes, tempo_map)
        sections = analyzer.analyze()
        
        compiler_player = Player(config, final_notes, sections, tempo_map)
        events_to_serialize = compiler_player.export_compiled_events()
        
        serialized_events = []
        for ev in events_to_serialize:
            serialized_events.append({
                'time': ev.time,
                'priority': ev.priority,
                'action': ev.action,
                'key_char': ev.key_char,
                'pitch': ev.pitch
            })
        
        metadata = {
            'creation_timestamp': datetime.now().isoformat(),
            'source_midi_filename': original_filename,
            'playback_settings': config
        }
        
        save_data = {
            'metadata': metadata,
            'compiled_events': serialized_events
        }
        
        timestamp_str = datetime.now().strftime("%Y%m%d_%H%M%S")
        output_filename = f"{Path(original_filename).stem}_{timestamp_str}.json"
        output_path = Path(save_dir) / output_filename
        
        try:
            with open(output_path, 'w') as f:
                json.dump(save_data, f, indent=4)
            self.status_updated.emit(f"Serialization successful: {output_path}")
            self.save_successful.emit(str(output_path), "Playback sequence serialized and saved successfully.")
        except Exception as e:
            self.status_updated.emit(f"Serialization failed: {e}")
            self.save_failed.emit(f"Failed to serialize playback data to Windows file system:\n{e}")

    def play(self, config: Dict, selected_tracks_info: List):
        self.status_updated.emit("Preparing playback...")
        tempo_scale = config.get('tempo', 100.0) / 100.0
        
        try:
            tracks, tempo_map = MidiParser.parse_structure(config['midi_file'], tempo_scale, None)
            selected_indices = [t.index for t, _ in selected_tracks_info]
            role_map = {t.index: r for t, r in selected_tracks_info}
            final_notes = []
            
            if config.get('debug_mode'): 
                self.status_updated.emit("\n=== RAW MIDI DATA (Selected Tracks) ===")
            for track in tracks:
                if track.index in selected_indices:
                    role = role_map[track.index]
                    if config.get('debug_mode'): 
                        self.status_updated.emit(f"Track {track.index} ({track.name}): {len(track.notes)} Notes | Role: {role}")
                    for note in track.notes:
                        new_note = copy.deepcopy(note)
                        if role == "Left Hand": new_note.hand = 'left'
                        elif role == "Right Hand": new_note.hand = 'right'
                        final_notes.append(new_note)
        except Exception as e:
            self.error_occurred.emit(f"Error preparing playback:\n{e}")
            return

        final_notes.sort(key=lambda n: n.start_time)
        
        if config.get('simulate_hands'):
            self.status_updated.emit("Simulating hands for unassigned notes...")
            engine = FingeringEngine()
            engine.assign_hands(final_notes)
        else:
            for note in final_notes:
                if note.hand == 'unknown':
                    note.hand = 'left' if note.pitch < 60 else 'right'

        self.status_updated.emit("Analyzing musical structure...")
        analyzer = SectionAnalyzer(final_notes, tempo_map)
        sections = analyzer.analyze()
        
        if config.get('debug_mode'):
            self.status_updated.emit("\n=== MUSICAL STRUCTURE ANALYSIS ===")
            for i, sec in enumerate(sections):
                self.status_updated.emit(f"SECTION {i} [{sec.start_time:.2f}s - {sec.end_time:.2f}s] {sec.articulation_label}")
                
        total_dur = max(n.end_time for n in final_notes) if final_notes else 1.0
        
        # Pass the processed timeline metrics back to the GUI
        self.timeline_data_ready.emit(final_notes, total_dur, tempo_map)

        self.player_thread = QThread()
        self.player = Player(config, final_notes, sections, tempo_map)
        self.player.moveToThread(self.player_thread)
        
        self.player_thread.started.connect(self.player.play)
        
        # Bridge Player signals through the Orchestrator
        self.player.playback_finished.connect(self._on_playback_finished)
        self.player.status_updated.connect(self.status_updated.emit)
        self.player.progress_updated.connect(self.progress_updated.emit)
        self.player.visualizer_updated.connect(self.visualizer_updated.emit)
        self.player.auto_paused.connect(self.auto_paused.emit)
        self.player.error_occurred.connect(self.error_occurred.emit)
        
        self.player_thread.start()

    def play_from_save(self, loaded_save_data: Dict):
        self.status_updated.emit("Initializing playback from pre-compiled serialization...")
        config = loaded_save_data.get('metadata', {}).get('playback_settings', {})
        events_data = loaded_save_data.get('compiled_events', [])
        
        reconstructed_events = []
        reconstructed_notes = []
        active_presses = {}
        
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
                        id=0, pitch=pitch_val, velocity=64, 
                        start_time=start, duration=dur, hand=hand
                    ))
                    
        reconstructed_notes = sorted(reconstructed_notes, key=lambda n: n.start_time)
        
        # Enforce chronological ordering on the compiled execution events to prevent instant loop exiting
        reconstructed_events.sort(key=lambda x: (x.time, x.priority))
        
        total_dur = reconstructed_events[-1].time if reconstructed_events else 1.0
        dummy_tempo = TempoMap([(0, 500000)], []) 
        
        self.timeline_data_ready.emit(reconstructed_notes, total_dur, dummy_tempo)
        
        self.player_thread = QThread()
        self.player = Player(config, [], [], dummy_tempo)
        self.player.compiled_events = reconstructed_events
        self.player.total_duration = total_dur
        
        self.player.moveToThread(self.player_thread)
        self.player_thread.started.connect(self.player.play_saved_events)
        
        self.player.playback_finished.connect(self._on_playback_finished)
        self.player.status_updated.connect(self.status_updated.emit)
        self.player.progress_updated.connect(self.progress_updated.emit)
        self.player.visualizer_updated.connect(self.visualizer_updated.emit)
        self.player.auto_paused.connect(self.auto_paused.emit)
        self.player.error_occurred.connect(self.error_occurred.emit)
        
        self.player_thread.start()