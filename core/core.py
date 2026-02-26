import mido
import bisect
from collections import defaultdict
from typing import List, Tuple, Dict, Optional
from core.models import Note, MidiTrack
from pynput.keyboard import Key

def get_time_groups(notes: List[Note], threshold: float = 0.015) -> List[List[Note]]:
    if not notes: return []
    groups, current_group = [], [notes[0]]
    for i in range(1, len(notes)):
        if notes[i].start_time - current_group[0].start_time <= threshold: current_group.append(notes[i])
        else:
            groups.append(current_group)
            current_group = [notes[i]]
    groups.append(current_group)
    return groups

class TempoMap:
    def __init__(self, tempo_events: List[Tuple[float, int]], time_signatures: List[Tuple[float, int, int]]):
        self.events = sorted(tempo_events, key=lambda x: x[0])
        self.time_signatures = sorted(time_signatures, key=lambda x: x[0])
        self.beat_map = [] 
        self._build_beat_map()
        self.has_explicit_time_signatures = len(time_signatures) > 0 and not (len(time_signatures) == 1 and time_signatures[0][0] == 0 and time_signatures[0][1] == 4)

    def _build_beat_map(self):
        current_beat = 0.0
        last_time = 0.0
        current_tempo = 500000 
        
        if not self.events or self.events[0][0] > 0:
             self.beat_map.append((0.0, 0.0, current_tempo))

        for time_sec, new_tempo in self.events:
            dt = time_sec - last_time
            sec_per_beat = current_tempo / 1_000_000.0
            delta_beats = dt / sec_per_beat
            current_beat += delta_beats
            
            self.beat_map.append((time_sec, current_beat, new_tempo))
            last_time = time_sec
            current_tempo = new_tempo

    def time_to_beat(self, t: float) -> float:
        idx = bisect.bisect_right([e[0] for e in self.beat_map], t) - 1
        if idx < 0: return 0.0
        
        start_time, start_beat, tempo = self.beat_map[idx]
        dt = t - start_time
        sec_per_beat = tempo / 1_000_000.0
        return start_beat + (dt / sec_per_beat)
        
    def beat_to_time(self, b: float) -> float:
        idx = bisect.bisect_right([e[1] for e in self.beat_map], b) - 1
        if idx < 0: return 0.0
        
        start_time, start_beat, tempo = self.beat_map[idx]
        dt_beats = b - start_beat
        sec_per_beat = tempo / 1_000_000.0
        return start_time + (dt_beats * sec_per_beat)

    def get_tempo_at(self, time: float) -> int:
        idx = bisect.bisect_right([e[0] for e in self.events], time) - 1
        if idx < 0: return 500000
        return self.events[idx][1]

    def get_measure_boundaries(self, total_duration: float) -> List[Tuple[float, float]]:
        measures = []
        ts_events = self.time_signatures if self.time_signatures else [(0.0, 4, 4)]
        total_beats = self.time_to_beat(total_duration)
        measure_start_beat = 0.0
        
        while measure_start_beat < total_beats:
            measure_start_time = self.beat_to_time(measure_start_beat)
            active_ts = ts_events[0]
            for ts in ts_events:
                if ts[0] <= measure_start_time + 0.001:
                    active_ts = ts
                else:
                    break
            
            current_numerator = active_ts[1]
            beat_len_factor = 4.0 / active_ts[2]
            measure_len_beats = current_numerator * beat_len_factor
            
            measure_end_beat = measure_start_beat + measure_len_beats
            measure_end_time = self.beat_to_time(measure_end_beat)
            
            measures.append((measure_start_time, measure_end_time))
            measure_start_beat = measure_end_beat
        return measures

class GlobalTickMap:
    def __init__(self, midi_file: mido.MidiFile):
        self.tick_map = [] 
        self.time_signatures = [] 
        self.ticks_per_beat = midi_file.ticks_per_beat or 480
        merged = mido.merge_tracks(midi_file.tracks)
        current_time = 0.0
        current_tick = 0
        current_tempo = 500000
        self.tick_map.append((0, 0.0, current_tempo))
        accumulated_ticks = 0
        
        for msg in merged:
            accumulated_ticks += msg.time
            delta_ticks = accumulated_ticks - current_tick
            delta_sec = mido.tick2second(delta_ticks, self.ticks_per_beat, current_tempo)
            current_time += delta_sec
            current_tick = accumulated_ticks
            
            if msg.type == 'set_tempo':
                current_tempo = msg.tempo
                self.tick_map.append((current_tick, current_time, current_tempo))
            elif msg.type == 'time_signature':
                self.time_signatures.append((current_time, msg.numerator, msg.denominator))

    def tick_to_time(self, tick: int) -> float:
        last_tick, last_time, tempo = self.tick_map[0]
        for t_tick, t_time, t_tempo in self.tick_map:
            if tick >= t_tick: last_tick, last_time, tempo = t_tick, t_time, t_tempo
            else: break
        delta_ticks = tick - last_tick
        return last_time + mido.tick2second(delta_ticks, self.ticks_per_beat, tempo)

