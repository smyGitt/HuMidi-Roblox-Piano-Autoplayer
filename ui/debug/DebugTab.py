"""Debug page: filterable session log console, per-level tallies, and a
live session snapshot of the loaded file's key statistics."""

import re
from datetime import datetime

from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QGridLayout, QFrame,
    QPushButton, QTextEdit, QApplication, QLabel, QFileDialog, QMessageBox,
    QScrollArea
)
from PySide6.QtCore import Qt
from PySide6.QtGui import QTextCursor

from ui.widgets import make_card, ElidedLabel
from ui.widgets.toggle_switch import ToggleSwitch
from ui.widgets.slider_spinbox import NoScrollComboBox

# Log levels, in Levels-card display order.
_LEVELS = ("INFO", "DEBUG", "WARN", "OK")

# Filter-bar label -> level key (None means show everything).
_FILTER_LEVELS = {
    "All": None,
    "Info": "INFO",
    "Debug": "DEBUG",
    "Warn": "WARN",
    "OK": "OK",
}

# Substring markers used to classify a message (matched lowercase, in order:
# WARN beats OK beats DEBUG beats INFO).
_WARN_MARKERS = (
    "error", "failed", "failure", "rejected", "aborted",
    "crashed", "cancelled", "not found",
)
_OK_MARKERS = ("successful", "complete", "accepted", "finished")

# Oldest entries beyond this count are dropped from both the store and the
# console view so a long debug session cannot grow memory without bound.
_MAX_ENTRIES = 5000

# Fixed width of the Filter/Levels/Session Snapshot column. Only the Console
# card has layout stretch, so this column stays this width on any resize.
_RIGHT_COLUMN_WIDTH = 240

# Session Snapshot rows: (dict key, display label), in display order.
_SNAPSHOT_ROWS = (
    ("file", "File"),
    ("source", "Source"),
    ("tracks", "Tracks"),
    ("notes", "Notes"),
    ("duration", "Duration"),
    ("pedal", "Pedal"),
    ("tempo", "Tempo"),
    ("pedal_style", "Pedal style"),
)

_SNAPSHOT_EMPTY = "-"

# Matches an absolute Windows filesystem path (drive-letter or UNC) and
# captures its final path component, so redaction can collapse a full path
# (which embeds the local username under C:\Users\<name>\...) down to just
# the file or folder name.
_PATH_PATTERN = re.compile(r'(?:[A-Za-z]:[\\/]|\\\\)[^\s"\']*[\\/]([^\\/:\s"\']+)')


def _redact_paths(text: str) -> str:
    """Collapse absolute filesystem paths in `text` down to their final component."""
    return _PATH_PATTERN.sub(lambda m: m.group(1), text)


def _classify(message: str) -> str:
    """Derive a log level from the message text.

    Diagnostic lines are bracket-tagged at the source (e.g. [ACT], [PEDAL]),
    so a leading '[' means DEBUG unless a warning or success marker wins.
    """
    lowered = message.lower()
    if any(marker in lowered for marker in _WARN_MARKERS):
        return "WARN"
    if any(marker in lowered for marker in _OK_MARKERS):
        return "OK"
    if message.startswith("["):
        return "DEBUG"
    return "INFO"


