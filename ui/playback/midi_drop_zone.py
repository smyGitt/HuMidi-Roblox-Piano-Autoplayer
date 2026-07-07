from PySide6.QtWidgets import (QFrame, QVBoxLayout, QHBoxLayout, QLabel,
                              QPushButton)
from PySide6.QtCore import Qt, Signal

from ui.widgets.ph_icon_label import PhIconLabel


class MidiDropZone(QFrame):
    """File-drop affordance for the Playback page File tab.

    Handles drag events and emits file_dropped(str). Has no own visual
    styling - it is placed inside a make_card("REPLACE", dashed_border=True)
    card, which provides the border and drag-over visual. Drag state is
    propagated to the parent card frame via _set_drag_active.

    The Browse and Load Save buttons are exposed as attributes so callers
    bind them exactly as before.
    """

    file_dropped = Signal(str)

    _ACCEPTED_EXTS = (".mid", ".midi")

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAcceptDrops(True)

        vbox = QVBoxLayout(self)
        vbox.setContentsMargins(0, 4, 0, 4)
        vbox.setSpacing(8)

        self.drop_icon = PhIconLabel("folder-open", size=48)

        drop_hint = QLabel("Drop a .mid file")
        drop_hint.setAlignment(Qt.AlignmentFlag.AlignCenter)
        drop_hint.setProperty("variant", "drop_hint")

        drop_sub = QLabel("OR BROWSE BELOW")
        drop_sub.setAlignment(Qt.AlignmentFlag.AlignCenter)
        drop_sub.setProperty("variant", "muted")

        self.browse_button = QPushButton("Browse…")
        self.browse_button.setToolTip("Open a MIDI file to play")

        self.load_saved_btn = QPushButton("Load Save")
        self.load_saved_btn.setToolTip("Load a previously saved humanized performance")

        btn_row = QHBoxLayout()
        btn_row.setSpacing(6)
        btn_row.addWidget(self.browse_button)
        btn_row.addWidget(self.load_saved_btn)

        vbox.addStretch()
        vbox.addWidget(self.drop_icon, alignment=Qt.AlignmentFlag.AlignHCenter)
        vbox.addWidget(drop_hint)
        vbox.addWidget(drop_sub)
        vbox.addSpacing(8)
        vbox.addLayout(btn_row)
        vbox.addStretch()

    @classmethod
    def _first_midi_path(cls, mime) -> str | None:
        if not mime.hasUrls():
            return None
        for url in mime.urls():
            if not url.isLocalFile():
                continue
            path = url.toLocalFile()
            if path.lower().endswith(cls._ACCEPTED_EXTS):
                return path
        return None

    def _set_drag_active(self, active: bool) -> None:
        card = self.parent()
        if card is not None and hasattr(card, "set_drag_active"):
            card.set_drag_active(active)

    def dragEnterEvent(self, event):
        if self._first_midi_path(event.mimeData()):
            event.acceptProposedAction()
            self._set_drag_active(True)
        else:
            event.ignore()

    def dragMoveEvent(self, event):
        if self._first_midi_path(event.mimeData()):
            event.acceptProposedAction()
        else:
            event.ignore()

    def dragLeaveEvent(self, event):
        self._set_drag_active(False)
        super().dragLeaveEvent(event)

    def dropEvent(self, event):
        path = self._first_midi_path(event.mimeData())
        self._set_drag_active(False)
        if path:
            event.acceptProposedAction()
            self.file_dropped.emit(path)
        else:
            event.ignore()
