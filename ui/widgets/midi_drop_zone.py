from PyQt6.QtWidgets import (QFrame, QVBoxLayout, QHBoxLayout, QLabel,
                              QPushButton)
from PyQt6.QtCore import Qt, QSize, pyqtSignal as Signal

from ui.widgets.ph_icon import ph_icon


class MidiDropZone(QFrame):
    """File-drop affordance for the Playback page File tab.

    Visually matches the previous REPLACE card content (section title, folder
    glyph, italic hint, muted sub-caption, Browse / Load Save buttons) and
    adds a thick dashed border that darkens on drag-over. Accepts .mid and
    .midi local file drops and emits file_dropped(str) with the absolute path.
    The Browse and Load Save buttons are exposed as attributes so callers
    bind them exactly as before.
    """

    file_dropped = Signal(str)

    _ACCEPTED_EXTS = (".mid", ".midi")

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setObjectName("midi_dropzone")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self.setAcceptDrops(True)
        self.setProperty("drag_active", "false")

        vbox = QVBoxLayout(self)
        vbox.setContentsMargins(14, 12, 14, 14)
        vbox.setSpacing(8)

        title_lbl = QLabel("REPLACE")
        title_lbl.setProperty("role", "section")

        self._drop_icon_lbl = QLabel()
        self._drop_icon_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._drop_icon_lbl.setFixedSize(QSize(48, 48))
        self._drop_icon_lbl.setScaledContents(True)

        drop_hint = QLabel("Drop a .mid file")
        drop_hint.setAlignment(Qt.AlignmentFlag.AlignCenter)
        drop_hint.setStyleSheet(
            'font-family: "Georgia", serif; font-style: italic; font-size: 13pt;'
        )

        drop_sub = QLabel("OR BROWSE BELOW")
        drop_sub.setAlignment(Qt.AlignmentFlag.AlignCenter)
        drop_sub.setProperty("role", "muted")

        self.browse_button = QPushButton("Browse…")
        self.browse_button.setToolTip("Open a MIDI file to play")

        self.load_saved_btn = QPushButton("Load Save")
        self.load_saved_btn.setToolTip("Load a previously saved humanized performance")

        btn_row = QHBoxLayout()
        btn_row.setSpacing(6)
        btn_row.addWidget(self.browse_button)
        btn_row.addWidget(self.load_saved_btn)

        vbox.addWidget(title_lbl)
        vbox.addStretch()
        vbox.addWidget(self._drop_icon_lbl, alignment=Qt.AlignmentFlag.AlignHCenter)
        vbox.addWidget(drop_hint)
        vbox.addWidget(drop_sub)
        vbox.addSpacing(8)
        vbox.addLayout(btn_row)
        vbox.addStretch()

    def update_icon_color(self, hex_color: str, size: int = 48) -> None:
        """Re-render the drop-zone folder icon with the given theme color."""
        self._drop_icon_lbl.setPixmap(
            ph_icon("folder-open", hex_color, size).pixmap(size * 2, size * 2)
        )

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
        self.setProperty("drag_active", "true" if active else "false")
        self.style().unpolish(self)
        self.style().polish(self)
        self.update()

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
