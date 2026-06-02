"""Reusable UI widgets for HuMidi.

This package was split out of the former monolithic ``ui/widgets.py``.
Public names are re-exported here so existing ``from ui.widgets import X``
imports continue to work unchanged.
"""

from ui.widgets.nav_button import NavButton
from ui.widgets.discord_button import DiscordNavButton
from ui.widgets.sub_tab_bar import SubTabBar
from ui.widgets.file_strip import FileStrip
from ui.widgets.section_card import make_card
from ui.widgets.midi_drop_zone import MidiDropZone
from ui.widgets.humidi_button import HuMidiButton
from ui.widgets.clickable_save_card import ClickableSaveCard
from ui.widgets.stats_tile import StatsTile
from ui.widgets.part_card import PartCard
from ui.widgets.loaded_row import LoadedRow
from ui.widgets.saved_songs_panel import SavedSongsPanel
from ui.widgets.performance_card import PerformanceCard
from ui.widgets.options_card import OptionsCard
from ui.widgets.humanize_master_row import HumanizeMasterRow
from ui.widgets.hum_row import HumRow

__all__ = [
    "NavButton",
    "DiscordNavButton",
    "SubTabBar",
    "FileStrip",
    "make_card",
    "MidiDropZone",
    "HuMidiButton",
    "ClickableSaveCard",
    "StatsTile",
    "PartCard",
    "LoadedRow",
    "SavedSongsPanel",
    "PerformanceCard",
    "OptionsCard",
    "HumanizeMasterRow",
    "HumRow",
]
