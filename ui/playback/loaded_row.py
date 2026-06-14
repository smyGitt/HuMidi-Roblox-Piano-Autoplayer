from PyQt6.QtWidgets import QWidget, QHBoxLayout, QLabel, QPushButton, QScrollArea, QFrame
from PyQt6.QtCore import Qt, QSize

from core.core import KeyMapper
from ui.playback.part_card import PartCard


class _HScrollArea(QScrollArea):
    """Horizontal-only scroll area that reports the inner widget's preferred height."""

    def sizeHint(self) -> QSize:
        hint = super().sizeHint()
        if self.widget():
            hint.setHeight(self.widget().sizeHint().height())
        return hint

    def minimumSizeHint(self) -> QSize:
        hint = super().minimumSizeHint()
        if self.widget():
            hint.setHeight(self.widget().minimumSizeHint().height())
        return hint


class LoadedRow(QWidget):
    """Horizontal content row for the LOADED card on PlaybackTab's File sub-tab.

    Owns the cards layout, the placeholder label, and the Edit Selection button.
    Call update_loaded_summary / update_loaded_summary_from_save / clear_loaded_summary
    to mutate the display; PlaybackTab accesses no child widget internals directly.
    """

    _HAND_LABEL = {
        "Auto-Detect": "Auto",
        "Left Hand":   "Left",
        "Right Hand":  "Right",
    }
    _NOTE_GLYPH = "♩"

    def __init__(self, parent=None):
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        outer = QHBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(8)

        self._loaded_cards_layout = QHBoxLayout()
        self._loaded_cards_layout.setContentsMargins(0, 0, 0, 0)
        self._loaded_cards_layout.setSpacing(8)

        self._tracks_placeholder = QLabel("No file loaded.")
        self._tracks_placeholder.setProperty("role", "muted")
        self._loaded_cards_layout.addWidget(self._tracks_placeholder)

        cards_container = QWidget()
        cards_container.setLayout(self._loaded_cards_layout)

        scroll = _HScrollArea()
        scroll.setWidget(cards_container)
        scroll.setWidgetResizable(True)
        scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAsNeeded)
        scroll.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        scroll.setFrameShape(QFrame.Shape.NoFrame)

        outer.addWidget(scroll, 1)

        self.edit_selection_btn = QPushButton("Edit Selection")
        self.edit_selection_btn.setEnabled(False)
        self.edit_selection_btn.setToolTip(
            "Reopen the track selection dialog to change which MIDI parts play"
        )
        outer.addWidget(self.edit_selection_btn, 0, Qt.AlignmentFlag.AlignRight)

    @staticmethod
    def _format_pitch(pitch: int) -> str:
        return KeyMapper.pitch_to_name(pitch).replace("#", "♯")

    def _clear_loaded_cards(self) -> None:
        while self._loaded_cards_layout.count():
            item = self._loaded_cards_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

    def update_loaded_summary(self, parts: list, pedal_count: int) -> None:
        """Refresh the LOADED row from live (MidiTrack, role) pairs."""
        self._clear_loaded_cards()
        if not parts:
            self._tracks_placeholder = QLabel("No tracks selected.")
            self._tracks_placeholder.setProperty("role", "muted")
            self._loaded_cards_layout.addWidget(self._tracks_placeholder)
            self.edit_selection_btn.setEnabled(True)
            return

        empty_index = 0
        for track, role in parts:
            title = (track.name or "").strip()
            if not title:
                empty_index += 1
                title = f"Track {empty_index}"
            pitches = [n.pitch for n in track.notes]
            if pitches:
                lo, hi = min(pitches), max(pitches)
                range_str = f"{self._format_pitch(lo)}-{self._format_pitch(hi)}"
            else:
                range_str = "no notes"
            hand_label = self._HAND_LABEL.get(role, role)
            meta = f"{track.note_count} {self._NOTE_GLYPH}   {hand_label}   {range_str}"
            self._loaded_cards_layout.addWidget(PartCard(title, meta))

        self._loaded_cards_layout.addWidget(
            PartCard("Pedal", f"{pedal_count} events")
        )
        self._loaded_cards_layout.addStretch()
        self.edit_selection_btn.setEnabled(True)

    def update_loaded_summary_from_save(self, track_details: list, pedal_count: int) -> None:
        """Populate the LOADED row from save-file metadata dicts.

        track_details: list of dicts with keys name, note_count, pitch_min, pitch_max, role.
        edit_selection_btn stays disabled (saves bypass track selection).
        """
        self._clear_loaded_cards()
        if not track_details:
            self._tracks_placeholder = QLabel("No track info in save.")
            self._tracks_placeholder.setProperty("role", "muted")
            self._loaded_cards_layout.addWidget(self._tracks_placeholder)
            return

        empty_index = 0
        for td in track_details:
            title = (td.get('name') or '').strip()
            if not title:
                empty_index += 1
                title = f"Track {empty_index}"
            note_count = td.get('note_count', 0)
            p_min = td.get('pitch_min')
            p_max = td.get('pitch_max')
            if p_min is not None and p_max is not None:
                range_str = f"{self._format_pitch(p_min)}-{self._format_pitch(p_max)}"
            else:
                range_str = "no notes"
            role = td.get('role', 'Auto-Detect')
            hand_label = self._HAND_LABEL.get(role, role)
            meta = f"{note_count} {self._NOTE_GLYPH}   {hand_label}   {range_str}"
            self._loaded_cards_layout.addWidget(PartCard(title, meta))

        self._loaded_cards_layout.addWidget(
            PartCard("Pedal", f"{pedal_count} events")
        )
        self._loaded_cards_layout.addStretch()

    def clear_loaded_summary(self) -> None:
        """Reset to the empty 'No file loaded.' state and disable the edit button."""
        self._clear_loaded_cards()
        self._tracks_placeholder = QLabel("No file loaded.")
        self._tracks_placeholder.setProperty("role", "muted")
        self._loaded_cards_layout.addWidget(self._tracks_placeholder)
        self.edit_selection_btn.setEnabled(False)
