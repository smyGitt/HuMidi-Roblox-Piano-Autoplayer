from PyQt6.QtWidgets import (QDialog, QVBoxLayout, QLabel, QTableWidget, 
                             QTableWidgetItem, QHeaderView, QAbstractItemView, 
                             QComboBox, QDialogButtonBox)
from PyQt6.QtCore import Qt

class TrackSelectionDialog(QDialog):
    def __init__(self, tracks, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Select Tracks & Assign Hands")
        self.resize(700, 400)
        self.tracks = tracks
        self._setup_ui()

    def _setup_ui(self):
        layout = QVBoxLayout(self)
        info_label = QLabel("Select the tracks you want to play. You can also manually assign hands to specific tracks.")
        info_label.setWordWrap(True)
        layout.addWidget(info_label)

        self.table = QTableWidget()
        self.table.setColumnCount(5)
        self.table.setHorizontalHeaderLabels(["Play", "Track Name", "Instrument", "Notes", "Hand Assignment"])
        self.table.horizontalHeader().setSectionResizeMode(1, QHeaderView.ResizeMode.Stretch)
        self.table.setSelectionBehavior(QAbstractItemView.SelectionBehavior.SelectRows)
        layout.addWidget(self.table)
        
        self.table.setRowCount(len(self.tracks))
        self.checkboxes = []
        self.role_combos = []

        for i, track in enumerate(self.tracks):
            check_item = QTableWidgetItem()
            check_item.setFlags(Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsEnabled)
            check_state = Qt.CheckState.Unchecked if track.is_drum else Qt.CheckState.Checked
            check_item.setCheckState(check_state)
            self.table.setItem(i, 0, check_item)
            self.checkboxes.append(check_item)
            self.table.setItem(i, 1, QTableWidgetItem(track.name))
            self.table.setItem(i, 2, QTableWidgetItem(track.instrument_name))
            self.table.setItem(i, 3, QTableWidgetItem(str(track.note_count)))
            combo = QComboBox()
            combo.addItems(["Auto-Detect", "Left Hand", "Right Hand"])
            self.table.setCellWidget(i, 4, combo)
            self.role_combos.append(combo)

        buttons = QDialogButtonBox(QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel)
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

    def get_selection(self):
        result = []
        for i, track in enumerate(self.tracks):
            if self.checkboxes[i].checkState() == Qt.CheckState.Checked:
                role = self.role_combos[i].currentText()
                result.append((track, role))
        return result