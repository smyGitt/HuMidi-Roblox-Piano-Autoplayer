from PyQt6.QtWidgets import (QDialog, QVBoxLayout, QLabel, QTableWidget,
                             QTableWidgetItem, QHeaderView, QAbstractItemView,
                             QComboBox, QDialogButtonBox)
from PyQt6.QtCore import Qt

from ui.theme import ThemeManager, generate_stylesheet


class TrackSelectionDialog(QDialog):
    def __init__(self, tracks, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Select Tracks")
        self.resize(720, 400)
        self.setStyleSheet(generate_stylesheet(ThemeManager.get_active()))
        self.tracks = tracks
        self._setup_ui()

    def _setup_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 12, 12, 12)
        layout.setSpacing(10)

        info_label = QLabel(
            "Select the tracks to include in playback. "
            "Optionally override the hand assignment for each track."
        )
        info_label.setWordWrap(True)
        info_label.setProperty("role", "muted")
        layout.addWidget(info_label)

        self.table = QTableWidget()
        self.table.setColumnCount(5)
        self.table.setHorizontalHeaderLabels(
            ["Play", "Track Name", "Instrument", "Notes", "Hand Assignment"]
        )
        self.table.horizontalHeader().setSectionResizeMode(1, QHeaderView.ResizeMode.Stretch)
        self.table.setSelectionBehavior(QAbstractItemView.SelectionBehavior.SelectRows)
        self.table.setAlternatingRowColors(True)
        self.table.verticalHeader().setVisible(False)
        self.table.setShowGrid(False)
        layout.addWidget(self.table)

        self.table.setRowCount(len(self.tracks))
        self.table.verticalHeader().setMinimumSectionSize(36)
        self.table.verticalHeader().setDefaultSectionSize(36)
        self.checkboxes = []
        self.role_combos = []

        for i, track in enumerate(self.tracks):
            check_item = QTableWidgetItem()
            check_item.setFlags(
                Qt.ItemFlag.ItemIsUserCheckable | Qt.ItemFlag.ItemIsEnabled
            )
            check_state = (
                Qt.CheckState.Unchecked if track.is_drum else Qt.CheckState.Checked
            )
            check_item.setCheckState(check_state)
            self.table.setItem(i, 0, check_item)
            self.checkboxes.append(check_item)

            name_item = QTableWidgetItem(track.name)
            name_item.setFlags(name_item.flags() & ~Qt.ItemFlag.ItemIsEditable)
            self.table.setItem(i, 1, name_item)

            inst_item = QTableWidgetItem(track.instrument_name)
            inst_item.setFlags(inst_item.flags() & ~Qt.ItemFlag.ItemIsEditable)
            self.table.setItem(i, 2, inst_item)

            notes_item = QTableWidgetItem(str(track.note_count))
            notes_item.setFlags(notes_item.flags() & ~Qt.ItemFlag.ItemIsEditable)
            notes_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
            self.table.setItem(i, 3, notes_item)

            combo = QComboBox()
            combo.setStyleSheet("QComboBox { min-height: 0px; padding: 2px 8px; }")
            combo.setFixedHeight(28)
            combo.addItems(["Auto-Detect", "Left Hand", "Right Hand"])
            self.table.setCellWidget(i, 4, combo)
            self.role_combos.append(combo)

        self.table.resizeColumnToContents(0)
        self.table.resizeColumnToContents(3)
        self.table.resizeColumnToContents(4)

        buttons = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        ok_btn = buttons.button(QDialogButtonBox.StandardButton.Ok)
        if ok_btn:
            ok_btn.setObjectName("save_button")
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
