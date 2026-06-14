from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QGridLayout, QLabel,
    QSlider, QDoubleSpinBox, QSpinBox, QComboBox
)
from PyQt6.QtCore import Qt

from ui.widgets.section_card import make_card
from ui.widgets.ph_icon_label import PhIconLabel


class PerformanceCard(QWidget):
    """Left 'PERFORMANCE' card in PlaybackTab's Playback sub-tab.

    Owns tempo_slider, tempo_spinbox, pedal_style_combo, and transpose_spinbox
    inside a QGridLayout inside the section card. Also owns PEDAL_MAPPING and
    PEDAL_MAPPING_INV; PlaybackTab re-exports them as class constants for
    backward compatibility.
    """

    PEDAL_MAPPING = {
        "Auto (Default)": "hybrid",
        "PedalAI":        "ai",
        "Harmonic":       "legato",
        "Rhythmic":       "rhythmic",
        "None":           "none",
    }
    PEDAL_MAPPING_INV = {v: k for k, v in PEDAL_MAPPING.items()}

    def __init__(self, parent=None):
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        self.reset_icon = PhIconLabel("arrow-counter-clockwise", size=16)
        self.reset_icon.setToolTip("Reset performance settings to defaults")
        self.reset_icon.setCursor(Qt.CursorShape.PointingHandCursor)
        self.reset_icon.clicked.connect(self.reset_to_default)

        card, body = make_card("PERFORMANCE", title_buttons=[self.reset_icon])

        grid = QGridLayout()
        grid.setVerticalSpacing(10)
        grid.setHorizontalSpacing(8)
        grid.setColumnMinimumWidth(1, 8)
        grid.setColumnStretch(2, 1)

        self.tempo_slider, self.tempo_spinbox = self._make_slider_spinbox(
            10.0, 200.0, 100.0, "%", factor=10.0, decimals=1
        )
        self.tempo_spinbox.setFixedWidth(72)
        self.tempo_slider.setToolTip("Playback speed as a percentage of the original tempo")
        self.tempo_spinbox.setToolTip("Playback speed as a percentage of the original tempo")
        grid.addWidget(self._make_label_pair("Tempo", "% of original"), 0, 0,
                       Qt.AlignmentFlag.AlignVCenter)
        grid.addWidget(self.tempo_slider,  0, 2, Qt.AlignmentFlag.AlignVCenter)
        grid.addWidget(self.tempo_spinbox, 0, 3, Qt.AlignmentFlag.AlignVCenter)

        self.pedal_style_combo = QComboBox()
        self.pedal_style_combo.addItems(list(self.PEDAL_MAPPING.keys()))
        self.pedal_style_combo.setToolTip(
            "Auto (Default): Adaptive hybrid of rhythmic and harmonic analysis\n"
            "AI Pedal: BiLSTM model-generated pedal with adaptive fallback\n"
            "Harmonic: Hold pedal through harmonic regions, releasing at chord/bass changes\n"
            "Rhythmic: Release pedal on beat boundaries only\n"
            "None: No sustain pedal"
        )
        grid.addWidget(self._make_label_pair("Pedal", "generation algorithm"), 1, 0,
                       Qt.AlignmentFlag.AlignVCenter)
        grid.addWidget(self.pedal_style_combo, 1, 2, 1, 2)

        self.transpose_spinbox = QSpinBox()
        self.transpose_spinbox.setRange(-24, 24)
        self.transpose_spinbox.setValue(0)
        self.transpose_spinbox.setSuffix(" st")
        self.transpose_spinbox.setFixedWidth(72)
        self.transpose_spinbox.setToolTip(
            "Shift all notes up or down by the given number of semitones"
        )
        grid.addWidget(self._make_label_pair("Transpose", "semitones"), 2, 0,
                       Qt.AlignmentFlag.AlignVCenter)
        grid.addWidget(self.transpose_spinbox, 2, 2, 1, 2)

        body.addLayout(grid)
        body.addStretch()

        outer.addWidget(card, 1)

    @staticmethod
    def _make_label_pair(label_text: str, desc_text: str) -> QWidget:
        container = QWidget()
        vbox = QVBoxLayout(container)
        vbox.setContentsMargins(0, 0, 0, 0)
        vbox.setSpacing(1)
        lbl = QLabel(label_text)
        desc = QLabel(desc_text)
        desc.setProperty("role", "muted")
        vbox.addWidget(lbl)
        vbox.addWidget(desc)
        return container

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

    def reset_to_default(self) -> None:
        self.tempo_spinbox.setValue(100.0)
        self.transpose_spinbox.setValue(0)
        self.pedal_style_combo.setCurrentText("Auto (Default)")

