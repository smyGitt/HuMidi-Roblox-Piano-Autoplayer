"""Threaded MIDI structure parse for the track-selection flow.

`MidiParser.parse_structure` is the blocking parse used to populate the
TrackSelectionDialog when a user opens a MIDI file. Running it on the GUI thread
freezes the window (and its status-indicator animation) for the duration of the
parse. `MidiParseWorker` moves that parse onto a QThread; the owning window opens
the modal dialog from the `parsed` slot once results are in.
"""

from PyQt6.QtCore import QObject, pyqtSignal as Signal

from core.core import MidiParser


class MidiParseWorker(QObject):
    """Runs MidiParser.parse_structure on a QThread.

    Emits `parsed(tracks, tempo_map, pedal_count)` on success or `failed(str)`
    on any exception, and always `finished()` last so the owner can quit/join
    the hosting thread.
    """
    parsed   = Signal(object, object, int)  # tracks, tempo_map, pedal_count
    failed   = Signal(str)
    finished = Signal()

    def __init__(self, filepath: str):
        super().__init__()
        self.filepath = filepath

    def run(self):
        try:
            tracks, tempo_map, pedal_count = MidiParser.parse_structure(self.filepath, 1.0, None)
        except Exception as e:
            self.failed.emit(str(e))
            self.finished.emit()
            return
        self.parsed.emit(tracks, tempo_map, pedal_count)
        self.finished.emit()
