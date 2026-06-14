from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QFrame, QStackedWidget, QScrollArea, QTextEdit, QSizePolicy)
from PyQt6.QtCore import Qt, pyqtSignal as Signal

from ui.widgets import make_card
from ui.widgets.toggle_switch import ToggleSwitch
from ui.widgets.ph_icon_label import PhIconLabel
from ui.playback.sub_tab_bar import SubTabBar
from ui.playback.file_strip import FileStrip
from ui.playback.midi_drop_zone import MidiDropZone
from ui.playback.loaded_row import LoadedRow
from ui.playback.saved_songs_panel import SavedSongsPanel
from ui.playback.performance_card import PerformanceCard
from ui.playback.options_card import OptionsCard
from ui.playback.humanize_master_row import HumanizeMasterRow
from ui.playback.hum_row import HumRow


class PlaybackTab(QWidget):

    edit_selection_requested = Signal()
    save_card_clicked = Signal(str, str, str)  # (filepath, save_name, song_name)

    # Re-exported from PerformanceCard for backward-compatible callers.
    PEDAL_MAPPING     = PerformanceCard.PEDAL_MAPPING
    PEDAL_MAPPING_INV = PerformanceCard.PEDAL_MAPPING_INV

    def __init__(self, parent=None):
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        # Persistent file info strip
        self.file_strip = FileStrip()
        outer.addWidget(self.file_strip)

        # Sub-tab bar (I-III)
        self._sub_tab_bar = SubTabBar()
        self._sub_tab_bar.tab_changed.connect(self._on_sub_tab_changed)
        outer.addWidget(self._sub_tab_bar)

        # Stacked content pages
        self._stack = QStackedWidget()
        outer.addWidget(self._stack, 1)

        self._stack.addWidget(self._scrollable(self._build_file_tab()))        # I
        self._stack.addWidget(self._scrollable(self._build_playback_tab())) # II
        self._stack.addWidget(self._scrollable(self._build_humanize_tab())) # III

    def _on_sub_tab_changed(self, index: int) -> None:
        self._stack.setCurrentIndex(index)

    @staticmethod
    def _scrollable(widget: QWidget) -> QScrollArea:
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        scroll.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAsNeeded)
        scroll.setWidget(widget)
        scroll.setFrameShape(QFrame.Shape.NoFrame)
        return scroll

    # -- Tab I: File ----------------------------------------------------------

    def _build_file_tab(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(12, 12, 12, 12)
        layout.setSpacing(10)

        # Row 1: LOADED card containing the LoadedRow widget.
        tracks_card, tracks_layout = make_card("LOADED")
        tracks_card.setMinimumHeight(100)
        self._loaded_row = LoadedRow()
        self._loaded_row.edit_selection_btn.clicked.connect(
            self.edit_selection_requested.emit
        )
        # Proxy for backward-compatible attribute access on PlaybackTab.
        self.edit_selection_btn = self._loaded_row.edit_selection_btn
        tracks_layout.addWidget(self._loaded_row)
        layout.addWidget(tracks_card)

        # Row 2: drop zone on the left, saved songs panel on the right.
        cols = QHBoxLayout()
        cols.setSpacing(14)

        self.drop_zone = MidiDropZone()
        self.browse_button  = self.drop_zone.browse_button
        self.load_saved_btn = self.drop_zone.load_saved_btn
        self._drop_card, drop_body = make_card("REPLACE", dashed_border=True)
        drop_body.addWidget(self.drop_zone)

        # file_path_label kept for API compat -- stores full path in toolTip.
        self.file_path_label = QLabel("No file selected.")
        self.file_path_label.setObjectName("file_path_label")
        self.file_path_label.setVisible(False)

        self._saved_panel = SavedSongsPanel()
        self._saved_panel.save_card_clicked.connect(self.save_card_clicked.emit)
        # Proxy attributes for MainWindow._bind_signals.
        self.all_saves_btn          = self._saved_panel.all_saves_btn
        self.refresh_saved_songs_btn = self._saved_panel.refresh_saved_songs_btn

        cols.addWidget(self._drop_card, 1)
        cols.addWidget(self._saved_panel, 1)
        layout.addLayout(cols, 1)
        return page

    # -- Tab II: Playback (includes Mapping and Activity) ---------------------

    def _build_playback_tab(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(12, 12, 12, 12)
        layout.setSpacing(10)

        # Row 1: two equal columns
        row1 = QHBoxLayout()
        row1.setSpacing(14)

        # Left column: PERFORMANCE card stretched to fill column height.
        self._perf_card = PerformanceCard()
        self._perf_card.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )
        self.tempo_slider      = self._perf_card.tempo_slider
        self.tempo_spinbox     = self._perf_card.tempo_spinbox
        self.pedal_style_combo = self._perf_card.pedal_style_combo
        self.transpose_spinbox = self._perf_card.transpose_spinbox
        self.perf_reset_icon   = self._perf_card.reset_icon

        # Right column: OPTIONS card (includes auto-detect hands).
        self._opts_card = OptionsCard()
        self._opts_card.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )
        self.use_88_key_check         = self._opts_card.use_88_key_check
        self.countdown_check          = self._opts_card.countdown_check
        self.debug_check              = self._opts_card.debug_check
        self._auto_detect_hands_check = self._opts_card.auto_detect_hands_check
        self.opts_reset_icon          = self._opts_card.reset_icon

        row1.addWidget(self._perf_card, 1)
        row1.addWidget(self._opts_card, 1)

        # Row 2: ACTIVITY log spanning full width, fills remaining height.
        act_card, act_layout = make_card("ACTIVITY")
        self.activity_log = QTextEdit()
        self.activity_log.setObjectName("activity_log")
        self.activity_log.setReadOnly(True)
        self.activity_log.setFontFamily("JetBrains Mono")
        self.activity_log.setFontPointSize(8)
        self.activity_log.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )
        self.activity_log.setMinimumHeight(60)
        act_layout.addWidget(self.activity_log, 1)
        act_card.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )

        layout.addLayout(row1)
        layout.addWidget(act_card, 1)
        return page

    # -- Tab III: Humanize ----------------------------------------------------

    def _build_humanize_tab(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(12, 12, 12, 12)
        layout.setSpacing(10)

        self.all_humanization_checks    = {}
        self.all_humanization_spinboxes = {}
        self.all_humanization_sliders   = {}

        # Master row card
        self._humanize_master = HumanizeMasterRow()
        self.select_all_humanization_check = (
            self._humanize_master.select_all_humanization_check
        )
        self.humanize_reset_icon = self._humanize_master.reset_icon
        self.all_humanization_checks['simulate_hands']   = (
            self._humanize_master.simulate_hands_check
        )
        self.all_humanization_checks['enable_chord_roll'] = (
            self._humanize_master.enable_chord_roll_check
        )
        layout.addWidget(self._humanize_master)

        # Stacked detail cards
        cols = QVBoxLayout()
        cols.setSpacing(10)

        # Left: Timing & Feel
        self.timing_reset_icon = PhIconLabel("arrow-counter-clockwise", size=16)
        self.timing_reset_icon.setToolTip("Reset timing & feel to defaults")
        self.timing_reset_icon.setCursor(Qt.CursorShape.PointingHandCursor)
        self.timing_reset_icon.clicked.connect(self._reset_timing_to_default)
        left_card, left_layout = make_card("TIMING & FEEL", title_buttons=[self.timing_reset_icon])
        self._add_hum_row(left_layout, "Vary Timing",       "vary_timing",
                          0, 0.1, 0.01, " s",  factor=10000.0,
                          tooltip="Add random timing offsets to note events (in seconds)",
                          desc="nudges each note slightly off the exact beat, like a real player")
        self._add_hum_row(left_layout, "Vary Articulation", "vary_articulation",
                          50, 100, 95,  "%",   factor=100.0, decimals=1,
                          tooltip="Randomize note hold duration; lower values create a more staccato feel",
                          desc="randomly shortens or lengthens how long each note is held down")
        self._add_hum_row(left_layout, "Tempo Sway",        "tempo_sway",
                          0, 0.1, 0,   " s",  factor=10000.0,
                          tooltip="Apply a sinusoidal tempo variation across the song for a more expressive feel",
                          desc="gently speeds up then slows down the tempo in a wave across the song")
        self.invert_sway_check = ToggleSwitch("Invert Sway")
        self.invert_sway_check.setToolTip("Invert the phase of the tempo sway curve")
        self.all_humanization_checks['invert_tempo_sway'] = self.invert_sway_check
        self.all_humanization_checks['tempo_sway'].toggled.connect(
            self.invert_sway_check.setEnabled
        )
        _invert_container = QWidget()
        _invert_vbox = QVBoxLayout(_invert_container)
        _invert_vbox.setContentsMargins(0, 0, 0, 0)
        _invert_vbox.setSpacing(1)
        _invert_vbox.addWidget(self.invert_sway_check)
        _invert_desc = QLabel("flips the sway so the tempo slows down first, then speeds back up")
        _invert_desc.setProperty("role", "muted")
        _invert_desc.setContentsMargins(36, 0, 0, 0)
        _invert_vbox.addWidget(_invert_desc)
        left_layout.addWidget(_invert_container)
        left_layout.addStretch()

        # Right: Hands & Imperfection
        self.hands_reset_icon = PhIconLabel("arrow-counter-clockwise", size=16)
        self.hands_reset_icon.setToolTip("Reset hands & imperfection to defaults")
        self.hands_reset_icon.setCursor(Qt.CursorShape.PointingHandCursor)
        self.hands_reset_icon.clicked.connect(self._reset_hands_to_default)
        right_card, right_layout = make_card("HANDS & IMPERFECTION", title_buttons=[self.hands_reset_icon])
        self._add_hum_row(right_layout, "Hand Drift",   "hand_drift",
                          0, 100, 25, "%", factor=100.0, decimals=1,
                          tooltip="Simulate gradual timing drift between the left and right hands",
                          desc="gradually shifts one hand's timing ahead or behind the other over time")
        self._add_hum_row(right_layout, "Mistakes",     "mistake_chance",
                          0, 10,   0, "%", factor=100.0, decimals=1,
                          tooltip="Randomly skip notes to simulate human errors",
                          desc="randomly drops a note here and there, like a real player slipping up")
        right_layout.addStretch()

        cols.addWidget(left_card)
        cols.addWidget(right_card)
        layout.addLayout(cols)

        # Dummy vary_velocity entry for legacy save compatibility.
        self.all_humanization_checks['vary_velocity'] = ToggleSwitch()

        self.select_all_humanization_check.toggled.connect(self._toggle_all)
        for check in self.all_humanization_checks.values():
            if check.text():
                check.toggled.connect(self._update_select_all_state)

        layout.addStretch()
        return page

    def _add_hum_row(self, parent_layout, name, key, min_val, max_val, def_val,
                     suffix, factor=1.0, decimals=3, tooltip="", desc=""):
        """Create a HumRow, add it to parent_layout, and register it in the dicts."""
        row = HumRow(name, min_val, max_val, def_val, suffix,
                     factor=factor, decimals=decimals, tooltip=tooltip, desc=desc)
        parent_layout.addWidget(row)
        self.all_humanization_checks[key]    = row.check
        self.all_humanization_sliders[key]   = row.slider
        self.all_humanization_spinboxes[key] = row.spinbox

    # -- Humanization helpers -------------------------------------------------

    def _toggle_all(self, checked: bool) -> None:
        for check in self.all_humanization_checks.values():
            if check.text():
                check.setChecked(checked)

    def _update_select_all_state(self) -> None:
        checks = [c for c in self.all_humanization_checks.values() if c.text()]
        self.select_all_humanization_check.blockSignals(True)
        self.select_all_humanization_check.setChecked(all(c.isChecked() for c in checks))
        self.select_all_humanization_check.blockSignals(False)

    # -- Public API -----------------------------------------------------------

    def update_file_label(self, text: str, tooltip: str = "") -> None:
        self.file_path_label.setText(text)
        self.file_path_label.setToolTip(tooltip)
        self.file_strip.update_file(text)

    def log_activity(self, msg: str) -> None:
        self.activity_log.append(msg)

    def update_loaded_summary(self, parts: list, pedal_count: int) -> None:
        """Refresh the LOADED row from live (MidiTrack, role) pairs."""
        self._loaded_row.update_loaded_summary(parts, pedal_count)

    def update_loaded_summary_from_save(self, track_details: list, pedal_count: int) -> None:
        """Populate the LOADED row from save-file metadata dicts."""
        self._loaded_row.update_loaded_summary_from_save(track_details, pedal_count)

    def clear_loaded_summary(self) -> None:
        """Reset the LOADED row to the empty placeholder state."""
        self._loaded_row.clear_loaded_summary()

    def refresh_saved_songs(self, save_dir) -> None:
        """Rescan save_dir and redraw the saved songs list."""
        self._saved_panel.refresh_saved_songs(save_dir)

    def set_groups_enabled(self, enabled: bool, skip_playback_humanization: bool = False) -> None:
        self.file_strip.setEnabled(enabled)
        if not skip_playback_humanization:
            self.tempo_slider.setEnabled(enabled)
            self.tempo_spinbox.setEnabled(enabled)
            self.pedal_style_combo.setEnabled(enabled)
            self.transpose_spinbox.setEnabled(enabled)
            self.use_88_key_check.setEnabled(enabled)
            self.countdown_check.setEnabled(enabled)
            self.debug_check.setEnabled(enabled)
            self.select_all_humanization_check.setEnabled(enabled)
            for w in list(self.all_humanization_checks.values()) + \
                     list(self.all_humanization_sliders.values()) + \
                     list(self.all_humanization_spinboxes.values()):
                w.setEnabled(enabled)

    def update_enabled_states(self) -> None:
        for key, check in self.all_humanization_checks.items():
            if not check.text():
                continue
            checked = check.isChecked()
            if key in self.all_humanization_sliders:
                self.all_humanization_sliders[key].setEnabled(checked)
            if key in self.all_humanization_spinboxes:
                self.all_humanization_spinboxes[key].setEnabled(checked)
        self.invert_sway_check.setEnabled(
            self.all_humanization_checks['tempo_sway'].isChecked()
        )

    def reset_to_default(self) -> None:
        self._perf_card.reset_to_default()
        self._opts_card.reset_to_default()
        self._humanize_master.reset_to_default()
        self._reset_timing_to_default()
        self._reset_hands_to_default()
        self.update_enabled_states()

    def _reset_timing_to_default(self) -> None:
        self.all_humanization_checks['vary_timing'].setChecked(False)
        self.all_humanization_spinboxes['vary_timing'].setValue(0.010)
        self.all_humanization_checks['vary_articulation'].setChecked(False)
        self.all_humanization_spinboxes['vary_articulation'].setValue(95.0)
        self.all_humanization_checks['tempo_sway'].setChecked(False)
        self.all_humanization_spinboxes['tempo_sway'].setValue(0.015)
        self.all_humanization_checks['invert_tempo_sway'].setChecked(False)
        self.update_enabled_states()

    def _reset_hands_to_default(self) -> None:
        self.all_humanization_checks['hand_drift'].setChecked(False)
        self.all_humanization_spinboxes['hand_drift'].setValue(25.0)
        self.all_humanization_checks['mistake_chance'].setChecked(False)
        self.all_humanization_spinboxes['mistake_chance'].setValue(0.5)
        self.update_enabled_states()

    def redraw_saved_song_cards(self) -> None:
        """Redraw saved song cards from cache with the current theme color."""
        self._saved_panel.redraw_cards()

    def load_config(self, config: dict) -> None:
        self.tempo_spinbox.setValue(config.get('tempo', 100.0))
        self.transpose_spinbox.setValue(config.get('transpose', 0))
        display = self.PEDAL_MAPPING_INV.get(
            config.get('pedal_style', 'hybrid'), "Auto (Default)"
        )
        self.pedal_style_combo.setCurrentText(display)
        self.use_88_key_check.setChecked(config.get('use_88_key_layout', False))
        self.countdown_check.setChecked(config.get('countdown', True))
        self.debug_check.setChecked(config.get('debug_mode', False))
        self.select_all_humanization_check.setChecked(
            config.get('select_all_humanization', False)
        )
        self.all_humanization_checks['simulate_hands'].setChecked(
            config.get('simulate_hands', False)
        )
        self.all_humanization_checks['enable_chord_roll'].setChecked(
            config.get('enable_chord_roll', False)
        )
        self.all_humanization_checks['vary_timing'].setChecked(
            config.get('enable_vary_timing', False)
        )
        self.all_humanization_spinboxes['vary_timing'].setValue(
            config.get('value_timing_variance', 0.010)
        )
        self.all_humanization_checks['vary_articulation'].setChecked(
            config.get('enable_vary_articulation', False)
        )
        self.all_humanization_spinboxes['vary_articulation'].setValue(
            config.get('value_articulation', 95.0)
        )
        self.all_humanization_checks['hand_drift'].setChecked(
            config.get('enable_hand_drift', False)
        )
        self.all_humanization_spinboxes['hand_drift'].setValue(
            config.get('value_hand_drift_decay', 25.0)
        )
        self.all_humanization_checks['mistake_chance'].setChecked(
            config.get('enable_mistakes', False)
        )
        self.all_humanization_spinboxes['mistake_chance'].setValue(
            config.get('value_mistake_chance', 0.5)
        )
        self.all_humanization_checks['tempo_sway'].setChecked(
            config.get('enable_tempo_sway', False)
        )
        self.all_humanization_spinboxes['tempo_sway'].setValue(
            config.get('value_tempo_sway_intensity', 0.015)
        )
        self.all_humanization_checks['invert_tempo_sway'].setChecked(
            config.get('invert_tempo_sway', False)
        )
        self.update_enabled_states()

    def gather_playback_config(self) -> dict:
        internal = self.PEDAL_MAPPING.get(
            self.pedal_style_combo.currentText(), 'hybrid'
        )
        return {
            'midi_file':               self.file_path_label.toolTip(),
            'tempo':                   self.tempo_spinbox.value(),
            'transpose':               self.transpose_spinbox.value(),
            'countdown':               self.countdown_check.isChecked(),
            'use_88_key_layout':       self.use_88_key_check.isChecked(),
            'pedal_style':             internal,
            'debug_mode':              self.debug_check.isChecked(),
            'simulate_hands':          self.all_humanization_checks['simulate_hands'].isChecked(),
            'vary_velocity':           False,
            'enable_chord_roll':       self.all_humanization_checks['enable_chord_roll'].isChecked(),
            'vary_timing':             self.all_humanization_checks['vary_timing'].isChecked(),
            'timing_variance':         self.all_humanization_spinboxes['vary_timing'].value(),
            'vary_articulation':       self.all_humanization_checks['vary_articulation'].isChecked(),
            'articulation':            self.all_humanization_spinboxes['vary_articulation'].value() / 100.0,
            'enable_drift_correction': self.all_humanization_checks['hand_drift'].isChecked(),
            'drift_decay_factor':      self.all_humanization_spinboxes['hand_drift'].value() / 100.0,
            'enable_mistakes':         self.all_humanization_checks['mistake_chance'].isChecked(),
            'mistake_chance':          self.all_humanization_spinboxes['mistake_chance'].value(),
            'enable_tempo_sway':       self.all_humanization_checks['tempo_sway'].isChecked(),
            'tempo_sway_intensity':    self.all_humanization_spinboxes['tempo_sway'].value(),
            'invert_tempo_sway':       self.all_humanization_checks['invert_tempo_sway'].isChecked(),
        }

    def gather_app_config(self) -> dict:
        internal = self.PEDAL_MAPPING.get(
            self.pedal_style_combo.currentText(), 'hybrid'
        )
        return {
            'tempo':                      self.tempo_spinbox.value(),
            'transpose':                  self.transpose_spinbox.value(),
            'pedal_style':                internal,
            'use_88_key_layout':          self.use_88_key_check.isChecked(),
            'countdown':                  self.countdown_check.isChecked(),
            'debug_mode':                 self.debug_check.isChecked(),
            'select_all_humanization':    self.select_all_humanization_check.isChecked(),
            'simulate_hands':             self.all_humanization_checks['simulate_hands'].isChecked(),
            'enable_chord_roll':          self.all_humanization_checks['enable_chord_roll'].isChecked(),
            'enable_vary_timing':         self.all_humanization_checks['vary_timing'].isChecked(),
            'value_timing_variance':      self.all_humanization_spinboxes['vary_timing'].value(),
            'enable_vary_articulation':   self.all_humanization_checks['vary_articulation'].isChecked(),
            'value_articulation':         self.all_humanization_spinboxes['vary_articulation'].value(),
            'enable_hand_drift':          self.all_humanization_checks['hand_drift'].isChecked(),
            'value_hand_drift_decay':     self.all_humanization_spinboxes['hand_drift'].value(),
            'enable_mistakes':            self.all_humanization_checks['mistake_chance'].isChecked(),
            'value_mistake_chance':       self.all_humanization_spinboxes['mistake_chance'].value(),
            'enable_tempo_sway':          self.all_humanization_checks['tempo_sway'].isChecked(),
            'value_tempo_sway_intensity': self.all_humanization_spinboxes['tempo_sway'].value(),
            'invert_tempo_sway':          self.all_humanization_checks['invert_tempo_sway'].isChecked(),
        }
