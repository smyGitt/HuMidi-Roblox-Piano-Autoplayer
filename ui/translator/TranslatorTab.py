from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QLabel, QComboBox,
    QTextEdit, QSpinBox, QPushButton, QFrame, QStackedWidget
)
from PyQt6.QtCore import pyqtSignal as Signal, Qt

from ui.widgets.toggle_switch import ToggleSwitch

from core.translator import FormatRegistry
from ui.widgets import make_card


class TranslatorTab(QWidget):
    # (text, format_name, bpm, humanize)
    play_sheet_requested = Signal(str, str, int, bool)
    # (format_name)
    export_requested = Signal(str)

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
        title_lbl = QLabel("Translator")
        title_lbl.setObjectName("page_header_title")
        hl.addWidget(title_lbl)
        hl.addStretch()
        outer.addWidget(header)

        # Body widget restores side margins
        body = QWidget()
        layout = QVBoxLayout(body)
        layout.setContentsMargins(16, 8, 16, 16)
        layout.setSpacing(0)
        outer.addWidget(body, 1)

        # -- Toolbar: format dropdown (left) | Import / Export toggle (right) --
        toolbar = QFrame()
        tbl = QHBoxLayout(toolbar)
        tbl.setContentsMargins(0, 4, 0, 8)
        tbl.setSpacing(8)

        fmt_lbl = QLabel("Format")
        fmt_lbl.setProperty("variant", "muted")
        self.format_combo = QComboBox()
        self.format_combo.addItems(FormatRegistry.names())
        self.format_combo.setToolTip("Select the Roblox piano sheet format")
        tbl.addWidget(fmt_lbl)
        tbl.addWidget(self.format_combo)
        tbl.addStretch()

        self._import_btn = QPushButton("Import")
        self._import_btn.setObjectName("sub_tab_btn")
        self._import_btn.setProperty("active", "true")
        self._import_btn.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self._export_btn = QPushButton("Export")
        self._export_btn.setObjectName("sub_tab_btn")
        self._export_btn.setProperty("active", "false")
        self._export_btn.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self._import_btn.clicked.connect(lambda: self._set_mode(0))
        self._export_btn.clicked.connect(lambda: self._set_mode(1))
        tbl.addWidget(self._import_btn)
        tbl.addWidget(self._export_btn)
        layout.addWidget(toolbar)

        # -- Workspace (stacked: import page / export page) --------------------
        self._workspace = QStackedWidget()
        self._workspace.addWidget(self._build_import_page())
        self._workspace.addWidget(self._build_export_page())
        layout.addWidget(self._workspace, 1)

    # ── Import page ───────────────────────────────────────────────────────────

    def _build_import_page(self) -> QWidget:
        page = QWidget()
        vl = QVBoxLayout(page)
        vl.setContentsMargins(0, 0, 0, 0)
        vl.setSpacing(10)

        body = QHBoxLayout()
        body.setSpacing(10)

        # Source card: sheet-text input
        src_card, src_body = make_card("Source")
        self.import_text = QTextEdit()
        self.import_text.setProperty("variant", "mono")
        self.import_text.setPlaceholderText(
            "e.g.\ne e e [6t] e\ne y 9 y t [wy] t\ne w [6e] e e t"
        )
        src_body.addWidget(self.import_text)
        body.addWidget(src_card, 1)

        # Preview card (notation legend / key-range bar / stat tiles not yet implemented)
        prev_card, prev_body = make_card("Preview")
        prev_lbl = QLabel("Not yet implemented")
        prev_lbl.setProperty("variant", "muted")
        prev_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        # TODO: parsed preview -- notation legend, key-range bar, stat tiles
        prev_body.addWidget(prev_lbl)
        body.addWidget(prev_card, 1)

        vl.addLayout(body, 1)

        # Action bar
        ab = QHBoxLayout()
        ab.setContentsMargins(0, 4, 0, 0)
        ab.setSpacing(8)
        bpm_lbl = QLabel("BPM")
        bpm_lbl.setProperty("variant", "muted")
        self.bpm_spinbox = QSpinBox()
        self.bpm_spinbox.setRange(20, 400)
        self.bpm_spinbox.setValue(120)
        self.bpm_spinbox.setFixedWidth(70)
        self.bpm_spinbox.setToolTip("Tempo used to calculate note durations from the sheet")
        self.humanize_check = ToggleSwitch("Humanize")
        self.humanize_check.setToolTip(
            "Apply current humanization settings during playback.\n"
            "When unchecked, the sheet plays back exactly as written."
        )
        self.import_play_btn = QPushButton("▶  Play Sheet")
        self.import_play_btn.setToolTip("Convert the pasted sheet to keystrokes and begin playback")
        self.import_play_btn.clicked.connect(self._on_play_clicked)
        ab.addWidget(bpm_lbl)
        ab.addWidget(self.bpm_spinbox)
        ab.addWidget(self.humanize_check)
        ab.addStretch()
        ab.addWidget(self.import_play_btn)
        vl.addLayout(ab)

        return page

    # ── Export page ───────────────────────────────────────────────────────────

    def _build_export_page(self) -> QWidget:
        page = QWidget()
        vl = QVBoxLayout(page)
        vl.setContentsMargins(0, 0, 0, 0)
        vl.setSpacing(10)

        body = QHBoxLayout()
        body.setSpacing(10)

        # Source card (loaded MIDI track list for export mode -- not yet implemented)
        track_card, track_body = make_card("Source")
        track_lbl = QLabel("Not yet implemented")
        track_lbl.setProperty("variant", "muted")
        track_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        # TODO: loaded MIDI track list for export mode
        track_body.addWidget(track_lbl)
        body.addWidget(track_card, 1)

        # Output card: generated sheet text
        out_card, out_body = make_card("Output")
        self.export_status_label = QLabel(
            "Load a MIDI file on the Playback tab, then click Generate."
        )
        self.export_status_label.setProperty("variant", "placeholder")
        out_body.addWidget(self.export_status_label)
        self.export_text = QTextEdit()
        self.export_text.setReadOnly(True)
        self.export_text.setProperty("variant", "mono")
        self.export_text.setPlaceholderText("Generated sheet will appear here...")
        out_body.addWidget(self.export_text)
        body.addWidget(out_card, 1)

        vl.addLayout(body, 1)

        # Action bar
        ab = QHBoxLayout()
        ab.setContentsMargins(0, 4, 0, 0)
        ab.setSpacing(8)
        self.export_generate_btn = QPushButton("Generate Sheet")
        self.export_generate_btn.setToolTip(
            "Convert the currently loaded MIDI notes to sheet text in the selected format"
        )
        self.export_generate_btn.clicked.connect(self._on_export_clicked)
        ab.addWidget(self.export_generate_btn)
        ab.addStretch()
        self.copy_btn = QPushButton("Copy to Clipboard")
        self.copy_btn.setToolTip("Copy the generated sheet to the clipboard")
        self.copy_btn.clicked.connect(self._on_copy_clicked)
        ab.addWidget(self.copy_btn)
        # TODO: Save Sheet to File button
        vl.addLayout(ab)

        return page

    # ── Internal ──────────────────────────────────────────────────────────────

    def _set_mode(self, index: int) -> None:
        self._workspace.setCurrentIndex(index)
        self._import_btn.setProperty("active", "true" if index == 0 else "false")
        self._export_btn.setProperty("active", "false" if index == 0 else "true")
        for btn in (self._import_btn, self._export_btn):
            btn.style().unpolish(btn)
            btn.style().polish(btn)

    def _on_play_clicked(self):
        text = self.import_text.toPlainText().strip()
        if not text:
            return
        self.play_sheet_requested.emit(
            text,
            self.format_combo.currentText(),
            self.bpm_spinbox.value(),
            self.humanize_check.isChecked(),
        )

    def _on_export_clicked(self):
        self.export_requested.emit(self.format_combo.currentText())

    def _on_copy_clicked(self):
        from PyQt6.QtWidgets import QApplication
        QApplication.clipboard().setText(self.export_text.toPlainText())

    # ── Public API ────────────────────────────────────────────────────────────

    def set_export_text(self, text: str):
        self.export_text.setPlainText(text)
        note_count = sum(
            1 for line in text.splitlines() if line.strip() and not line.startswith('#')
        )
        self.export_status_label.setText(f"Generated {note_count} line(s).")
        self.export_status_label.setProperty("variant", "success")
        self.export_status_label.style().unpolish(self.export_status_label)
        self.export_status_label.style().polish(self.export_status_label)
