from PySide6.QtWidgets import QFrame, QVBoxLayout, QHBoxLayout
from PySide6.QtCore import Qt, Signal

from ui.widgets.elided_label import ElidedLabel
from ui.widgets.ph_icon_label import PhIconLabel


class ClickableSaveCard(QFrame):
    clicked = Signal()

    def __init__(self, title: str, meta: str, time_str: str = "", parent=None):
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

        if time_str:
            time_row = QHBoxLayout()
            time_row.setContentsMargins(0, 0, 0, 0)
            time_row.setSpacing(4)

            # Clock icon color comes from QSS via PhIconLabel qproperty-iconColor.
            time_icon = PhIconLabel("clock", 10)
            time_icon.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
            time_row.addWidget(time_icon)

            time_lbl = ElidedLabel(time_str)
            time_lbl.setObjectName("part_card_meta")
            time_row.addWidget(time_lbl, 1)

            vbox.addLayout(time_row)

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
