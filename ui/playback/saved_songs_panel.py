import json
import os
from datetime import datetime

from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QFrame, QScrollArea, QLabel, QSizePolicy
)
from PySide6.QtCore import Qt, Signal

from ui.playback.clickable_save_card import ClickableSaveCard
from ui.widgets.ph_icon_label import PhIconLabel
from ui.widgets.section_card import make_card


class SavedSongsPanel(QWidget):
    """Full SAVED SONGS card column for PlaybackTab's File sub-tab.

    Owns the card frame, scroll area, inner panel, save-cards list layout,
    the refresh button, and the All Saves button. Scans the save directory on
    request and renders up to _SAVE_CARD_MAX ClickableSaveCard entries.

    Signals:
        save_card_clicked(filepath, save_name, song_name): emitted when any
            save card is clicked; PlaybackTab re-emits as its own signal.

    Expose all_saves_btn and refresh_saved_songs_btn as proxy attributes so
    MainWindow can wire them without reaching into the panel's internals.
    """

    save_card_clicked = Signal(str, str, str)  # (filepath, save_name, song_name)

    _SAVE_CARD_MAX = 15

    def __init__(self, parent=None):
        super().__init__(parent)
        self._save_dir = None
        self._saves_cache: list = []
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        _SIDE_PAD = 14

        self.refresh_saved_songs_btn = PhIconLabel("arrows-clockwise", size=16)
        self.refresh_saved_songs_btn.setToolTip("Refresh saved songs list")
        self.refresh_saved_songs_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self.refresh_saved_songs_btn.clicked.connect(
            lambda: self.refresh_saved_songs(self._save_dir)
        )

        self.all_saves_btn = PhIconLabel("list-magnifying-glass", size=16)
        self.all_saves_btn.setToolTip(
            "Open the save browser to rename, delete, or load any save"
        )
        self.all_saves_btn.setCursor(Qt.CursorShape.PointingHandCursor)

        saved_card, saved_layout = make_card(
            "SAVED SONGS",
            title_buttons=[self.refresh_saved_songs_btn, self.all_saves_btn],
            outer_margins=(0, 6, 0, 7),
            row_h_pad=_SIDE_PAD,
        )

        self._saved_songs_panel = QFrame()
        self._saved_songs_panel.setObjectName("saved_songs_list_panel")
        self._saved_songs_panel.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self._saved_songs_list_layout = QVBoxLayout(self._saved_songs_panel)
        self._saved_songs_list_layout.setContentsMargins(_SIDE_PAD, 8, _SIDE_PAD, 8)
        self._saved_songs_list_layout.setSpacing(4)

        self._saved_songs_scroll = QScrollArea()
        self._saved_songs_scroll.setObjectName("saved_songs_scroll")
        self._saved_songs_scroll.setWidgetResizable(True)
        self._saved_songs_scroll.setHorizontalScrollBarPolicy(
            Qt.ScrollBarPolicy.ScrollBarAlwaysOff
        )
        self._saved_songs_scroll.setVerticalScrollBarPolicy(
            Qt.ScrollBarPolicy.ScrollBarAsNeeded
        )
        self._saved_songs_scroll.setFrameShape(QFrame.Shape.NoFrame)
        self._saved_songs_scroll.setWidget(self._saved_songs_panel)
        saved_layout.addWidget(self._saved_songs_scroll, 1)

        outer.addWidget(saved_card, 1)

        # Seed the empty-state placeholder before any external refresh.
        self.refresh_saved_songs(None)

    def refresh_saved_songs(self, save_dir) -> None:
        """Rescan save_dir for .json saves, update cache, and redraw.

        Pass None or a missing directory to render the empty-state placeholder.
        """
        self._save_dir = save_dir
        self._saves_cache = []

        if save_dir and os.path.isdir(save_dir):
            for filename in os.listdir(save_dir):
                if not filename.endswith('.json'):
                    continue
                filepath = os.path.join(save_dir, filename)
                try:
                    with open(filepath, 'r') as f:
                        data = json.load(f)
                except Exception:
                    continue
                metadata = data.get('metadata', {}) or {}
                created = metadata.get('creation_timestamp', '')
                self._saves_cache.append({
                    'filepath': filepath,
                    'song_name': metadata.get('source_midi_filename', 'Unknown MIDI'),
                    'save_name': metadata.get('custom_name') or os.path.splitext(filename)[0],
                    'created': created,
                    # Legacy saves predate last_accessed; fall back to created.
                    'last_accessed': metadata.get('last_accessed') or created,
                })

        self._saves_cache.sort(key=lambda s: s['last_accessed'], reverse=True)
        self._render_saved_songs()

    def _render_saved_songs(self) -> None:
        """Redraw from cache without hitting disk. Up to _SAVE_CARD_MAX entries."""
        while self._saved_songs_list_layout.count():
            item = self._saved_songs_list_layout.takeAt(0)
            if item.widget():
                item.widget().deleteLater()

        if not self._saves_cache:
            empty = QLabel("No saved songs.")
            empty.setProperty("variant", "muted")
            self._saved_songs_list_layout.addWidget(empty)
            return

        for save in self._saves_cache[:self._SAVE_CARD_MAX]:
            created_str = self._format_save_timestamp(save['created'])
            accessed_str = self._format_save_timestamp(save['last_accessed'])
            time_str = accessed_str or created_str

            card = ClickableSaveCard(
                save['save_name'],
                save['song_name'],
                time_str=time_str,
            )
            card.clicked.connect(
                lambda fp=save['filepath'], sn=save['save_name'], mn=save['song_name']:
                self.save_card_clicked.emit(fp, sn, mn)
            )
            tooltip_lines = [save['song_name'], save['save_name']]
            if created_str:
                tooltip_lines.append(f"Saved: {created_str}")
            if accessed_str and accessed_str != created_str:
                tooltip_lines.append(f"Last opened: {accessed_str}")
            card.setToolTip("\n".join(tooltip_lines))
            card.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Fixed)
            self._saved_songs_list_layout.addWidget(card)
        self._saved_songs_list_layout.addStretch()

    @staticmethod
    def _format_save_timestamp(ts: str) -> str:
        if not ts:
            return ""
        try:
            return datetime.fromisoformat(ts).strftime("%Y-%m-%d %H:%M")
        except Exception:
            return ts