class MidiParser:
    @staticmethod
    def parse_structure(filepath: str, tempo_scale: float = 1.0, debug_log: Optional[List[str]] = None) -> Tuple[List[MidiTrack], TempoMap]:
        try:
            mid = mido.MidiFile(filepath)
        except Exception as e:
            raise IOError(f"Could not read MIDI file: {e}")
            
        global_map = GlobalTickMap(mid)
        tempo_map_data = [(entry[1], entry[2]) for entry in global_map.tick_map]
        tempo_map = TempoMap(tempo_map_data, global_map.time_signatures)
        tracks = []
        note_id_counter = 0
        
        for i, track in enumerate(mid.tracks):
            track_name = f"Track {i}"
            program_change = 0
            is_drum = False
            notes: List[Note] = []
            open_notes: Dict[int, List[Dict]] = defaultdict(list)
            current_abs_tick = 0
            
            for msg in track:
                current_abs_tick += msg.time
                if msg.type == 'track_name': track_name = msg.name
                if msg.type == 'program_change':
                    program_change = msg.program
                    if msg.channel == 9: is_drum = True
                
                if msg.type == 'note_on' and msg.velocity > 0:
                    open_notes[msg.note].append({'start_tick': current_abs_tick, 'vel': msg.velocity})
                elif msg.type == 'note_off' or (msg.type == 'note_on' and msg.velocity == 0):
                    if open_notes[msg.note]:
                        note_data = open_notes[msg.note].pop(0)
                        start_sec = global_map.tick_to_time(note_data['start_tick'])
                        end_sec = global_map.tick_to_time(current_abs_tick)
                        duration = end_sec - start_sec
                        if duration > 0.01:
                            scaled_start = start_sec / tempo_scale
                            scaled_duration = duration / tempo_scale
                            notes.append(Note(note_id_counter, msg.note, note_data['vel'], scaled_start, scaled_duration, 'unknown', i, msg.channel))
                            note_id_counter += 1
            if any(n.channel == 9 for n in notes): is_drum = True
            if notes:
                notes.sort(key=lambda n: n.start_time)
                tracks.append(MidiTrack(i, track_name, program_change, is_drum, notes))
        return tracks, tempo_map

class KeyMapper:
    SYMBOL_MAP = {'!': '1', '@': '2', '#': '3', '$': '4', '%': '5', '^': '6', '&': '7', '*': '8', '(': '9', ')': '0'}
    LEFT_CTRL_KEYS = "1234567890qwert" 
    MIDDLE_WHITE_KEYS = "1234567890qwertyuiopasdfghjklzxcvbnm" 
    RIGHT_CTRL_KEYS = "yuiopasdfghj"
    PITCH_START_LEFT = 21 
    PITCH_START_MIDDLE = 36 
    PITCH_START_RIGHT = 97 
    
    def __init__(self, use_88_key_layout: bool = False):
        self.use_88_key_layout = use_88_key_layout
        self.key_map = {}
        if self.use_88_key_layout:
            self.min_pitch = 21; self.max_pitch = 108 
        else:
            self.min_pitch = 36; self.max_pitch = 96  
        self.init_key_map()

    def init_key_map(self):
        if self.use_88_key_layout:
            current_pitch = self.PITCH_START_LEFT
            for char in self.LEFT_CTRL_KEYS:
                self.key_map[current_pitch] = {'key': char, 'modifiers': [Key.ctrl]}
                current_pitch += 1
            current_pitch = self.PITCH_START_RIGHT
            for char in self.RIGHT_CTRL_KEYS:
                self.key_map[current_pitch] = {'key': char, 'modifiers': [Key.ctrl]}
                current_pitch += 1

        white_key_index = 0
        current_pitch = self.PITCH_START_MIDDLE
        while current_pitch <= 108 and white_key_index < len(self.MIDDLE_WHITE_KEYS):
            base_char = self.MIDDLE_WHITE_KEYS[white_key_index]
            if current_pitch not in self.key_map:
                self.key_map[current_pitch] = {'key': base_char, 'modifiers': []}
            next_pitch = current_pitch + 1
            if self.is_black_key(next_pitch):
                if next_pitch not in self.key_map:
                    self.key_map[next_pitch] = {'key': base_char, 'modifiers': [Key.shift]}
                current_pitch += 2
            else:
                current_pitch += 1
            white_key_index += 1

    def get_key_data(self, pitch: int) -> Optional[Dict]:
        if pitch < self.min_pitch:
            while pitch < self.min_pitch: pitch += 12
        elif pitch > self.max_pitch:
            while pitch > self.max_pitch: pitch -= 12
        return self.key_map.get(pitch)

    def get_key_for_pitch(self, pitch: int) -> Optional[str]:
        data = self.get_key_data(pitch)
        return data['key'] if data else None

    @staticmethod
    def is_black_key(pitch: int) -> bool:
        return (pitch % 12) in {1, 3, 6, 8, 10}
    
    @staticmethod
    def pitch_to_name(pitch: int) -> str:
        names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
        return f"{names[pitch % 12]}{(pitch // 12) - 1}"
    
    @property
    def lower_ctrl_bound(self): return 0 
    @property
    def upper_ctrl_bound(self): return 128