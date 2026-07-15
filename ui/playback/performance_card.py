from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QGridLayout, QLabel
)
from PySide6.QtCore import Qt

from ui.widgets.section_card import make_card
from ui.widgets.ph_icon_label import PhIconLabel
from ui.widgets.slider_spinbox import NoScrollSpinBox, NoScrollComboBox
from ui.widgets.toggle_switch import ToggleSwitch


class PerformanceCard(QWidget):
    """Left 'PERFORMANCE' card in PlaybackTab's Playback sub-tab.

    Owns pedal_style_combo and transpose_spinbox inside a QGridLayout inside
    the section card. Also owns PEDAL_MAPPING and PEDAL_MAPPING_INV;
    PlaybackTab re-exports them as class constants for backward compatibility.
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

        self.pedal_style_combo = NoScrollComboBox()
        self.pedal_style_combo.addItems(list(self.PEDAL_MAPPING.keys()))
        self.pedal_style_combo.setCurrentText("PedalAI")
        self.pedal_style_combo.setToolTip(
            "Auto (Default): Adaptive hybrid of rhythmic and harmonic analysis\n"
            "AI Pedal: BiLSTM model-generated pedal with adaptive fallback\n"
            "Harmonic: Hold pedal through harmonic regions, releasing at chord/bass changes\n"
            "Rhythmic: Release pedal on beat boundaries only\n"
            "None: No sustain pedal"
        )
        grid.addWidget(self._make_label_pair("Pedal", "generation algorithm"), 0, 0,
                       Qt.AlignmentFlag.AlignVCenter)
        grid.addWidget(self.pedal_style_combo, 0, 2, 1, 2)

        self.transpose_spinbox = NoScrollSpinBox()
        self.transpose_spinbox.setRange(-24, 24)
        self.transpose_spinbox.setValue(0)
        self.transpose_spinbox.setSuffix(" st")
        self.transpose_spinbox.setFixedWidth(72)
        self.transpose_spinbox.setToolTip(
            "Shift all notes up or down by the given number of semitones"
        )
        grid.addWidget(self._make_label_pair("Transpose", "semitones"), 1, 0,
                       Qt.AlignmentFlag.AlignVCenter)
        grid.addWidget(self.transpose_spinbox, 1, 2, 1, 2)

        self._midi_pedal_label = self._make_label_pair("Use MIDI Pedal", "from the loaded file")
        self.use_midi_pedal_check = ToggleSwitch()
        self.use_midi_pedal_check.setChecked(False)
        self.use_midi_pedal_check.setToolTip(
            "When checked, sustain pedal events embedded in the MIDI file are used directly.\n"
            "When unchecked, the pedal generation algorithm is used instead."
        )
        self._midi_pedal_label.setVisible(False)
        self.use_midi_pedal_check.setVisible(False)
        grid.addWidget(self._midi_pedal_label, 2, 0, Qt.AlignmentFlag.AlignVCenter)
        grid.addWidget(self.use_midi_pedal_check, 2, 2, 1, 2)

        self.use_velocity_accent_check = ToggleSwitch()
        self.use_velocity_accent_check.setChecked(False)
        self.use_velocity_accent_check.setToolTip(
            "Some MIDI files record a velocity (how hard each note was struck) per note, "
            "instead of a single flat value for every note. When checked, that recorded "
            "velocity is honored as-is with no filtering or minimum cutoff: every note "
            "with velocity data holds Alt down during its press. This is useful for games "
            "(e.g. Visual Piano) that read a held Alt key as a dynamics accent, so an "
            "expressively-performed MIDI translates into an expressive-sounding playback "
            "instead of a flat one."
        )
        grid.addWidget(self._make_label_pair(
            "Use Velocity", "play recorded note velocity as-is"), 3, 0,
            Qt.AlignmentFlag.AlignVCenter)
        grid.addWidget(self.use_velocity_accent_check, 3, 2, 1, 2)

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
        desc.setProperty("variant", "muted")
        vbox.addWidget(lbl)
        vbox.addWidget(desc)
        return container

    def set_midi_pedal_available(self, available: bool) -> None:
        """Show or hide the 'Use MIDI Pedal' row based on whether the loaded MIDI has CC 64 events.

        Always resets the toggle to unchecked (off by default). LoadCoordinator
        decides whether to check it afterward: automatically for a small
        number of pedal events, or via an explicit Yes/No prompt when the
        file has a significant number of them (see LoadCoordinator._on_midi_parsed).
        """
        self._midi_pedal_label.setVisible(available)
        self.use_midi_pedal_check.setVisible(available)
        self.use_midi_pedal_check.setChecked(False)

    def reset_to_default(self) -> None:
        self.transpose_spinbox.setValue(0)
        self.pedal_style_combo.setCurrentText("PedalAI")
        self.use_velocity_accent_check.setChecked(False)

