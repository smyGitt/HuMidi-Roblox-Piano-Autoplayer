from PySide6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QFrame, QStackedWidget, QScrollArea, QSizePolicy)
from PySide6.QtCore import Qt, Signal

from ui.widgets import make_card
from ui.widgets.toggle_switch import ToggleSwitch
from ui.widgets.ph_icon_label import PhIconLabel
from ui.widgets.slider_spinbox import make_slider_spinbox, NoScrollDoubleSpinBox
from ui.playback.sub_tab_bar import SubTabBar
from ui.playback.file_strip import FileStrip
from ui.playback.midi_drop_zone import MidiDropZone
from ui.playback.loaded_row import LoadedRow
from ui.playback.saved_songs_panel import SavedSongsPanel
from ui.playback.performance_card import PerformanceCard
from ui.playback.options_card import OptionsCard
from ui.playback.humanize_master_row import HumanizeMasterRow
from ui.playback.hum_row import HumRow
from ui.playback.pedal_ai_card import PedalAICard
from ui.playback.apply_toast import ApplyToast


class PlaybackTab(QWidget):

    edit_selection_requested = Signal()
    save_card_clicked        = Signal(str, str, str)  # (filepath, save_name, song_name)
    apply_requested          = Signal()
    discard_requested        = Signal()
    tab_shown                = Signal()
    config_changed           = Signal()
    generate_pedal_requested = Signal()

    # Re-exported from PerformanceCard for backward-compatible callers.
    PEDAL_MAPPING     = PerformanceCard.PEDAL_MAPPING
    PEDAL_MAPPING_INV = PerformanceCard.PEDAL_MAPPING_INV

    def __init__(self, parent=None):
        super().__init__(parent)
        self._original_bpm: float = 0.0
        self._toast_notes_dirty: bool = False
        self._toast_pedal_dirty: bool = False
        self._restoring: bool = False
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

        self._toast = ApplyToast(self)
        self._toast.apply_clicked.connect(self._on_toast_apply)
        self._toast.discard_clicked.connect(self._on_toast_discard)
        self._wire_config_changed_signals()

    def _on_sub_tab_changed(self, index: int) -> None:
        self._stack.setCurrentIndex(index)

    def _emit_config_changed(self, *_) -> None:
        if not self._restoring:
            self.config_changed.emit()

    def _wire_config_changed_signals(self) -> None:
        emit = self._emit_config_changed
        self.tempo_spinbox.valueChanged.connect(emit)
        self.use_88_key_check.toggled.connect(emit)
        self.pedal_style_combo.currentIndexChanged.connect(emit)
        for check in self.all_humanization_checks.values():
            check.toggled.connect(emit)
        for spinbox in self.all_humanization_spinboxes.values():
            spinbox.valueChanged.connect(emit)
        self._pedal_ai_card.threshold_on_spinbox.valueChanged.connect(emit)
        self._pedal_ai_card.threshold_off_spinbox.valueChanged.connect(emit)
        self.use_midi_pedal_check.toggled.connect(emit)
        self.use_velocity_accent_check.toggled.connect(emit)
        self.velocity_accent_threshold_spinbox.valueChanged.connect(emit)

    def showEvent(self, event) -> None:
        super().showEvent(event)
        self.tab_shown.emit()

    def resizeEvent(self, event) -> None:
        super().resizeEvent(event)
        self._reposition_toast()

    # -- Toast helpers --------------------------------------------------------

    def _reposition_toast(self) -> None:
        if self._toast.is_toast_visible():
            self._toast.resize(self.width(), ApplyToast.HEIGHT)
            self._toast.move(0, self.height() - ApplyToast.HEIGHT)

    def _set_scroll_bottom_pad(self, extra: int) -> None:
        bottom = 12 + extra
        for layout in (
            self._file_page_layout,
            self._playback_page_layout,
            self._humanize_page_layout,
        ):
            m = layout.contentsMargins()
            layout.setContentsMargins(m.left(), m.top(), m.right(), bottom)

    def _on_toast_apply(self) -> None:
        self.apply_requested.emit()
        self.hide_toast()

    def _on_toast_discard(self) -> None:
        self.hide_toast()
        self.discard_requested.emit()

    def show_toast(self, notes_dirty: bool, pedal_dirty_independent: bool) -> None:
        """Update toast message and slide it into view if not already visible."""
        self._toast_notes_dirty = notes_dirty
        self._toast_pedal_dirty = pedal_dirty_independent
        self._toast.update_message(notes_dirty, pedal_dirty_independent)
        if not self._toast.is_toast_visible():
            self._set_scroll_bottom_pad(ApplyToast.HEIGHT)
            self._toast.show_sliding()

    def hide_toast(self) -> None:
        self._set_scroll_bottom_pad(0)
        self._toast.hide_sliding()
        self._toast_notes_dirty = False
        self._toast_pedal_dirty = False

    def shake_toast(self) -> None:
        if not self._toast.is_toast_visible():
            self._set_scroll_bottom_pad(ApplyToast.HEIGHT)
            self._toast.show_sliding()
        self._toast.shake()

    def navigate_to_playback_sub_tab(self) -> None:
        """Switch to the Playback sub-tab (index 1) programmatically."""
        self._sub_tab_bar.set_active(1)

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
        self._file_page_layout = layout
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
        self._playback_page_layout = layout
        layout.setSpacing(10)

        # Row 1: two equal columns
        row1 = QHBoxLayout()
        row1.setSpacing(14)

        # Left column: PERFORMANCE card stretched to fill column height.
        self._perf_card = PerformanceCard()
        self._perf_card.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )
        self.pedal_style_combo    = self._perf_card.pedal_style_combo
        self.transpose_spinbox    = self._perf_card.transpose_spinbox
        self.perf_reset_icon      = self._perf_card.reset_icon
        self.use_midi_pedal_check = self._perf_card.use_midi_pedal_check
        self.use_velocity_accent_check         = self._perf_card.use_velocity_accent_check
        self.velocity_accent_threshold_spinbox = self._perf_card.velocity_accent_threshold_spinbox

        # Right column: OPTIONS card.
        self._opts_card = OptionsCard()
        self._opts_card.setSizePolicy(
            QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
        )
        self.use_88_key_check = self._opts_card.use_88_key_check
        self.countdown_check  = self._opts_card.countdown_check
        self.debug_check      = self._opts_card.debug_check
        self.opts_reset_icon  = self._opts_card.reset_icon

        row1.addWidget(self._perf_card, 1)
        row1.addWidget(self._opts_card, 1)

        # Row 2: TEMPO card spanning full width.
        tempo_card, tempo_body = make_card("TEMPO")
        tempo_row = QHBoxLayout()
        tempo_row.setSpacing(8)

        _label_container = QWidget()
        _lbl_vbox = QVBoxLayout(_label_container)
        _lbl_vbox.setContentsMargins(0, 0, 0, 0)
        _lbl_vbox.setSpacing(1)
        _lbl_vbox.addWidget(QLabel("Tempo"))
        _tempo_desc = QLabel("speed multiplier")
        _tempo_desc.setProperty("variant", "muted")
        _lbl_vbox.addWidget(_tempo_desc)

        self.tempo_slider, self.tempo_spinbox = make_slider_spinbox(
            0.1, 10.0, 1.0, "x", factor=100.0, decimals=2
        )
        self.tempo_spinbox.setFixedWidth(72)
        self.tempo_slider.setToolTip("Playback speed as a multiplier of the original tempo")
        self.tempo_spinbox.setToolTip("Playback speed as a multiplier of the original tempo")

        tempo_row.addWidget(_label_container)
        tempo_row.addWidget(self.tempo_slider, 1)
        tempo_row.addWidget(self.tempo_spinbox)
        tempo_body.addLayout(tempo_row)

        bpm_row = QHBoxLayout()
        bpm_row.setContentsMargins(0, 2, 0, 0)
        bpm_row.setSpacing(4)
        bpm_row.addStretch(1)
        _orig_prefix = QLabel("Original:")
        _orig_prefix.setProperty("variant", "muted")
        self._bpm_original_label = QLabel("-- BPM")
        _sep = QLabel("·")
        _sep.setProperty("variant", "muted")
        _target_prefix = QLabel("Target:")
        _target_prefix.setProperty("variant", "muted")
        self.target_bpm_spinbox = NoScrollDoubleSpinBox()
        self.target_bpm_spinbox.setRange(1.0, 9999.0)
        self.target_bpm_spinbox.setDecimals(1)
        self.target_bpm_spinbox.setSuffix(" BPM")
        self.target_bpm_spinbox.setFixedWidth(100)
        self.target_bpm_spinbox.setEnabled(False)
        self.target_bpm_spinbox.setButtonSymbols(
            NoScrollDoubleSpinBox.ButtonSymbols.NoButtons
        )
        bpm_row.addWidget(_orig_prefix)
        bpm_row.addWidget(self._bpm_original_label)
        bpm_row.addSpacing(8)
        bpm_row.addWidget(_sep)
        bpm_row.addSpacing(8)
        bpm_row.addWidget(_target_prefix)
        bpm_row.addWidget(self.target_bpm_spinbox)
        bpm_row.addStretch(1)
        tempo_body.addLayout(bpm_row)
        tempo_body.addStretch()

        self.tempo_spinbox.valueChanged.connect(self._update_result_bpm)
        self.target_bpm_spinbox.editingFinished.connect(self._on_target_bpm_edited)

        # Row 3: PEDAL AI THRESHOLDS card (full width; locked until first AI generation).
        self._pedal_ai_card = PedalAICard()
        self._pedal_ai_card.generate_requested.connect(self.generate_pedal_requested.emit)
        self.pedal_ai_reset_icon = self._pedal_ai_card.reset_icon

        layout.addLayout(row1, 1)
        layout.addWidget(tempo_card)
        layout.addWidget(self._pedal_ai_card)
        return page

    # -- BPM display ----------------------------------------------------------

    def update_bpm_display(self, original_bpm: float) -> None:
        self._original_bpm = original_bpm
        self.target_bpm_spinbox.setEnabled(original_bpm > 0)
        self._update_result_bpm(self.tempo_spinbox.value())

    def _update_result_bpm(self, multiplier: float) -> None:
        if self._original_bpm > 0:
            self._bpm_original_label.setText(f"{self._original_bpm:.1f} BPM")
            self.target_bpm_spinbox.setValue(self._original_bpm * multiplier)
        else:
            self._bpm_original_label.setText("-- BPM")

    def _on_target_bpm_edited(self) -> None:
        if self._original_bpm <= 0:
            return
        multiplier = self.target_bpm_spinbox.value() / self._original_bpm
        multiplier = max(0.1, min(10.0, multiplier))
        self.tempo_slider.setValue(int(round(multiplier * 100.0)))

    # -- Tab III: Humanize ----------------------------------------------------

    def _build_humanize_tab(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(12, 12, 12, 12)
        self._humanize_page_layout = layout
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
        _invert_desc.setProperty("variant", "muted")
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

    def set_midi_pedal_available(self, available: bool) -> None:
        """Show or hide the 'Use MIDI Pedal' toggle based on CC 64 event presence."""
        self._perf_card.set_midi_pedal_available(available)

    def set_ai_thresholds(self, threshold_on: float, threshold_off: float) -> None:
        """Populate the PEDAL AI THRESHOLDS card with auto-computed values.

        Called by MainWindow after the first successful AI pedal generation.
        Enables the threshold spinboxes for user adjustment.
        """
        self._pedal_ai_card.set_thresholds(threshold_on, threshold_off)

    def set_ai_pedal_stats(self, avg_dur: float, min_dur: float, max_dur: float, presses_per_min: float) -> None:
        """Populate the pedal stats section of the PEDAL AI THRESHOLDS card."""
        self._pedal_ai_card.set_stats(avg_dur, min_dur, max_dur, presses_per_min)

    def set_generate_pedal_enabled(self, enabled: bool) -> None:
        """Enable or disable the Generate button in the PEDAL AI THRESHOLDS card."""
        self._pedal_ai_card.set_generate_enabled(enabled)

    def reset_pedal_ai_card(self) -> None:
        """Reset the PEDAL AI THRESHOLDS card to its pre-generate state.

        Call whenever the active song changes (new MIDI opened or save loaded)
        so stale thresholds and stats from the previous song are not shown.
        """
        self._pedal_ai_card.reset()

    def update_file_label(self, text: str, tooltip: str = "") -> None:
        self.file_path_label.setText(text)
        self.file_path_label.setToolTip(tooltip)
        self.file_strip.update_file(text)

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

    def restore_from_runtime_config(self, config: dict) -> None:
        """Restore UI controls from a runtime config dict (gather_playback_config format).

        Used by the Discard path to roll back UI to the last compiled snapshot.
        Only touches keys present in config; ignores unknown keys silently.
        """
        self._restoring = True
        try:
            self._restore_from_runtime_config_impl(config)
        finally:
            self._restoring = False

    def _restore_from_runtime_config_impl(self, config: dict) -> None:
        if 'tempo' in config:
            m = config['tempo'] / 100.0
            self.tempo_slider.blockSignals(True)
            self.tempo_slider.setValue(int(round(m * 100.0)))
            self.tempo_slider.blockSignals(False)
            self.tempo_spinbox.setValue(m)
        if 'pedal_style' in config:
            display = self.PEDAL_MAPPING_INV.get(config['pedal_style'], "Auto (Default)")
            self.pedal_style_combo.setCurrentText(display)
        if 'pedal_threshold_on' in config and 'pedal_threshold_off' in config:
            self._pedal_ai_card.set_thresholds(
                config['pedal_threshold_on'], config['pedal_threshold_off']
            )
        if 'use_88_key_layout' in config:
            self.use_88_key_check.setChecked(config['use_88_key_layout'])
        if 'simulate_hands' in config:
            self.all_humanization_checks['simulate_hands'].setChecked(config['simulate_hands'])
        if 'enable_chord_roll' in config:
            self.all_humanization_checks['enable_chord_roll'].setChecked(config['enable_chord_roll'])
        if 'vary_timing' in config:
            self.all_humanization_checks['vary_timing'].setChecked(config['vary_timing'])
        if 'timing_variance' in config:
            self.all_humanization_spinboxes['vary_timing'].setValue(config['timing_variance'])
        if 'vary_articulation' in config:
            self.all_humanization_checks['vary_articulation'].setChecked(config['vary_articulation'])
        if 'articulation' in config:
            self.all_humanization_spinboxes['vary_articulation'].setValue(config['articulation'] * 100.0)
        if 'enable_drift_correction' in config:
            self.all_humanization_checks['hand_drift'].setChecked(config['enable_drift_correction'])
        if 'drift_decay_factor' in config:
            self.all_humanization_spinboxes['hand_drift'].setValue(config['drift_decay_factor'] * 100.0)
        if 'enable_tempo_sway' in config:
            self.all_humanization_checks['tempo_sway'].setChecked(config['enable_tempo_sway'])
        if 'tempo_sway_intensity' in config:
            self.all_humanization_spinboxes['tempo_sway'].setValue(config['tempo_sway_intensity'])
        if 'invert_tempo_sway' in config:
            self.all_humanization_checks['invert_tempo_sway'].setChecked(config['invert_tempo_sway'])
        if 'enable_mistakes' in config:
            self.all_humanization_checks['mistake_chance'].setChecked(config['enable_mistakes'])
        if 'mistake_chance' in config:
            self.all_humanization_spinboxes['mistake_chance'].setValue(config['mistake_chance'])
        if 'use_velocity_accent' in config:
            self.use_velocity_accent_check.setChecked(config['use_velocity_accent'])
        if 'velocity_accent_threshold' in config:
            self.velocity_accent_threshold_spinbox.setValue(config['velocity_accent_threshold'])
        self.update_enabled_states()

    def set_groups_enabled(self, enabled: bool, skip_playback_humanization: bool = False) -> None:
        self.file_strip.setEnabled(enabled)
        if not skip_playback_humanization:
            self.tempo_slider.setEnabled(enabled)
            self.tempo_spinbox.setEnabled(enabled)
            self.target_bpm_spinbox.setEnabled(enabled and self._original_bpm > 0)
            self.pedal_style_combo.setEnabled(enabled)
            self.transpose_spinbox.setEnabled(enabled)
            self.use_velocity_accent_check.setEnabled(enabled)
            self.velocity_accent_threshold_spinbox.setEnabled(enabled)
            self.use_88_key_check.setEnabled(enabled)
            self.countdown_check.setEnabled(enabled)
            self.debug_check.setEnabled(enabled)
            self.select_all_humanization_check.setEnabled(enabled)
            for w in list(self.all_humanization_checks.values()) + \
                     list(self.all_humanization_sliders.values()) + \
                     list(self.all_humanization_spinboxes.values()):
                w.setEnabled(enabled)
            self._pedal_ai_card.set_spinboxes_enabled(enabled)
            if not enabled:
                self._pedal_ai_card.set_generate_enabled(False)

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
        self.tempo_slider.setValue(int(1.0 * 100.0))
        self.tempo_spinbox.setValue(1.0)
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

    def load_config(self, config: dict) -> None:
        _multiplier = config.get('tempo', 100.0) / 100.0
        self.tempo_slider.setValue(int(_multiplier * 100.0))
        self.tempo_spinbox.setValue(_multiplier)
        self.transpose_spinbox.setValue(config.get('transpose', 0))
        display = self.PEDAL_MAPPING_INV.get(
            config.get('pedal_style', 'ai'), "PedalAI"
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
        if 'use_midi_pedal' in config:
            self.use_midi_pedal_check.setChecked(config['use_midi_pedal'])
        self.use_velocity_accent_check.setChecked(config.get('use_velocity_accent', False))
        self.velocity_accent_threshold_spinbox.setValue(config.get('velocity_accent_threshold', 100))
        self.update_enabled_states()

    def gather_playback_config(self) -> dict:
        internal = self.PEDAL_MAPPING.get(
            self.pedal_style_combo.currentText(), 'hybrid'
        )
        uses_ai = internal in ('ai', 'hybrid')
        if uses_ai and self._pedal_ai_card.has_thresholds:
            t_on  = self._pedal_ai_card.get_threshold_on()
            t_off = self._pedal_ai_card.get_threshold_off()
        else:
            t_on  = -1.0
            t_off = -1.0
        return {
            'midi_file':               self.file_path_label.toolTip(),
            'tempo':                   self.tempo_spinbox.value() * 100.0,
            'transpose':               self.transpose_spinbox.value(),
            'countdown':               self.countdown_check.isChecked(),
            'use_88_key_layout':       self.use_88_key_check.isChecked(),
            'pedal_style':             internal,
            'pedal_threshold_on':      t_on,
            'pedal_threshold_off':     t_off,
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
            'use_midi_pedal':          self.use_midi_pedal_check.isChecked(),
            'use_velocity_accent':     self.use_velocity_accent_check.isChecked(),
            'velocity_accent_threshold': self.velocity_accent_threshold_spinbox.value(),
        }

    def gather_app_config(self) -> dict:
        internal = self.PEDAL_MAPPING.get(
            self.pedal_style_combo.currentText(), 'hybrid'
        )
        return {
            'tempo':                      self.tempo_spinbox.value() * 100.0,
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
            'use_midi_pedal':             self.use_midi_pedal_check.isChecked(),
            'use_velocity_accent':        self.use_velocity_accent_check.isChecked(),
            'velocity_accent_threshold':  self.velocity_accent_threshold_spinbox.value(),
        }
