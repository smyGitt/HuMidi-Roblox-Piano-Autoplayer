from PyQt6.QtWidgets import (QWidget, QVBoxLayout, QHBoxLayout, QPushButton, 
                             QCheckBox, QSlider, QLabel, QGroupBox, QTabWidget, 
                             QTextEdit, QComboBox, QDoubleSpinBox, QGridLayout, 
                             QScrollArea, QLineEdit, QStatusBar, QApplication)
from PyQt6.QtCore import Qt, QObject
from PyQt6.QtGui import QFont

from ui.visualizer import PianoWidget, TimelineWidget

class MainWindowUI(QObject):
    def __init__(self, main_window):
        super().__init__()
        self.main_window = main_window
        
        self.pedal_mapping = {
            "Automatic (Default)": "hybrid",
            "Always Sustain": "legato",
            "Rhythmic Only": "rhythmic",
            "No Pedal": "none"
        }
        self.pedal_mapping_inv = {v: k for k, v in self.pedal_mapping.items()}
        
        self.setup_ui()

    def setup_ui(self):
        main_widget = QWidget()
        self.main_window.setCentralWidget(main_widget)
        main_layout = QVBoxLayout(main_widget)
        main_layout.setContentsMargins(10, 10, 10, 5)

        self.tabs = QTabWidget()
        main_layout.addWidget(self.tabs)
        controls_tab, visual_tab, settings_tab, log_tab = QWidget(), QWidget(), QWidget(), QWidget()
        self.tabs.addTab(controls_tab, "Playback")
        self.tabs.addTab(visual_tab, "Visualizer")
        self.tabs.addTab(settings_tab, "Settings")
        self.tabs.addTab(log_tab, "Debug")

        # --- Visualizer Tab ---
        vis_layout = QVBoxLayout(visual_tab)
        vis_layout.setContentsMargins(5, 5, 5, 5)
        
        self.scroll_area = QScrollArea()
        self.scroll_area.setWidgetResizable(True) 
        self.scroll_area.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOn)
        self.scroll_area.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        
        self.timeline_widget = TimelineWidget()
        self.scroll_area.setWidget(self.timeline_widget)
        vis_layout.addWidget(self.scroll_area)
        
        self.piano_widget = PianoWidget()
        vis_layout.addWidget(self.piano_widget)

        # --- Controls Tab ---
        controls_layout = QVBoxLayout(controls_tab)
        self.file_group = self._create_file_group()
        controls_layout.addWidget(self.file_group)
        self.playback_group = self._create_playback_group()
        controls_layout.addWidget(self.playback_group)
        self.humanization_group = self._create_humanization_group()
        controls_layout.addWidget(self.humanization_group)
        controls_layout.addStretch()

        # --- Settings Tab ---
        settings_layout = QVBoxLayout(settings_tab)
        hk_group = QGroupBox("Hotkey")
        hk_layout = QHBoxLayout(hk_group)
        self.hk_label = QLabel(f"Start/Stop Hotkey: ")
        self.hk_btn = QPushButton("Change")
        hk_layout.addWidget(self.hk_label)
        hk_layout.addWidget(self.hk_btn)
        settings_layout.addWidget(hk_group)

        overlay_group = QGroupBox("Overlay Mode")
        ov_layout = QGridLayout(overlay_group)
        self.always_top_check = QCheckBox("Window Always on Top")
        
        opacity_label = QLabel("Window Opacity:")
        self.opacity_slider = QSlider(Qt.Orientation.Horizontal)
        self.opacity_slider.setRange(20, 100)
        self.opacity_slider.setValue(100)
        
        ov_layout.addWidget(self.always_top_check, 0, 0, 1, 2)
        ov_layout.addWidget(opacity_label, 1, 0)
        ov_layout.addWidget(self.opacity_slider, 1, 1)
        settings_layout.addWidget(overlay_group)
        
        # --- Save Directory Settings ---
        save_group = QGroupBox("Save Configuration")
        save_layout = QHBoxLayout(save_group)
        self.save_path_input = QLineEdit()
        self.save_path_input.setReadOnly(True)
        self.save_browse_btn = QPushButton("Browse")
        save_layout.addWidget(self.save_path_input)
        save_layout.addWidget(self.save_browse_btn)
        settings_layout.addWidget(save_group)
        
        settings_layout.addStretch()

        # --- Log Tab ---
        self.log_output = QTextEdit()
        self.log_output.setReadOnly(True)
        self.log_output.setFont(QFont("Courier", 9))
        log_layout = QVBoxLayout(log_tab)
        log_layout.addWidget(self.log_output)
        
        log_btn_layout = QHBoxLayout()
        self.log_clear_btn = QPushButton("Clear")
        self.log_copy_btn = QPushButton("Copy to Clipboard")
        self.log_clear_btn.clicked.connect(self.log_output.clear)
        self.log_copy_btn.clicked.connect(self.copy_log_to_clipboard)
        log_btn_layout.addWidget(self.log_clear_btn)
        log_btn_layout.addWidget(self.log_copy_btn)
        log_layout.addLayout(log_btn_layout)

        # Main Action Buttons (Bottom)
        media_layout = QHBoxLayout()
        self.time_label = QLabel("00:00 / 00:00")
        self.time_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        media_layout.addWidget(self.time_label)

        button_layout = QHBoxLayout()
        self.play_button = QPushButton("Play") 
        self.stop_button = QPushButton("Stop")
        self.save_button = QPushButton("Save")
        self.reset_button = QPushButton("Reset Defaults")
        button_layout.addWidget(self.play_button)
        button_layout.addWidget(self.stop_button)
        button_layout.addWidget(self.save_button)
        button_layout.addStretch()
        button_layout.addWidget(self.reset_button)
        
        main_layout.addLayout(media_layout)
        main_layout.addLayout(button_layout)
        
        # --- GitHub Link Integration ---
        github_layout = QHBoxLayout()
        github_label = QLabel('<a href="https://github.com/smyGitt/HuMidi-Roblox-Piano-Autoplayer"><span style="color: gray; text-decoration: underline;">by smyGitt on GitHub</span></a>')
        github_label.setOpenExternalLinks(True)
        github_layout.addStretch()
        github_layout.addWidget(github_label)
        main_layout.addLayout(github_layout)

        # GUI initialization dependencies
        self.play_button.setEnabled(False) 
        self.stop_button.setEnabled(False)
        self.save_button.setEnabled(False)
        status_bar = QStatusBar()
        self.main_window.setStatusBar(status_bar)

    def _create_slider_and_spinbox(self, min_val, max_val, default_val, text_suffix="", factor=10000.0, decimals=4):
        slider = QSlider(Qt.Orientation.Horizontal)
        slider.setRange(int(min_val * factor), int(max_val * factor))
        spinbox = QDoubleSpinBox()
        spinbox.setDecimals(decimals)
        spinbox.setRange(0.0, 9999.9999)
        spinbox.setSingleStep(1.0 / factor)
        spinbox.setSuffix(text_suffix)
        slider.setValue(int(default_val * factor))
        spinbox.setValue(default_val)
        slider.valueChanged.connect(lambda v: spinbox.setValue(v / factor))
        spinbox.valueChanged.connect(lambda v: slider.setValue(int(v * factor)))
        return slider, spinbox

    def _create_file_group(self):
        group = QGroupBox("MIDI")
        layout = QVBoxLayout(group)
        self.file_path_label = QLabel("No file selected.")
        self.file_path_label.setStyleSheet("font-style: italic; color: grey;")
        
        btn_layout = QHBoxLayout()
        self.browse_button = QPushButton("Browse for MIDI File")
        self.load_saved_btn = QPushButton("Load saved...")
        
        btn_layout.addWidget(self.browse_button)
        btn_layout.addWidget(self.load_saved_btn)
        
        layout.addWidget(self.file_path_label)
        layout.addLayout(btn_layout)
        return group

    def _create_playback_group(self):
        group = QGroupBox("Playback")
        grid = QGridLayout(group)
        tempo_label = QLabel("Tempo")
        self.tempo_slider, self.tempo_spinbox = self._create_slider_and_spinbox(10.0, 200.0, 100.0, "%", factor=10.0, decimals=1)
        grid.addWidget(tempo_label, 0, 0) 
        grid.addWidget(self.tempo_slider, 0, 2); grid.addWidget(self.tempo_spinbox, 0, 3)

        pedal_label = QLabel("Pedal Style")
        self.pedal_style_combo = QComboBox()
        self.pedal_style_combo.addItems(list(self.pedal_mapping.keys()))
        
        grid.addWidget(pedal_label, 1, 0)
        grid.addWidget(self.pedal_style_combo, 1, 2, 1, 2)
        self.use_88_key_check = QCheckBox("Use 88-Key Extended Layout")
        grid.addWidget(self.use_88_key_check, 2, 0, 1, 4)
        self.countdown_check = QCheckBox("3 second countdown")
        self.debug_check = QCheckBox("Enable debug output")
        grid.addWidget(self.countdown_check, 3, 0, 1, 4)
        grid.addWidget(self.debug_check, 4, 0, 1, 4)
        grid.setColumnStretch(2, 1)
        return group

    def _create_humanization_group(self):
        group = QGroupBox("Humanization")
        main_v_layout = QVBoxLayout(group)
        self.select_all_humanization_check = QCheckBox("Select/Deselect All")
        main_v_layout.addWidget(self.select_all_humanization_check)
        self.all_humanization_checks = {}
        self.all_humanization_spinboxes = {}
        self.all_humanization_sliders = {}

        simple_toggles_layout = QHBoxLayout()
        self.all_humanization_checks['simulate_hands'] = QCheckBox("Simulate Hands")
        self.all_humanization_checks['enable_chord_roll'] = QCheckBox("Chord Rolling")
        simple_toggles_layout.addWidget(self.all_humanization_checks['simulate_hands'])
        simple_toggles_layout.addStretch(1)
        simple_toggles_layout.addWidget(self.all_humanization_checks['enable_chord_roll'])
        main_v_layout.addLayout(simple_toggles_layout)
        
        detailed_layout = QGridLayout()
        detailed_layout.setColumnStretch(2, 1) 
        
        def add_detailed_row(row_idx, name, key, min_val, max_val, def_val, suffix, factor=1.0, decimals=3):
            check = QCheckBox(name)
            slider, spinbox = self._create_slider_and_spinbox(min_val, max_val, def_val, suffix, factor=factor, decimals=decimals)
            check.toggled.connect(slider.setEnabled)
            check.toggled.connect(spinbox.setEnabled)
            detailed_layout.addWidget(check, row_idx, 0)
            detailed_layout.addWidget(slider, row_idx, 2)
            detailed_layout.addWidget(spinbox, row_idx, 3)
            self.all_humanization_checks[key] = check
            self.all_humanization_sliders[key] = slider
            self.all_humanization_spinboxes[key] = spinbox

        add_detailed_row(0, "Vary Timing", "vary_timing", 0, 0.1, 0.01, " s", factor=10000.0)
        add_detailed_row(1, "Vary Articulation", "vary_articulation", 50, 100, 95, "%", factor=100.0, decimals=1)
        add_detailed_row(2, "Hand Drift", "hand_drift", 0, 100, 25, "%", factor=100.0, decimals=1)
        add_detailed_row(3, "Mistake Chance", "mistake_chance", 0, 10, 0, "%", factor=100.0, decimals=1)
        add_detailed_row(4, "Tempo Sway", "tempo_sway", 0, 0.1, 0, " s", factor=10000.0)

        self.invert_sway_check = QCheckBox("Invert tempo sway")
        self.all_humanization_checks['invert_tempo_sway'] = self.invert_sway_check
        self.all_humanization_checks['tempo_sway'].toggled.connect(self.invert_sway_check.setEnabled)
        detailed_layout.addWidget(self.invert_sway_check, 5, 0)
        main_v_layout.addLayout(detailed_layout)
        
        self.all_humanization_checks['vary_velocity'] = QCheckBox() # Dummy for logic compatibility
        self.select_all_humanization_check.toggled.connect(self._toggle_all_humanization)
        for check in self.all_humanization_checks.values():
            if check.text(): check.toggled.connect(self._update_select_all_state)
            
        return group

    def reset_controls_to_default(self):
        self.tempo_spinbox.setValue(100)
        self.pedal_style_combo.setCurrentText("Automatic (Default)")
        self.use_88_key_check.setChecked(False)
        self.countdown_check.setChecked(True)
        self.debug_check.setChecked(False)
        
        self.all_humanization_spinboxes['vary_timing'].setValue(0.010)
        self.all_humanization_spinboxes['vary_articulation'].setValue(95.0)
        self.all_humanization_spinboxes['hand_drift'].setValue(25.0)
        self.all_humanization_spinboxes['mistake_chance'].setValue(0.5)
        self.all_humanization_spinboxes['tempo_sway'].setValue(0.015)
        for check in self.all_humanization_checks.values(): 
            if check.text(): check.setChecked(False)
        self.update_enabled_states()

    def _toggle_all_humanization(self, checked):
        for check in self.all_humanization_checks.values(): 
            if check.text(): check.setChecked(checked)

    def _update_select_all_state(self):
        checks = [c for c in self.all_humanization_checks.values() if c.text()]
        is_all_checked = all(c.isChecked() for c in checks)
        self.select_all_humanization_check.blockSignals(True)
        self.select_all_humanization_check.setChecked(is_all_checked)
        self.select_all_humanization_check.blockSignals(False)

    def set_controls_enabled(self, enabled, ignore_if_loaded=False):
        # Strictly explicit groups. Avoids disabling visualizer/settings tabs.
        groups_to_toggle = [self.file_group, self.playback_group, self.humanization_group]
        for group in groups_to_toggle:
            if group in (self.playback_group, self.humanization_group) and ignore_if_loaded and enabled:
                continue 
            group.setEnabled(enabled)

    def update_enabled_states(self):
        for key, check in self.all_humanization_checks.items():
            if not check.text(): continue
            is_checked = check.isChecked()
            if key in self.all_humanization_sliders: self.all_humanization_sliders[key].setEnabled(is_checked)
            if key in self.all_humanization_spinboxes: self.all_humanization_spinboxes[key].setEnabled(is_checked)
        self.invert_sway_check.setEnabled(self.all_humanization_checks['tempo_sway'].isChecked())

    def copy_log_to_clipboard(self):
        clipboard = QApplication.clipboard()
        clipboard.setText(self.log_output.toPlainText())
        self.main_window.statusBar().showMessage("Log copied to clipboard!", 2000)

    def update_progress(self, current_time, total_duration):
        if not self.timeline_widget.is_dragging:
            self.timeline_widget.set_position(current_time)
            self.update_time_label(current_time, total_duration)
            timeline_width = self.timeline_widget.width()
            scroll_width = self.scroll_area.width()
            if total_duration > 0:
                ratio = current_time / total_duration
                cursor_x = ratio * timeline_width
                target_scroll = cursor_x - (scroll_width / 2)
                self.scroll_area.horizontalScrollBar().setValue(int(target_scroll))

    def update_time_label(self, current, total):
        def fmt(s):
            m = int(s // 60); sec = int(s % 60)
            return f"{m:02d}:{sec:02d}"
        self.time_label.setText(f"{fmt(current)} / {fmt(total)}")

    def load_config_to_ui(self, config, save_dir):
        self.tempo_spinbox.setValue(config.get('tempo', 100.0))
        internal_style = config.get('pedal_style', 'hybrid')
        display_text = self.pedal_mapping_inv.get(internal_style, "Automatic (Default)")
        self.pedal_style_combo.setCurrentText(display_text)
        self.use_88_key_check.setChecked(config.get('use_88_key_layout', False))
        self.countdown_check.setChecked(config.get('countdown', True))
        self.debug_check.setChecked(config.get('debug_mode', False))
        self.select_all_humanization_check.setChecked(config.get('select_all_humanization', False))
        self.all_humanization_checks['simulate_hands'].setChecked(config.get('simulate_hands', False))
        self.all_humanization_checks['enable_chord_roll'].setChecked(config.get('enable_chord_roll', False))
        self.all_humanization_checks['vary_timing'].setChecked(config.get('enable_vary_timing', False))
        self.all_humanization_spinboxes['vary_timing'].setValue(config.get('value_timing_variance', 0.010))
        self.all_humanization_checks['vary_articulation'].setChecked(config.get('enable_vary_articulation', False))
        self.all_humanization_spinboxes['vary_articulation'].setValue(config.get('value_articulation', 95.0))
        self.all_humanization_checks['hand_drift'].setChecked(config.get('enable_hand_drift', False))
        self.all_humanization_spinboxes['hand_drift'].setValue(config.get('value_hand_drift_decay', 25.0))
        self.all_humanization_checks['mistake_chance'].setChecked(config.get('enable_mistakes', False))
        self.all_humanization_spinboxes['mistake_chance'].setValue(config.get('value_mistake_chance', 0.5))
        self.all_humanization_checks['tempo_sway'].setChecked(config.get('enable_tempo_sway', False))
        self.all_humanization_spinboxes['tempo_sway'].setValue(config.get('value_tempo_sway_intensity', 0.015))
        self.all_humanization_checks['invert_tempo_sway'].setChecked(config.get('invert_tempo_sway', False))
        self.always_top_check.setChecked(config.get('always_on_top', False))
        self.opacity_slider.setValue(config.get('opacity', 100))
        self.save_path_input.setText(save_dir)
        self.update_enabled_states()

    def gather_playback_config(self):
        """Constructs strictly the properties necessary for executing/modifying MIDI objects"""
        display_text = self.pedal_style_combo.currentText()
        internal_style = self.pedal_mapping.get(display_text, 'hybrid')
        return {
            'midi_file': self.file_path_label.toolTip(), 
            'tempo': self.tempo_spinbox.value(), 
            'countdown': self.countdown_check.isChecked(),
            'use_88_key_layout': self.use_88_key_check.isChecked(),
            'pedal_style': internal_style, 
            'debug_mode': self.debug_check.isChecked(),
            'simulate_hands': self.all_humanization_checks['simulate_hands'].isChecked(),
            'vary_velocity': False,
            'enable_chord_roll': self.all_humanization_checks['enable_chord_roll'].isChecked(),
            'vary_timing': self.all_humanization_checks['vary_timing'].isChecked(), 
            'timing_variance': self.all_humanization_spinboxes['vary_timing'].value(),
            'vary_articulation': self.all_humanization_checks['vary_articulation'].isChecked(), 
            'articulation': self.all_humanization_spinboxes['vary_articulation'].value() / 100.0,
            'enable_drift_correction': self.all_humanization_checks['hand_drift'].isChecked(), 
            'drift_decay_factor': self.all_humanization_spinboxes['hand_drift'].value() / 100.0,
            'enable_mistakes': self.all_humanization_checks['mistake_chance'].isChecked(), 
            'mistake_chance': self.all_humanization_spinboxes['mistake_chance'].value(),
            'enable_tempo_sway': self.all_humanization_checks['tempo_sway'].isChecked(), 
            'tempo_sway_intensity': self.all_humanization_spinboxes['tempo_sway'].value(),
            'invert_tempo_sway': self.all_humanization_checks['invert_tempo_sway'].isChecked(),
        }
        
    def gather_app_config(self):
        """Constructs an exhaustive dictionary of all physical widget states to be serialized"""
        display_text = self.pedal_style_combo.currentText()
        internal_style = self.pedal_mapping.get(display_text, 'hybrid')
        return {
            'tempo': self.tempo_spinbox.value(),
            'pedal_style': internal_style,
            'use_88_key_layout': self.use_88_key_check.isChecked(),
            'countdown': self.countdown_check.isChecked(),
            'debug_mode': self.debug_check.isChecked(),
            'select_all_humanization': self.select_all_humanization_check.isChecked(),
            'simulate_hands': self.all_humanization_checks['simulate_hands'].isChecked(),
            'enable_chord_roll': self.all_humanization_checks['enable_chord_roll'].isChecked(),
            'enable_vary_timing': self.all_humanization_checks['vary_timing'].isChecked(), 
            'value_timing_variance': self.all_humanization_spinboxes['vary_timing'].value(),
            'enable_vary_articulation': self.all_humanization_checks['vary_articulation'].isChecked(), 
            'value_articulation': self.all_humanization_spinboxes['vary_articulation'].value(),
            'enable_hand_drift': self.all_humanization_checks['hand_drift'].isChecked(), 
            'value_hand_drift_decay': self.all_humanization_spinboxes['hand_drift'].value(),
            'enable_mistakes': self.all_humanization_checks['mistake_chance'].isChecked(), 
            'value_mistake_chance': self.all_humanization_spinboxes['mistake_chance'].value(),
            'enable_tempo_sway': self.all_humanization_checks['tempo_sway'].isChecked(), 
            'value_tempo_sway_intensity': self.all_humanization_spinboxes['tempo_sway'].value(),
            'invert_tempo_sway': self.all_humanization_checks['invert_tempo_sway'].isChecked(),
            'always_on_top': self.always_top_check.isChecked(),
            'opacity': self.opacity_slider.value()
        }