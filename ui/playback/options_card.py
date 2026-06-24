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


class OptionsCard(QWidget):
    """Right 'OPTIONS' card in PlaybackTab's Playback sub-tab.

    Owns use_88_key_check, countdown_check, and debug_check.
    PlaybackTab exposes the three checkboxes as direct attributes for use by
    gather_playback_config, load_config, set_groups_enabled, etc.
    """

    def __init__(self, parent=None):
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        self.reset_icon = PhIconLabel("arrow-counter-clockwise", size=16)
        self.reset_icon.setToolTip("Reset options to defaults")
        self.reset_icon.setCursor(Qt.CursorShape.PointingHandCursor)
        self.reset_icon.clicked.connect(self.reset_to_default)

        card, body = make_card("OPTIONS", title_buttons=[self.reset_icon])

        self.use_88_key_check = ToggleSwitch("88-Key Layout")
        self.use_88_key_check.setToolTip(
            "Map notes to the full 88-key piano layout instead of a compressed keyboard layout"
        )
        self.countdown_check = ToggleSwitch("Countdown")
        self.countdown_check.setToolTip("Show a 3-second countdown before playback begins")
        self.debug_check = ToggleSwitch("Debug Output")
        self.debug_check.setToolTip(
            "Print verbose event logs to the Debug tab during playback"
        )
        self.auto_detect_hands_check = ToggleSwitch("Auto-detect hands")
        self.auto_detect_hands_check.setToolTip(
            "Use MIDI track names to assign left/right hand zones automatically"
        )

        body.addWidget(_make_check_pair(self.use_88_key_check,
                                        "Full piano, disable if range is limited"))
        body.addWidget(_make_check_pair(self.countdown_check,
                                        "3 second countdown before starting playback"))
        body.addWidget(_make_check_pair(self.debug_check, "Enable debug output"))
        body.addWidget(_make_check_pair(self.auto_detect_hands_check,
                                        "Auto-detect hand zones from MIDI track names"))
        body.addStretch()

        outer.addWidget(card)

    def reset_to_default(self) -> None:
        self.use_88_key_check.setChecked(False)
        self.countdown_check.setChecked(True)
        self.debug_check.setChecked(False)

