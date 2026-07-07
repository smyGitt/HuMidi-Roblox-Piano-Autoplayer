from typing import Callable

from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QGridLayout, QLabel,
    QPushButton, QStackedWidget,
)
from ui.widgets.slider_spinbox import NoScrollDoubleSpinBox
from PySide6.QtCore import Qt, Signal

from ui.widgets.section_card import make_card
from ui.widgets.ph_icon_label import PhIconLabel


class PedalAICard(QWidget):
    """PEDAL AI THRESHOLDS card in PlaybackTab's Playback sub-tab.

    Acts as a two-state gateway. Before the first generation, the body shows
    only a Generate button and a short description. After the first successful
    AI pedal generation, set_thresholds() reveals the threshold spinboxes and
    stats. Subsequent threshold edits take effect via the Apply toast; no
    re-generation is required unless the user wants to change thresholds.

    Never enters the post-generate state when loading from a save (pre-compiled
    events skip pedal generation entirely).
    """

    generate_requested = Signal()

    def __init__(self, parent=None):
        super().__init__(parent)
        self._default_on:  float = 0.5
        self._default_off: float = 0.5
        self._has_thresholds: bool = False
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        self.reset_icon = PhIconLabel("arrow-counter-clockwise", size=16)
        self.reset_icon.setToolTip("Reset thresholds to last auto-computed values")
        self.reset_icon.setCursor(Qt.CursorShape.PointingHandCursor)
        self.reset_icon.clicked.connect(self.reset_to_default)
        self.reset_icon.setVisible(False)

        card, body = make_card("PEDAL AI THRESHOLDS", title_buttons=[self.reset_icon])

        self._stack = QStackedWidget()
        body.addWidget(self._stack)

        # Page 0: pre-generate ------------------------------------------------
        pre_page = QWidget()
        pre_layout = QVBoxLayout(pre_page)
        pre_layout.setContentsMargins(0, 4, 0, 4)
        pre_layout.setSpacing(10)

        hint = QLabel(
            "Thresholds can be tweaked when:\n"
            "  1. Pedal is set to \"PedalAI\" in the Performance card\n"
            "  2. A MIDI file is loaded (not from a save)\n"
            "  3. Generate is clicked below"
        )
        hint.setProperty("variant", "muted")
        hint.setWordWrap(True)
        pre_layout.addWidget(hint)

        self.generate_btn = QPushButton("Generate pedal events")
        self.generate_btn.setEnabled(False)
        self.generate_btn.clicked.connect(self.generate_requested.emit)
        pre_layout.addWidget(self.generate_btn)
        pre_layout.addStretch()
        self._stack.addWidget(pre_page)

        # Page 1: post-generate (thresholds + stats) --------------------------
        post_page = QWidget()
        post_layout = QVBoxLayout(post_page)
        post_layout.setContentsMargins(0, 0, 0, 0)
        post_layout.setSpacing(0)

        row = QHBoxLayout()
        row.setSpacing(16)

        # Left: threshold controls + status
        left = QVBoxLayout()
        left.setSpacing(0)

        grid = QGridLayout()
        grid.setVerticalSpacing(10)
        grid.setHorizontalSpacing(8)
        grid.setColumnStretch(0, 1)

        self.threshold_on_spinbox = self._make_spinbox(
            "Pedal engages above this value (rising edge)"
        )
        grid.addWidget(
            self._make_label_pair("On Threshold", "pedal engages above this value"),
            0, 0, Qt.AlignmentFlag.AlignVCenter,
        )
        grid.addWidget(self.threshold_on_spinbox, 0, 1)

        self.threshold_off_spinbox = self._make_spinbox(
            "Pedal releases at or below this value (falling edge)"
        )
        grid.addWidget(
            self._make_label_pair("Off Threshold", "pedal releases at or below this value"),
            1, 0, Qt.AlignmentFlag.AlignVCenter,
        )
        grid.addWidget(self.threshold_off_spinbox, 1, 1)

        left.addLayout(grid)
        left.addStretch()

        row.addLayout(left)

        # Right: stats column
        right = QVBoxLayout()
        right.setSpacing(4)

        stats_title = QLabel("Pedal Stats")
        stats_title.setProperty("variant", "muted")
        right.addWidget(stats_title)

        sg = QGridLayout()
        sg.setVerticalSpacing(6)
        sg.setHorizontalSpacing(8)
        sg.setColumnStretch(1, 1)

        self._stat_avg  = self._add_stat(sg, 0, 0, "Avg hold")
        self._stat_min  = self._add_stat(sg, 1, 0, "Min hold")
        self._stat_max  = self._add_stat(sg, 2, 0, "Max hold")
        self._stat_freq = self._add_stat(sg, 3, 0, "Frequency")

        right.addLayout(sg)
        right.addStretch()

        row.addLayout(right)

        post_layout.addLayout(row)

        # Diagnostic section -- hidden until quality issues are detected
        self._diag_widget = QWidget()
        self._diag_widget.setObjectName("pedal_diag")
        self._diag_widget.setVisible(False)
        diag_layout = QVBoxLayout(self._diag_widget)
        diag_layout.setContentsMargins(8, 8, 8, 8)
        diag_layout.setSpacing(4)

        self._diag_title = QLabel()
        diag_layout.addWidget(self._diag_title)

        self._diag_rows_layout = QVBoxLayout()
        self._diag_rows_layout.setSpacing(2)
        diag_layout.addLayout(self._diag_rows_layout)

        post_layout.addWidget(self._diag_widget)
        post_layout.addStretch()
        self._stack.addWidget(post_page)

        outer.addWidget(card)

    @staticmethod
    def _add_stat(grid: QGridLayout, row: int, col: int, label: str) -> QLabel:
        name = QLabel(label)
        name.setProperty("variant", "muted")
        val = QLabel("--")
        val.setAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
        grid.addWidget(name, row, col)
        grid.addWidget(val, row, col + 1)
        return val

    @staticmethod
    def _make_spinbox(tooltip: str) -> NoScrollDoubleSpinBox:
        sb = NoScrollDoubleSpinBox()
        sb.setRange(0.0, 1.0)
        sb.setDecimals(3)
        sb.setSingleStep(0.001)
        sb.setFixedWidth(88)
        sb.setButtonSymbols(NoScrollDoubleSpinBox.ButtonSymbols.NoButtons)
        sb.setEnabled(False)
        sb.setToolTip(tooltip)
        return sb

    @staticmethod
    def _make_label_pair(label_text: str, desc_text: str) -> QWidget:
        container = QWidget()
        vbox = QVBoxLayout(container)
        vbox.setContentsMargins(0, 0, 0, 0)
        vbox.setSpacing(1)
        lbl = QLabel(label_text)
        desc = QLabel(desc_text)
        desc.setProperty("variant", "muted")
        vbox.addWidget(lbl)
        vbox.addWidget(desc)
        return container

    # Public API --------------------------------------------------------------

    @property
    def has_thresholds(self) -> bool:
        """True once set_thresholds() has been called successfully."""
        return self._has_thresholds

    def set_thresholds(self, on: float, off: float) -> None:
        """Populate controls, reveal the post-generate view, and enable editing.

        Only the first call (per reset() cycle) captures _default_on/_default_off,
        so reset_to_default() always restores the original Otsu-computed values
        from the first generation, not whatever was most recently regenerated.
        """
        if not self._has_thresholds:
            self._default_on  = on
            self._default_off = off
        self._has_thresholds = True
        self.threshold_on_spinbox.blockSignals(True)
        self.threshold_off_spinbox.blockSignals(True)
        self.threshold_on_spinbox.setValue(on)
        self.threshold_off_spinbox.setValue(off)
        self.threshold_on_spinbox.blockSignals(False)
        self.threshold_off_spinbox.blockSignals(False)
        self.threshold_on_spinbox.setEnabled(True)
        self.threshold_off_spinbox.setEnabled(True)
        self._stack.setCurrentIndex(1)
        self.reset_icon.setVisible(True)

    def reset_to_default(self) -> None:
        """Restore the original Otsu-computed threshold values from the
        first generation of this song (see set_thresholds)."""
        if not self._has_thresholds:
            return
        self.threshold_on_spinbox.blockSignals(True)
        self.threshold_off_spinbox.blockSignals(True)
        self.threshold_on_spinbox.setValue(self._default_on)
        self.threshold_off_spinbox.setValue(self._default_off)
        self.threshold_on_spinbox.blockSignals(False)
        self.threshold_off_spinbox.blockSignals(False)

    def get_threshold_on(self) -> float:
        return self.threshold_on_spinbox.value()

    def get_threshold_off(self) -> float:
        return self.threshold_off_spinbox.value()

    def set_stats(self, avg_dur: float, min_dur: float, max_dur: float, presses_per_min: float) -> None:
        """Populate pedal stats labels and evaluate output quality."""
        self._stat_avg.setText(f"{avg_dur:.2f} s")
        self._stat_min.setText(f"{min_dur:.2f} s")
        self._stat_max.setText(f"{max_dur:.2f} s")
        self._stat_freq.setText(f"{presses_per_min:.1f}/min")
        self._update_diagnostics(avg_dur, min_dur, max_dur, presses_per_min)

    def _update_diagnostics(
        self,
        avg_dur: float,
        min_dur: float,
        max_dur: float,
        presses_per_min: float,
    ) -> None:
        self._clear_diag_rows()

        is_chattering = presses_per_min > 45 or avg_dur < 0.25 or min_dur < 0.08
        is_sparse = (
            not is_chattering
            and (presses_per_min < 3 or max_dur > 12
                 or (avg_dur > 0 and max_dur / avg_dur > 8))
        )

        if not is_chattering and not is_sparse:
            self._diag_widget.setVisible(False)
            return

        curr_on  = self.threshold_on_spinbox.value()
        curr_off = self.threshold_off_spinbox.value()

        if is_chattering:
            self._diag_title.setText("⚠  Pedal chattering detected")
            suggested_on  = min(0.990, round(curr_on  + 0.08, 3))
            suggested_off = max(0.010, round(curr_off - 0.05, 3))
            self._add_diag_row(
                f"  Raise On Threshold to {suggested_on:.3f}  (fewer false triggers)",
                lambda v=suggested_on: self.threshold_on_spinbox.setValue(v),
            )
            self._add_diag_row(
                f"  Lower Off Threshold to {suggested_off:.3f}  (widen hysteresis gap)",
                lambda v=suggested_off: self.threshold_off_spinbox.setValue(v),
            )
        else:
            self._diag_title.setText("⚠  Pedal holds too long or too sparse")
            suggested_off = max(0.010, min(
                round(curr_on - 0.02, 3),
                round(curr_off + 0.05, 3),
            ))
            self._add_diag_row(
                f"  Raise Off Threshold to {suggested_off:.3f}  (releases more easily)",
                lambda v=suggested_off: self.threshold_off_spinbox.setValue(v),
            )

        self._add_diag_row(
            "  Regenerate with fresh auto-thresholds",
            self.generate_requested.emit,
        )
        self._diag_widget.setVisible(True)

    def _clear_diag_rows(self) -> None:
        while self._diag_rows_layout.count():
            item = self._diag_rows_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

    def _add_diag_row(self, description: str, action: Callable) -> None:
        btn = QPushButton(f"→  {description}")
        btn.setProperty("variant", "diag_action")
        btn.clicked.connect(lambda _: action())
        self._diag_rows_layout.addWidget(btn)

    def reset(self) -> None:
        """Return the card to the pre-generate state. Call on every song change."""
        self._has_thresholds = False
        self._default_on = 0.5
        self._default_off = 0.5
        self.threshold_on_spinbox.blockSignals(True)
        self.threshold_off_spinbox.blockSignals(True)
        self.threshold_on_spinbox.setValue(0.5)
        self.threshold_off_spinbox.setValue(0.5)
        self.threshold_on_spinbox.blockSignals(False)
        self.threshold_off_spinbox.blockSignals(False)
        self.threshold_on_spinbox.setEnabled(False)
        self.threshold_off_spinbox.setEnabled(False)
        self._stat_avg.setText("--")
        self._stat_min.setText("--")
        self._stat_max.setText("--")
        self._stat_freq.setText("--")
        self._clear_diag_rows()
        self._diag_widget.setVisible(False)
        self.reset_icon.setVisible(False)
        self._stack.setCurrentIndex(0)
        self.generate_btn.setEnabled(False)

    def set_generate_enabled(self, enabled: bool) -> None:
        """Enable or disable the Generate button (used during compilation)."""
        self.generate_btn.setEnabled(enabled)

    def set_spinboxes_enabled(self, enabled: bool) -> None:
        """Enable or disable spinboxes while preserving the has_thresholds flag.

        Used by PlaybackTab.set_groups_enabled to lock controls during playback
        and restore them (only if thresholds exist) afterward.
        """
        if enabled and not self._has_thresholds:
            return
        self.threshold_on_spinbox.setEnabled(enabled)
        self.threshold_off_spinbox.setEnabled(enabled)
