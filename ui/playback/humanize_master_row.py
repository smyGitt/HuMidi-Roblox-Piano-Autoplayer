from PyQt6.QtWidgets import QWidget, QVBoxLayout, QLabel
from PyQt6.QtCore import Qt

from ui.widgets.section_card import make_card
from ui.widgets.ph_icon_label import PhIconLabel
from ui.widgets.toggle_switch import ToggleSwitch


def _make_check_pair(checkbox: ToggleSwitch, desc_text: str) -> QWidget:
    container = QWidget()
    vbox = QVBoxLayout(container)
    vbox.setContentsMargins(0, 0, 0, 0)
    vbox.setSpacing(1)
    desc = QLabel(desc_text)
    desc.setProperty("variant", "muted")
    desc.setContentsMargins(36, 0, 0, 0)
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

        self.reset_icon = PhIconLabel("arrow-counter-clockwise", size=16)
        self.reset_icon.setToolTip("Reset humanize master options to defaults")
        self.reset_icon.setCursor(Qt.CursorShape.PointingHandCursor)
        self.reset_icon.clicked.connect(self.reset_to_default)

        card, body = make_card("GENERAL SETTINGS", title_buttons=[self.reset_icon])

        self.select_all_humanization_check = ToggleSwitch("Humanize all")
        self.select_all_humanization_check.setToolTip(
            "Enable or disable all humanization options at once"
        )
        self.simulate_hands_check = ToggleSwitch("Simulate Hands")
        self.simulate_hands_check.setToolTip(
            "Assign notes to left/right hand and limit simultaneous finger usage "
            "to simulate realistic hand behavior"
        )
        self.enable_chord_roll_check = ToggleSwitch("Chord Roll")
        self.enable_chord_roll_check.setToolTip(
            "Slightly stagger the notes within each chord to simulate the natural "
            "roll of fingers across the keys"
        )

        body.addWidget(_make_check_pair(
            self.select_all_humanization_check,
            "turn all humanization options on or off at once",
        ))
        body.addWidget(_make_check_pair(
            self.simulate_hands_check,
            "limits notes per hand to what a real pianist could physically reach",
        ))
        body.addWidget(_make_check_pair(
            self.enable_chord_roll_check,
            "rolls simultaneous notes slightly apart, like fingers naturally landing one after another",
        ))

        outer.addWidget(card)

    def reset_to_default(self) -> None:
        self.simulate_hands_check.setChecked(False)
        self.enable_chord_roll_check.setChecked(False)

