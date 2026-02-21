"""Data models for MIDI parsing and playback (notes, tracks, key events, sections, key state)."""

from dataclasses import dataclass, field
from typing import List, Optional, Tuple


@dataclass
class Note:
    """Single note: pitch (MIDI 0–127), start/duration in seconds, optional hand assignment."""
    id: int
    pitch: int
    velocity: int
    start_time: float
    duration: float
    hand: str = 'unknown'
    original_track_index: int = -1
    channel: int = -1

    @property
    def end_time(self) -> float:
        return self.start_time + self.duration

@dataclass
class MidiTrack:
    """Single MIDI track metadata plus list of notes; instrument_name uses GM program ranges."""
    index: int
    name: str
    program_change: int
    is_drum: bool
    notes: List[Note]
    pedal_events: List[Tuple[float, int]] = field(default_factory=list)

    @property
    def note_count(self) -> int:
        return len(self.notes)
    
    @property
    def instrument_name(self) -> str:
        if self.is_drum: return "Drums/Percussion"
        if 0 <= self.program_change <= 7: return "Piano"
        if 8 <= self.program_change <= 15: return "Chromatic Perc"
        if 16 <= self.program_change <= 23: return "Organ"
        if 24 <= self.program_change <= 31: return "Guitar"
        if 32 <= self.program_change <= 39: return "Bass"
        if 40 <= self.program_change <= 47: return "Strings"
        if 48 <= self.program_change <= 55: return "Ensemble"
        return f"Instrument {self.program_change}"

@dataclass(order=True)
class KeyEvent:
    """Event at a given time: key press/release or pedal; priority used to order same-frame events."""
    time: float
    priority: int = field(compare=True)
    action: str = field(compare=False)
    key_char: str = field(compare=False)
    pitch: Optional[int] = field(default=None, compare=False)
    velocity: int = field(default=100, compare=False)

@dataclass
class MusicalSection:
    """Time span of notes with articulation and pace labels for humanization/rubato."""
    start_time: float
    end_time: float
    notes: List[Note]
    articulation_label: str = 'unknown'
    pace_label: str = 'normal'
    start_beat: float = 0.0
    end_beat: float = 0.0

@dataclass
class KeyState:
    """Tracks whether a key is currently down or sustained by pedal."""
    key_char: str
    is_active: bool = False
    is_sustained: bool = False
    
    def press(self):
        self.is_active = True

    def release(self):
        self.is_active = False
        self.is_sustained = False 

    @property
    def is_physically_down(self) -> bool:
        return self.is_active

@dataclass
class Finger:
    """Finger slot for fingering engine (id, hand, current pitch, last press time)."""
    id: int
    hand: str
    current_pitch: Optional[int] = None
    last_press_time: float = -1.0