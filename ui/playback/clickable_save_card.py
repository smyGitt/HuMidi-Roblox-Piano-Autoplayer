from PyQt6.QtWidgets import QFrame, QVBoxLayout, QHBoxLayout, QLabel
from PyQt6.QtCore import Qt, pyqtSignal as Signal

from ui.widgets.elided_label import ElidedLabel
from ui.widgets.ph_icon import ph_icon


class ClickableSaveCard(QFrame):
    clicked = Signal()

    def __init__(self, title: str, meta: str, time_str: str = "",
                 icon_color: str = "#888888", parent=None):
        super().__init__(parent)
        self.setObjectName("save_card")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setMinimumWidth(0)

        vbox = QVBoxLayout(self)
        vbox.setContentsMargins(8, 5, 8, 5)
        vbox.setSpacing(2)

        title_lbl = ElidedLabel(title)
        title_lbl.setObjectName("part_card_title")
        vbox.addWidget(title_lbl)

        meta_lbl = ElidedLabel(meta)
        meta_lbl.setObjectName("part_card_meta")
        vbox.addWidget(meta_lbl)

        self._time_icon_lbl: QLabel | None = None
        if time_str:
            time_row = QHBoxLayout()
            time_row.setContentsMargins(0, 0, 0, 0)
            time_row.setSpacing(4)

            self._time_icon_lbl = QLabel()
            self._time_icon_lbl.setFixedSize(10, 10)
            self._time_icon_lbl.setScaledContents(True)
            self._time_icon_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
            time_row.addWidget(self._time_icon_lbl)

            time_lbl = ElidedLabel(time_str)
            time_lbl.setObjectName("part_card_meta")
            time_row.addWidget(time_lbl, 1)

            vbox.addLayout(time_row)
            self._render_time_icon(icon_color)

    def _render_time_icon(self, color: str) -> None:
        if self._time_icon_lbl is None:
            return
        _sz = 10
        pix = ph_icon("clock", color, _sz).pixmap(_sz * 2, _sz * 2)
        self._time_icon_lbl.setPixmap(pix)

    def mousePressEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            self.setProperty("pressed", True)
            self.style().unpolish(self)
            self.style().polish(self)
        super().mousePressEvent(event)

    def mouseReleaseEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            self.setProperty("pressed", False)
            self.style().unpolish(self)
            self.style().polish(self)
            if self.rect().contains(event.position().toPoint()):
                self.clicked.emit()
        super().mouseReleaseEvent(event)
