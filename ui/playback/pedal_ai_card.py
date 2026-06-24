from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QGridLayout, QLabel, QDoubleSpinBox,
)
from PyQt6.QtCore import Qt

from ui.widgets.section_card import make_card
from ui.widgets.ph_icon_label import PhIconLabel


class PedalAICard(QWidget):
    """PEDAL AI THRESHOLDS card in PlaybackTab's Playback sub-tab.

    Controls the on/off sigmoid thresholds used by _generate_ai_pedal.
    Starts locked; call set_thresholds() after the first successful AI pedal
    generation to populate and enable the spinboxes. reset_to_default()
    restores the last auto-computed values. Never enabled when loading from
    a save (pre-compiled events skip pedal generation entirely).
    """

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

        card, body = make_card("PEDAL AI THRESHOLDS", title_buttons=[self.reset_icon])

        info_label = QLabel(
            "Editable after first playback with PedalAI enabled.\n"
            "Not available when loading from a save."
        )
        info_label.setProperty("variant", "muted")
        info_label.setWordWrap(True)
        body.addWidget(info_label)
        body.addSpacing(6)

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

        body.addLayout(grid)
        body.addSpacing(6)

        self._status_label = QLabel("")
        self._status_label.setProperty("variant", "muted")
        body.addWidget(self._status_label)
        body.addSpacing(6)

        self.threshold_on_spinbox.valueChanged.connect(self._on_threshold_changed)
        self.threshold_off_spinbox.valueChanged.connect(self._on_threshold_changed)

        stats_title = QLabel("Pedal Stats")
        stats_title.setProperty("variant", "muted")
        body.addWidget(stats_title)
        body.addSpacing(4)

        sg = QGridLayout()
        sg.setVerticalSpacing(6)
        sg.setHorizontalSpacing(16)
        sg.setColumnStretch(1, 1)
        sg.setColumnStretch(3, 1)

        self._stat_avg  = self._add_stat(sg, 0, 0, "Avg hold")
        self._stat_min  = self._add_stat(sg, 0, 2, "Min hold")
        self._stat_max  = self._add_stat(sg, 1, 0, "Max hold")
        self._stat_freq = self._add_stat(sg, 1, 2, "Frequency")

        body.addLayout(sg)
        body.addStretch()

        outer.addWidget(card)

    @staticmethod
    def _add_stat(grid: QGridLayout, row: int, col: int, label: str) -> QLabel:
        name = QLabel(label)
        name.setProperty("variant", "muted")
        val = QLabel("--")
        grid.addWidget(name, row, col)
        grid.addWidget(val, row, col + 1)
        return val

    @staticmethod
    def _make_spinbox(tooltip: str) -> QDoubleSpinBox:
        sb = QDoubleSpinBox()
        sb.setRange(0.0, 1.0)
        sb.setDecimals(3)
        sb.setSingleStep(0.001)
        sb.setFixedWidth(88)
        sb.setButtonSymbols(QDoubleSpinBox.ButtonSymbols.NoButtons)
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

    # -- Private slots --------------------------------------------------------

    def _on_threshold_changed(self) -> None:
        self._status_label.setText("Modified -- re-play to apply")

    # -- Public API -----------------------------------------------------------

    @property
    def has_thresholds(self) -> bool:
        """True once set_thresholds() has been called successfully."""
        return self._has_thresholds

    def set_thresholds(self, on: float, off: float) -> None:
        """Populate controls with auto-computed thresholds and enable editing."""
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
        self._status_label.setText("Applied")

    def reset_to_default(self) -> None:
        """Restore the last applied threshold values."""
        if not self._has_thresholds:
            return
        self.threshold_on_spinbox.blockSignals(True)
        self.threshold_off_spinbox.blockSignals(True)
        self.threshold_on_spinbox.setValue(self._default_on)
        self.threshold_off_spinbox.setValue(self._default_off)
        self.threshold_on_spinbox.blockSignals(False)
        self.threshold_off_spinbox.blockSignals(False)
        self._status_label.setText("Applied")

    def get_threshold_on(self) -> float:
        return self.threshold_on_spinbox.value()

    def get_threshold_off(self) -> float:
        return self.threshold_off_spinbox.value()

    def set_stats(self, avg_dur: float, min_dur: float, max_dur: float, presses_per_min: float) -> None:
        """Populate pedal stats labels with values from the last AI generation."""
        self._stat_avg.setText(f"{avg_dur:.2f} s")
        self._stat_min.setText(f"{min_dur:.2f} s")
        self._stat_max.setText(f"{max_dur:.2f} s")
        self._stat_freq.setText(f"{presses_per_min:.1f}/min")

    def set_spinboxes_enabled(self, enabled: bool) -> None:
        """Enable or disable spinboxes while preserving the has_thresholds flag.

        Used by PlaybackTab.set_groups_enabled to lock controls during playback
        and restore them (only if thresholds exist) afterward.
        """
        if enabled and not self._has_thresholds:
            return
        self.threshold_on_spinbox.setEnabled(enabled)
        self.threshold_off_spinbox.setEnabled(enabled)
