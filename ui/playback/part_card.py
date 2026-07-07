from PySide6.QtWidgets import QFrame, QVBoxLayout, QLabel
from PySide6.QtCore import Qt


class PartCard(QFrame):
    def __init__(self, title: str, meta: str, parent=None):
        super().__init__(parent)
        self.setObjectName("part_card")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        vbox = QVBoxLayout(self)
        vbox.setContentsMargins(8, 5, 8, 5)
        vbox.setSpacing(1)
        title_lbl = QLabel(title)
        title_lbl.setObjectName("part_card_title")
        vbox.addWidget(title_lbl)
        meta_lbl = QLabel(meta)
        meta_lbl.setObjectName("part_card_meta")
        vbox.addWidget(meta_lbl)
