from PySide6.QtWidgets import (QFrame, QVBoxLayout, QHBoxLayout, QLabel,
                              QPushButton, QSizePolicy, QWidget)
from PySide6.QtCore import Qt, Signal

from ui.widgets.ph_icon_label import PhIconLabel


class FileStrip(QFrame):
    """Persistent file info strip shown above the sub-tab content area."""

    replace_requested = Signal()
    reveal_requested  = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setObjectName("file_strip")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)

        hbox = QHBoxLayout(self)
        hbox.setContentsMargins(14, 8, 14, 8)
        hbox.setSpacing(10)

        # Icon tile
        tile = QFrame()
        tile.setObjectName("file_strip_tile")
        tile.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        tile.setFixedSize(34, 34)
        tile_layout = QVBoxLayout(tile)
        tile_layout.setContentsMargins(0, 0, 0, 0)
        self.tile_icon = PhIconLabel("music-note", size=20)
        tile_layout.addWidget(self.tile_icon, alignment=Qt.AlignmentFlag.AlignCenter)
        hbox.addWidget(tile)

        # File info column
        info_col = QWidget()
        info_col.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents, False)
        info_layout = QVBoxLayout(info_col)
        info_layout.setContentsMargins(0, 0, 0, 0)
        info_layout.setSpacing(2)

        self._name_lbl = _ElidingLabel("No file loaded.")
        self._name_lbl.setObjectName("file_strip_name")

        self._meta_lbl = QLabel("")
        self._meta_lbl.setObjectName("file_strip_meta")
        self._meta_lbl.setVisible(False)

        info_layout.addWidget(self._name_lbl)
        info_layout.addWidget(self._meta_lbl)
        hbox.addWidget(info_col, 1)

        # Action buttons
        btn_layout = QHBoxLayout()
        btn_layout.setContentsMargins(0, 0, 0, 0)
        btn_layout.setSpacing(6)

        self.replace_btn = QPushButton("Replace")
        self.replace_btn.setToolTip("Load a different MIDI file")
        self.replace_btn.setFixedHeight(28)

        self.reveal_btn = QPushButton("Reveal")
        self.reveal_btn.setToolTip("Show the file in Explorer")
        self.reveal_btn.setFixedHeight(28)

        self.replace_btn.clicked.connect(self.replace_requested)
        self.reveal_btn.clicked.connect(self.reveal_requested)

        btn_layout.addWidget(self.replace_btn)
        btn_layout.addWidget(self.reveal_btn)
        hbox.addLayout(btn_layout)

    def update_file(self, name: str, meta: str = "") -> None:
        self._name_lbl.setText(name)
        self._meta_lbl.setText(meta)
        self._meta_lbl.setVisible(bool(meta))


class _ElidingLabel(QLabel):
    """QLabel that elides text with '...' when it does not fit."""

    def __init__(self, text="", parent=None):
        super().__init__(parent)
        self._full_text = text
        self.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Preferred)
        if text:
            self._update_elided()

    def setText(self, text):
        self._full_text = text
        self._update_elided()

    def resizeEvent(self, event):
        super().resizeEvent(event)
        self._update_elided()

    def _update_elided(self):
        width = self.contentsRect().width()
        if width <= 0:
            return
        elided = self.fontMetrics().elidedText(
            self._full_text, Qt.TextElideMode.ElideRight, width
        )
        super().setText(elided)
