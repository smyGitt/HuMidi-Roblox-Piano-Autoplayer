from PyQt6.QtWidgets import QFrame, QVBoxLayout, QLabel
from PyQt6.QtCore import Qt, pyqtSignal as Signal


class ClickableSaveCard(QFrame):
    clicked = Signal()

    def __init__(self, title: str, meta: str, parent=None):
        super().__init__(parent)
        self.setObjectName("save_card")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        vbox = QVBoxLayout(self)
        vbox.setContentsMargins(8, 5, 8, 5)
        vbox.setSpacing(1)
        title_lbl = QLabel(title)
        title_lbl.setObjectName("part_card_title")
        vbox.addWidget(title_lbl)
        meta_lbl = QLabel(meta)
        meta_lbl.setObjectName("part_card_meta")
        vbox.addWidget(meta_lbl)

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
