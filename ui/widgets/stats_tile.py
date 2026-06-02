from PyQt6.QtWidgets import QFrame, QVBoxLayout, QLabel
from PyQt6.QtCore import Qt


class StatsTile(QFrame):
    def __init__(self, label: str, value: str, parent=None):
        super().__init__(parent)
        self.setObjectName("stats_tile")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        vbox = QVBoxLayout(self)
        vbox.setContentsMargins(10, 6, 10, 6)
        vbox.setSpacing(2)
        val_lbl = QLabel(value)
        val_lbl.setObjectName("stats_tile_value")
        val_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        lbl_lbl = QLabel(label)
        lbl_lbl.setObjectName("stats_tile_label")
        lbl_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        vbox.addWidget(val_lbl)
        vbox.addWidget(lbl_lbl)
