"""Shared UI widgets for HuMidi.

Only widgets used by more than one tab or by MainWindowUI live here.
Tab-exclusive widgets live alongside their tab (e.g. ui/playback/).
"""

from ui.widgets.nav_button import NavButton
from ui.widgets.discord_button import DiscordNavButton
from ui.widgets.section_card import make_card
from ui.widgets.humidi_button import HuMidiButton
from ui.widgets.elided_label import ElidedLabel
from ui.widgets.status_indicator import StatusIndicator
from ui.widgets.ph_icon_label import PhIconLabel, IconProvider
from ui.widgets.toggle_switch import ToggleSwitch

__all__ = [
    "NavButton",
    "DiscordNavButton",
    "make_card",
    "HuMidiButton",
    "ElidedLabel",
    "StatusIndicator",
    "PhIconLabel",
    "IconProvider",
    "ToggleSwitch",
]
