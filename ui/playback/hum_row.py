from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QGridLayout, QDoubleSpinBox, QSlider, QLabel)
from PyQt6.QtCore import Qt

from ui.widgets.toggle_switch import ToggleSwitch


class HumRow(QWidget):
    """One humanization parameter row: a named checkbox, a value spinbox, and a slider.

    The checkbox controls whether the slider and spinbox are enabled (wired
    internally). Callers register check, slider, and spinbox in PlaybackTab's
    all_humanization_checks / all_humanization_sliders / all_humanization_spinboxes
    dicts under the parameter's key string.
    """

    def __init__(self, name, min_val, max_val, def_val, suffix,
                 factor=1.0, decimals=3, tooltip="", desc="", parent=None):
        super().__init__(parent)
        self._setup_ui(name, min_val, max_val, def_val, suffix, factor, decimals, tooltip, desc)

    def _setup_ui(self, name, min_val, max_val, def_val, suffix, factor, decimals, tooltip, desc):
        self.check = ToggleSwitch(name)
        self.slider, self.spinbox = self._make_slider_spinbox(
            min_val, max_val, def_val, suffix, factor=factor, decimals=decimals
        )
        self.spinbox.setFixedWidth(80)
        self.check.toggled.connect(self.slider.setEnabled)
        self.check.toggled.connect(self.spinbox.setEnabled)
        if tooltip:
            self.check.setToolTip(tooltip)
            self.slider.setToolTip(tooltip)
            self.spinbox.setToolTip(tooltip)

        grid = QGridLayout(self)
        grid.setContentsMargins(0, 0, 0, 0)
        grid.setHorizontalSpacing(8)
        grid.setVerticalSpacing(3)
        grid.setColumnStretch(1, 1)

        if desc:
            check_container = QWidget()
            check_vbox = QVBoxLayout(check_container)
            check_vbox.setContentsMargins(0, 0, 0, 0)
            check_vbox.setSpacing(1)
            check_vbox.addWidget(self.check)
            desc_label = QLabel(desc)
            desc_label.setProperty("role", "muted")
            desc_label.setContentsMargins(36, 0, 0, 0)
            check_vbox.addWidget(desc_label)
            grid.addWidget(check_container, 0, 0)
        else:
            grid.addWidget(self.check, 0, 0)

        grid.addWidget(self.spinbox, 0, 2)
        grid.addWidget(self.slider,  1, 0, 1, 3)

    @staticmethod
    def _make_slider_spinbox(min_val, max_val, default_val,
                             text_suffix="", factor=10000.0, decimals=4):
        slider = QSlider(Qt.Orientation.Horizontal)
        slider.setRange(int(min_val * factor), int(max_val * factor))
        spinbox = QDoubleSpinBox()
        spinbox.setDecimals(decimals)
        spinbox.setRange(-2147483648, 2147483647)
        spinbox.setSingleStep(1.0 / factor)
        spinbox.setSuffix(text_suffix)
        slider.setValue(int(default_val * factor))
        spinbox.setValue(default_val)
        slider.valueChanged.connect(lambda v: spinbox.setValue(v / factor))
        spinbox.valueChanged.connect(lambda v: slider.setValue(int(v * factor)))
        return slider, spinbox
