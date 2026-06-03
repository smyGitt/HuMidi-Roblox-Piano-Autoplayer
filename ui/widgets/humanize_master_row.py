from PyQt6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QCheckBox, QFrame, QLabel
from PyQt6.QtCore import QSize
from PyQt6.QtGui import QIcon

from ui.widgets.section_card import make_card
from ui.widgets.humidi_button import HuMidiButton
from ui.widgets.ph_icon import ph_icon


def _make_check_pair(checkbox: QCheckBox, desc_text: str) -> QWidget:
    container = QWidget()
    vbox = QVBoxLayout(container)
    vbox.setContentsMargins(0, 0, 0, 0)
    vbox.setSpacing(1)
    desc = QLabel(desc_text)
    desc.setProperty("role", "muted")
    desc.setContentsMargins(25, 0, 0, 0)
    vbox.addWidget(checkbox)
    vbox.addWidget(desc)
    return container


class HumanizeMasterRow(QWidget):
    """Master card at the top of PlaybackTab's Humanize sub-tab.

    Owns select_all_humanization_check, the vertical separator,
    simulate_hands_check, and enable_chord_roll_check in a single horizontal
    row inside an empty-title section card.

    PlaybackTab aliases these attributes into its own namespace and registers
    simulate_hands_check / enable_chord_roll_check in all_humanization_checks.
    """

    def __init__(self, parent=None):
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        card, body = make_card("")

        master_row = QHBoxLayout()
        master_row.setSpacing(14)

        self.select_all_humanization_check = QCheckBox("Humanize all")
        self.select_all_humanization_check.setToolTip(
            "Enable or disable all humanization options at once"
        )

        v_sep = QFrame()
        v_sep.setObjectName("v_sep")
        v_sep.setFrameShape(QFrame.Shape.VLine)
        v_sep.setFixedWidth(1)

        self.simulate_hands_check = QCheckBox("Simulate Hands")
        self.simulate_hands_check.setToolTip(
            "Assign notes to left/right hand and limit simultaneous finger usage "
            "to simulate realistic hand behavior"
        )
        self.enable_chord_roll_check = QCheckBox("Chord Roll")
        self.enable_chord_roll_check.setToolTip(
            "Slightly stagger the notes within each chord to simulate the natural "
            "roll of fingers across the keys"
        )

        master_row.addWidget(_make_check_pair(
            self.select_all_humanization_check,
            "enable or disable all humanization",
        ))
        master_row.addWidget(v_sep)
        master_row.addWidget(_make_check_pair(
            self.simulate_hands_check,
            "separate timing per hand",
        ))
        master_row.addWidget(_make_check_pair(
            self.enable_chord_roll_check,
            "slight arpeggiation of simultaneous notes",
        ))
        master_row.addStretch()

        self._reset_btn = HuMidiButton(tooltip="Reset humanize master options to defaults")
        self._reset_btn.setProperty("role", "card_reset")
        self._reset_btn.setFixedSize(28, 28)
        self._reset_btn.clicked.connect(self.reset_to_default)
        master_row.addWidget(self._reset_btn)

        body.addLayout(master_row)

        outer.addWidget(card)

    def reset_to_default(self) -> None:
        self.simulate_hands_check.setChecked(False)
        self.enable_chord_roll_check.setChecked(False)

    def update_icon_color(self, color: str) -> None:
        _logical = 14
        _pix = ph_icon("arrow-counter-clockwise", color, _logical).pixmap(_logical * 2, _logical * 2)
        _pix.setDevicePixelRatio(2.0)
        self._reset_btn.setIcon(QIcon(_pix))
        self._reset_btn.setIconSize(QSize(_logical, _logical))
