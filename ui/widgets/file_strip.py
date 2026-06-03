from PyQt6.QtWidgets import (QFrame, QVBoxLayout, QHBoxLayout, QLabel,
                              QPushButton, QSizePolicy, QWidget)
from PyQt6.QtCore import Qt, QSize, pyqtSignal as Signal

from ui.widgets.ph_icon import ph_icon


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
        self._tile_icon = QLabel()
        self._tile_icon.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._tile_icon.setFixedSize(QSize(20, 20))
        self._tile_icon.setScaledContents(True)
        tile_layout.addWidget(self._tile_icon, alignment=Qt.AlignmentFlag.AlignCenter)
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

    def update_icon_color(self, hex_color: str, size: int = 20) -> None:
        """Re-render the music-note tile icon with the given theme color."""
        self._tile_icon.setPixmap(ph_icon("music-note", hex_color, size).pixmap(size * 2, size * 2))

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
