from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QFrame,
    QPushButton, QTextEdit, QApplication, QLabel
)
from PySide6.QtCore import Qt

from ui.widgets import make_card


class DebugTab(QWidget):

    def __init__(self, parent=None):
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        # Full-width page header bar
        header = QFrame()
        header.setObjectName("page_header")
        header.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        hl = QHBoxLayout(header)
        hl.setContentsMargins(14, 10, 14, 10)
        hl.setSpacing(8)
        title_lbl = QLabel("Debug")
        title_lbl.setObjectName("page_header_title")
        hl.addWidget(title_lbl)
        hl.addStretch()
        outer.addWidget(header)

        # Body widget restores side margins
        body = QWidget()
        layout = QVBoxLayout(body)
        layout.setContentsMargins(16, 8, 16, 12)
        layout.setSpacing(0)
        outer.addWidget(body, 1)

        # -- Level filter bar --------------------------------------------------
        # TODO: wire filter buttons to actually filter log_output by level
        filter_bar = QFrame()
        fbl = QHBoxLayout(filter_bar)
        fbl.setContentsMargins(0, 0, 0, 8)
        fbl.setSpacing(4)
        for label in ("All", "Info", "Debug", "Warn", "OK"):
            btn = QPushButton(label)
            btn.setObjectName("sub_tab_btn")
            btn.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
            btn.setProperty("active", "true" if label == "All" else "false")
            btn.setEnabled(False)  # placeholder -- level filtering not yet implemented
            fbl.addWidget(btn)
        fbl.addStretch()
        # TODO: Auto-scroll toggle
        layout.addWidget(filter_bar)

        # -- Split body: Console (left, ~1.55x) | Levels + Snapshot (right) ---
        body_row = QHBoxLayout()
        body_row.setSpacing(10)

        # Console card (left)
        console_card, console_body = make_card("Console")
        self.log_output = QTextEdit()
        self.log_output.setReadOnly(True)
        self.log_output.setProperty("variant", "mono")
        console_body.addWidget(self.log_output)
        body_row.addWidget(console_card, 31)  # 31:20 ~= 1.55:1

        # Right column: Levels chip card + Session Snapshot card (stacked)
        right_col = QVBoxLayout()
        right_col.setSpacing(10)

        levels_card, levels_body = make_card("Levels")
        levels_lbl = QLabel("Not yet implemented")
        levels_lbl.setProperty("variant", "muted")
        levels_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        # TODO: per-level count chips (Info / Debug / Warn / OK tallies)
        levels_body.addWidget(levels_lbl)
        right_col.addWidget(levels_card)

        snap_card, snap_body = make_card("Session Snapshot")
        snap_lbl = QLabel("Not yet implemented")
        snap_lbl.setProperty("variant", "muted")
        snap_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        # TODO: key/value snapshot rows (MIDI file, tempo, pedal style, etc.)
        snap_body.addWidget(snap_lbl)
        right_col.addWidget(snap_card)
        right_col.addStretch()

        body_row.addLayout(right_col, 20)
        layout.addLayout(body_row, 1)

        # -- Footer action bar -------------------------------------------------
        footer = QHBoxLayout()
        footer.setContentsMargins(0, 8, 0, 0)
        footer.setSpacing(6)
        self.log_clear_btn = QPushButton("Clear")
        self.log_clear_btn.setToolTip("Clear all log entries")
        self.log_copy_btn = QPushButton("Copy Log")
        self.log_copy_btn.setToolTip("Copy the full log to clipboard")
        self.log_clear_btn.clicked.connect(self.log_output.clear)
        self.log_copy_btn.clicked.connect(self._copy_to_clipboard)
        footer.addWidget(self.log_clear_btn)
        footer.addWidget(self.log_copy_btn)
        footer.addStretch()
        # TODO: warnings/errors summary label (e.g. "2 warnings, 0 errors")
        layout.addLayout(footer)

    def _copy_to_clipboard(self) -> None:
        QApplication.clipboard().setText(self.log_output.toPlainText())
