from PyQt6.QtWidgets import QWidget, QVBoxLayout, QCheckBox, QLabel
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

        self._reset_btn = HuMidiButton(tooltip="Reset options to defaults")
        self._reset_btn.setProperty("role", "card_reset")
        self._reset_btn.setFixedSize(28, 28)
        self._reset_btn.clicked.connect(self.reset_to_default)

        card, body = make_card("OPTIONS", title_buttons=[self._reset_btn])

        self.use_88_key_check = QCheckBox("88-Key Layout")
        self.use_88_key_check.setToolTip(
            "Map notes to the full 88-key piano layout instead of a compressed keyboard layout"
        )
        self.countdown_check = QCheckBox("Countdown")
        self.countdown_check.setToolTip("Show a 3-second countdown before playback begins")
        self.debug_check = QCheckBox("Debug Output")
        self.debug_check.setToolTip(
            "Print verbose event logs to the Debug tab during playback"
        )
        self.auto_detect_hands_check = QCheckBox("Auto-detect hands")
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

    def update_icon_color(self, color: str) -> None:
        _logical = 14
        _pix = ph_icon("arrow-counter-clockwise", color, _logical).pixmap(_logical * 2, _logical * 2)
        _pix.setDevicePixelRatio(2.0)
        self._reset_btn.setIcon(QIcon(_pix))
        self._reset_btn.setIconSize(QSize(_logical, _logical))
