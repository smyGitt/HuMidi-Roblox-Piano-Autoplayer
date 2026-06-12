from dataclasses import dataclass, field
from typing import Any, List, Optional


@dataclass
class AppState:
    """All mutable application-level state for a HuMidi session.

    Owned by MainWindow and mutated by its file-handling, save/load, and
    playback-data slots. Grouping it here gives the state a single named
    owner and makes it clear which fields belong to session logic vs window
    behaviour vs playback routing.
    """

    loaded_save_data: Optional[dict] = None
    loaded_save_filename: Optional[str] = None
    selected_tracks_info: Optional[List] = None
    parsed_tracks: Optional[List] = None
    loaded_pedal_count: int = 0
    current_notes: List = field(default_factory=list)
    note_start_times: List = field(default_factory=list)
    total_song_duration_sec: float = 1.0
    max_note_duration: float = 0.0
    current_pedal_intervals: List = field(default_factory=list)
    parsed_tempo_map: Optional[Any] = None
