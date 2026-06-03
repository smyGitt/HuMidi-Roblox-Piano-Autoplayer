from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QPushButton, QCheckBox, QSlider,
    QLabel, QComboBox, QLineEdit, QGridLayout, QFrame)
from PyQt6.QtCore import Qt

from ui.widgets import make_card
from ui.theme import ThemeManager


class SettingsTab(QWidget):

    def __init__(self, parent=None):
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(16, 0, 16, 16)
        outer.setSpacing(0)

        # -- Page header -------------------------------------------------------
        header = QFrame()
        header.setObjectName("page_header")
        hl = QHBoxLayout(header)
        hl.setContentsMargins(0, 14, 0, 8)
        hl.setSpacing(8)
        title_lbl = QLabel("Settings")
        title_lbl.setProperty("role", "title")
        hl.addWidget(title_lbl)
        hl.addStretch()
        # TODO: meta chips (e.g. active theme name)
        outer.addWidget(header)

        # -- Save Path card (full width) ---------------------------------------
        save_card, save_content = make_card("Save Path")
        save_row = QHBoxLayout()
        save_row.setSpacing(8)
        self.save_path_input = QLineEdit()
        self.save_path_input.setReadOnly(True)
        self.save_path_input.setToolTip("Directory where humanized performance saves are stored")
        self.save_browse_btn = QPushButton("Browse")
        self.save_browse_btn.setToolTip("Choose where to save humanized performance files")
        save_row.addWidget(self.save_path_input)
        save_row.addWidget(self.save_browse_btn)
        save_content.addLayout(save_row)
        outer.addWidget(save_card)
        outer.addSpacing(10)

        # -- Aligned 2-column grid: Overlay & Window | Hotkeys | Updates | Visualizer
        grid_widget = QWidget()
        grid = QGridLayout(grid_widget)
        grid.setContentsMargins(0, 0, 0, 0)
        grid.setSpacing(10)
        grid.setColumnStretch(0, 1)
        grid.setColumnStretch(1, 1)
        grid.setRowStretch(0, 1)
        grid.setRowStretch(1, 1)

        # Overlay & Window card (row 0, col 0)
        ov_card, ov_content = make_card("Overlay & Window")
        ov_grid = QGridLayout()
        ov_grid.setSpacing(8)
        self.always_top_check = QCheckBox("Always on Top")
        self.always_top_check.setToolTip("Keep this window above all other windows")
        opacity_label = QLabel("Opacity")
        self.opacity_slider = QSlider(Qt.Orientation.Horizontal)
        self.opacity_slider.setRange(20, 100)
        self.opacity_slider.setValue(100)
        self.opacity_slider.setToolTip("Adjust window transparency (20-100%)")
        ov_grid.addWidget(self.always_top_check, 0, 0, 1, 2)
        ov_grid.addWidget(opacity_label,         1, 0)
        ov_grid.addWidget(self.opacity_slider,   1, 1)
        ov_content.addLayout(ov_grid)
        ov_content.addStretch()
        grid.addWidget(ov_card, 0, 0)

        # Hotkeys card (row 0, col 1)
        hk_card, hk_content = make_card("Hotkeys")
        hk_row = QHBoxLayout()
        hk_row.setSpacing(8)
        self.hk_label = QLabel("Hotkey: ")
        self.hk_btn = QPushButton("Change")
        self.hk_btn.setToolTip("Click to bind a new hotkey for toggling playback")
        hk_row.addWidget(self.hk_label, 1)
        hk_row.addWidget(self.hk_btn)
        hk_content.addLayout(hk_row)
        hk_content.addStretch()
        grid.addWidget(hk_card, 0, 1)

        # Updates card (row 1, col 0)
        upd_card, upd_content = make_card("Updates")
        self.check_update_btn = QPushButton("Check for updates")
        self.check_update_btn.setToolTip("Check GitHub for a newer version of HuMidi")
        upd_content.addWidget(self.check_update_btn)
        # TODO: update status label and installed/latest version display
        upd_content.addStretch()
        grid.addWidget(upd_card, 1, 0)

        # Visualizer card (row 1, col 1)
        vis_card, vis_content = make_card("Visualizer")
        self.timeline_vis_check = QCheckBox("Timeline")
        self.timeline_vis_check.setChecked(True)
        self.timeline_vis_check.setToolTip(
            "Show the piano-roll timeline in the Visualizer tab "
            "(disable for a simple seek slider)"
        )
        self.piano_vis_check = QCheckBox("Piano Keys")
        self.piano_vis_check.setChecked(True)
        self.piano_vis_check.setToolTip("Show the piano key visualizer in the Visualizer tab")
        vis_content.addWidget(self.timeline_vis_check)
        vis_content.addWidget(self.piano_vis_check)
        vis_content.addStretch()
        grid.addWidget(vis_card, 1, 1)

        outer.addWidget(grid_widget)
        outer.addSpacing(10)

        # -- Bottom row: Appearance | Reset (side by side) --------------------
        bottom_row_widget = QWidget()
        bottom_row = QHBoxLayout(bottom_row_widget)
        bottom_row.setContentsMargins(0, 0, 0, 0)
        bottom_row.setSpacing(10)

        # Appearance card
        app_card, app_content = make_card("Appearance")
        theme_row = QHBoxLayout()
        theme_row.setSpacing(8)
        self.theme_combo = QComboBox()
        self.theme_combo.setToolTip("Switch the application colour theme")
        self._populate_theme_combo()
        self.theme_customize_btn = QPushButton("Customize...")
        self.theme_customize_btn.setToolTip(
            "Open the theme editor to create or modify colour presets"
        )
        theme_row.addWidget(self.theme_combo, 1)
        theme_row.addWidget(self.theme_customize_btn)
        app_content.addLayout(theme_row)
        app_content.addStretch()
        bottom_row.addWidget(app_card, 1)

        # Reset card
        reset_card, reset_content = make_card("Reset")
        reset_desc = QLabel("Restore all playback and humanization settings to their defaults.")
        reset_desc.setProperty("role", "muted")
        reset_desc.setWordWrap(True)
        self.reset_all_btn = QPushButton("Reset All Settings")
        self.reset_all_btn.setToolTip("Reset all playback and humanization settings to their default values")
        reset_content.addWidget(reset_desc)
        reset_content.addSpacing(8)
        reset_content.addWidget(self.reset_all_btn)
        reset_content.addStretch()
        bottom_row.addWidget(reset_card, 1)

        outer.addWidget(bottom_row_widget)

    def _populate_theme_combo(self) -> None:
        active = ThemeManager.get_active_name()
        for name in ThemeManager.all_themes():
            self.theme_combo.addItem(name)
        idx = self.theme_combo.findText(active)
        if idx >= 0:
            self.theme_combo.setCurrentIndex(idx)

    # ── Public API ─────────────────────────────────────────────────────────────

    def refresh_theme_combo(self) -> None:
        self.theme_combo.blockSignals(True)
        self.theme_combo.clear()
        self._populate_theme_combo()
        self.theme_combo.blockSignals(False)

    def load_config(self, config: dict, save_dir: str) -> None:
        self.always_top_check.setChecked(config.get('always_on_top', False))
        self.opacity_slider.setValue(config.get('opacity', 100))
        self.timeline_vis_check.setChecked(config.get('show_timeline_visualizer', True))
        self.piano_vis_check.setChecked(config.get('show_piano_visualizer', True))
        self.save_path_input.setText(save_dir)

    def gather_config(self) -> dict:
        return {
            'always_on_top':             self.always_top_check.isChecked(),
            'opacity':                   self.opacity_slider.value(),
            'show_timeline_visualizer':  self.timeline_vis_check.isChecked(),
            'show_piano_visualizer':     self.piano_vis_check.isChecked(),
        }