class DebugTab(QWidget):
    """Filterable log console with level tallies and a session snapshot."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self._entries: list = []            # (level, formatted line) tuples
        self._counts = {lvl: 0 for lvl in _LEVELS}
        self._active_filter = None          # level key, None = All
        self._redact_paths = True           # mirrors Settings > Privacy > "Redact file paths"
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

        # -- Split body: Console (left, fills all extra space) | fixed-width
        # Filter + Levels + Snapshot column (right). Only the Console card
        # grows when the window is resized; the right column stays a fixed
        # width so its cards never stretch.
        body_row = QHBoxLayout()
        body_row.setSpacing(10)

        # Console card (left) -- the only widget with stretch, so it alone
        # absorbs any extra space from a window resize.
        console_card, console_body = make_card("Console")
        self.log_output = QTextEdit()
        self.log_output.setReadOnly(True)
        self.log_output.setProperty("variant", "mono")
        self.log_output.document().setMaximumBlockCount(_MAX_ENTRIES)
        console_body.addWidget(self.log_output)
        body_row.addWidget(console_card, 1)

        # Right column: Filter + Levels tally card + Session Snapshot card
        # (stacked, scrollable so extra cards never get clipped vertically).
        # Fixed width + stretch=0 below keeps it from growing on resize.
        right_col_widget = QWidget()
        right_col = QVBoxLayout(right_col_widget)
        right_col.setContentsMargins(0, 0, 4, 0)
        right_col.setSpacing(10)

        filter_card, filter_body = make_card("Filter")
        self.level_filter_combo = NoScrollComboBox()
        self.level_filter_combo.addItems(list(_FILTER_LEVELS.keys()))
        self.level_filter_combo.setProperty("variant", "compact")
        self.level_filter_combo.currentTextChanged.connect(self._set_filter)
        filter_body.addWidget(self.level_filter_combo)
        self.autoscroll_check = ToggleSwitch("Auto-scroll")
        self.autoscroll_check.setChecked(True)
        self.autoscroll_check.setToolTip("Keep the console scrolled to the newest entry")
        filter_body.addWidget(self.autoscroll_check)
        right_col.addWidget(filter_card)

        levels_card, levels_body = make_card("Levels")
        levels_grid = QGridLayout()
        levels_grid.setContentsMargins(0, 0, 0, 0)
        levels_grid.setHorizontalSpacing(8)
        levels_grid.setVerticalSpacing(2)
        levels_grid.setColumnStretch(1, 1)
        self._count_labels = {}
        for row, lvl in enumerate(_LEVELS):
            name_lbl = QLabel(lvl if lvl == "OK" else lvl.title())
            name_lbl.setProperty("variant", "muted")
            count_lbl = QLabel("0")
            count_lbl.setProperty("variant", "value")
            count_lbl.setAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
            levels_grid.addWidget(name_lbl, row, 0)
            levels_grid.addWidget(count_lbl, row, 1)
            self._count_labels[lvl] = count_lbl
        levels_body.addLayout(levels_grid)
        right_col.addWidget(levels_card)

        snap_card, snap_body = make_card("Session Snapshot")
        snap_grid = QGridLayout()
        snap_grid.setContentsMargins(0, 0, 0, 0)
        snap_grid.setHorizontalSpacing(8)
        snap_grid.setVerticalSpacing(2)
        snap_grid.setColumnStretch(1, 1)
        self._snapshot_labels = {}
        for row, (key, label) in enumerate(_SNAPSHOT_ROWS):
            name_lbl = QLabel(label)
            name_lbl.setProperty("variant", "muted")
            value_lbl = ElidedLabel(_SNAPSHOT_EMPTY)
            value_lbl.setProperty("variant", "value")
            value_lbl.setAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
            snap_grid.addWidget(name_lbl, row, 0)
            snap_grid.addWidget(value_lbl, row, 1)
            self._snapshot_labels[key] = value_lbl
        snap_body.addLayout(snap_grid)
        right_col.addWidget(snap_card)
        right_col.addStretch()

        right_scroll = QScrollArea()
        right_scroll.setWidgetResizable(True)
        right_scroll.setFrameShape(QFrame.Shape.NoFrame)
        right_scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        right_scroll.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAsNeeded)
        right_scroll.setWidget(right_col_widget)
        right_scroll.setFixedWidth(_RIGHT_COLUMN_WIDTH)
        body_row.addWidget(right_scroll, 0)
        layout.addLayout(body_row, 1)

        # -- Footer action bar -------------------------------------------------
        footer = QHBoxLayout()
        footer.setContentsMargins(0, 8, 0, 0)
        footer.setSpacing(6)
        self.log_clear_btn = QPushButton("Clear")
        self.log_clear_btn.setToolTip("Clear all log entries and level tallies")
        self.log_copy_btn = QPushButton("Copy Log")
        self.log_copy_btn.setToolTip("Copy the full log to clipboard")
        self.log_export_btn = QPushButton("Export Log")
        self.log_export_btn.setToolTip("Save the full log to a text file")
        self.log_clear_btn.clicked.connect(self.clear_log)
        self.log_copy_btn.clicked.connect(self._copy_to_clipboard)
        self.log_export_btn.clicked.connect(self._export_log)
        footer.addWidget(self.log_clear_btn)
        footer.addWidget(self.log_copy_btn)
        footer.addWidget(self.log_export_btn)
        footer.addStretch()
        layout.addLayout(footer)

    # -- Logging API -----------------------------------------------------------

    def append_log(self, message: str) -> None:
        """Classify, timestamp, store, and (filter permitting) display a message.

        This is the single entry point for all log traffic; every emitter
        (controller signals and direct MainWindow calls) routes through it so
        each entry gets a level and a timestamp exactly once.
        """
        text = message.strip("\n")
        if not text.strip():
            return
        if self._redact_paths:
            text = _redact_paths(text)
        level = _classify(text.lstrip().splitlines()[0])
        stamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
        entry = f"[{stamp}] {level:<5} {text}"

        self._entries.append((level, entry))
        if len(self._entries) > _MAX_ENTRIES:
            del self._entries[: len(self._entries) - _MAX_ENTRIES]
        self._counts[level] += 1
        self._count_labels[level].setText(str(self._counts[level]))

        if self._active_filter is None or self._active_filter == level:
            self._append_to_view(entry)

    def clear_log(self) -> None:
        """Clear the console, the entry store, and all level tallies."""
        self._entries.clear()
        for lvl in _LEVELS:
            self._counts[lvl] = 0
            self._count_labels[lvl].setText("0")
        self.log_output.clear()

    def set_redact_paths(self, redact: bool) -> None:
        """Toggle path redaction (Settings > Privacy) for subsequently logged messages.

        Does not rewrite entries already stored/displayed before the toggle.
        """
        self._redact_paths = redact

    # -- Session snapshot API ----------------------------------------------------

    def update_snapshot(self, fields: dict) -> None:
        """Update Session Snapshot rows from a partial dict.

        Recognised keys: file, source, tracks, notes, duration, pedal, tempo,
        pedal_style. A None value resets the row to the empty placeholder;
        unknown keys are ignored.
        """
        for key, value in fields.items():
            label = self._snapshot_labels.get(key)
            if label is not None:
                label.set_full_text(_SNAPSHOT_EMPTY if value is None else str(value))

    def clear_snapshot(self) -> None:
        """Reset every Session Snapshot row to the empty placeholder."""
        self.update_snapshot({key: None for key, _ in _SNAPSHOT_ROWS})

    # -- Internal helpers --------------------------------------------------------

    def _append_to_view(self, entry: str) -> None:
        # Insert as plain text via the cursor: QTextEdit.append() would try to
        # interpret entries containing '<' (e.g. pynput Key reprs) as rich text.
        cursor = self.log_output.textCursor()
        cursor.movePosition(QTextCursor.MoveOperation.End)
        if not self.log_output.document().isEmpty():
            cursor.insertBlock()
        cursor.insertText(entry)
        self._scroll_to_bottom()

    def _scroll_to_bottom(self) -> None:
        if self.autoscroll_check.isChecked():
            bar = self.log_output.verticalScrollBar()
            bar.setValue(bar.maximum())

    def _set_filter(self, name: str) -> None:
        self._active_filter = _FILTER_LEVELS.get(name)
        self._rebuild_view()

    def _rebuild_view(self) -> None:
        visible = [
            text for lvl, text in self._entries
            if self._active_filter is None or lvl == self._active_filter
        ]
        self.log_output.setPlainText("\n".join(visible))
        self._scroll_to_bottom()

    def _copy_to_clipboard(self) -> None:
        QApplication.clipboard().setText(self.log_output.toPlainText())

    def _export_log(self) -> None:
        default_name = datetime.now().strftime("humidi_log_%Y%m%d_%H%M%S.txt")
        path, _ = QFileDialog.getSaveFileName(
            self, "Export Log", default_name, "Text files (*.txt);;All files (*)"
        )
        if not path:
            return
        try:
            with open(path, "w", encoding="utf-8") as f:
                f.write("\n".join(text for _, text in self._entries))
        except OSError as e:
            QMessageBox.critical(self, "Export Failed", f"Could not write the log file:\n{e}")
