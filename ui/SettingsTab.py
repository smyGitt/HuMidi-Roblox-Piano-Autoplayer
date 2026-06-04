from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QPushButton, QCheckBox, QSlider,
    QLabel, QComboBox, QLineEdit, QFrame, QStackedWidget)
from PyQt6.QtCore import Qt

from ui.widgets import make_card
from ui.theme import ThemeManager


class SettingsTab(QWidget):

    def __init__(self, parent=None):
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        outer = QVBoxLayout(self)
        outer.setContentsMargins(0, 0, 0, 0)
        outer.setSpacing(0)

        # Full-width page header bar
        header = QFrame()
        header.setObjectName("page_header")
        header.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        hl = QHBoxLayout(header)
        hl.setContentsMargins(14, 10, 14, 10)
        hl.setSpacing(8)
        title_lbl = QLabel("Settings")
        title_lbl.setObjectName("page_header_title")
        hl.addWidget(title_lbl)
        hl.addStretch()
        outer.addWidget(header)

        # Body widget restores side margins
        body = QWidget()
        body_layout = QVBoxLayout(body)
        body_layout.setContentsMargins(16, 8, 16, 16)
        body_layout.setSpacing(0)
        outer.addWidget(body, 1)

        # Horizontal split: nav | separator | content
        split = QHBoxLayout()
        split.setContentsMargins(0, 0, 0, 0)
        split.setSpacing(0)

        # --- Left nav panel ---
        nav_panel = QWidget()
        nav_panel.setObjectName("settings_nav_panel")
        nav_panel.setFixedWidth(110)
        nav_layout = QVBoxLayout(nav_panel)
        nav_layout.setContentsMargins(0, 8, 0, 8)
        nav_layout.setSpacing(0)

        self._tab_btns = []
        for i, name in enumerate(["Display", "Files", "Shortcut", "System"]):
            btn = QPushButton(name)
            btn.setCheckable(True)
            btn.setProperty("role", "settings_nav")
            btn.clicked.connect(lambda checked, idx=i: self._switch_tab(idx))
            nav_layout.addWidget(btn)
            self._tab_btns.append(btn)
        nav_layout.addStretch()

        # Vertical divider
        sep = QFrame()
        sep.setFrameShape(QFrame.Shape.VLine)
        sep.setObjectName("settings_nav_sep")

        # --- Content stack ---
        self._stack = QStackedWidget()
        self._stack.addWidget(self._make_display_page())
        self._stack.addWidget(self._make_files_page())
        self._stack.addWidget(self._make_shortcut_page())
        self._stack.addWidget(self._make_system_page())

        split.addWidget(nav_panel)
        split.addWidget(sep)
        split.addWidget(self._stack, 1)

        card, card_body = make_card("", outer_margins=(0, 0, 0, 0))
        card_body.addLayout(split)
        body_layout.addWidget(card, 1)

        self._switch_tab(0)

    # ── Page builders ───────────────────────────────────────────────────────────

    def _make_display_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(16, 12, 16, 12)
        layout.setSpacing(6)

        layout.addWidget(self._section_label("Window"))

        self.always_top_check = QCheckBox("Always on Top")
        self.always_top_check.setToolTip("Keep this window above all other windows")
        layout.addWidget(self.always_top_check)

        opacity_row = QHBoxLayout()
        opacity_row.setSpacing(8)
        opacity_row.addWidget(QLabel("Opacity"))
        self.opacity_slider = QSlider(Qt.Orientation.Horizontal)
        self.opacity_slider.setRange(20, 100)
        self.opacity_slider.setValue(100)
        self.opacity_slider.setToolTip("Adjust window transparency (20-100%)")
        opacity_row.addWidget(self.opacity_slider, 1)
        layout.addLayout(opacity_row)

        layout.addSpacing(8)
        layout.addWidget(self._section_label("Visualizer"))

        self.timeline_vis_check = QCheckBox("Timeline")
        self.timeline_vis_check.setChecked(True)
        self.timeline_vis_check.setToolTip(
            "Show the piano-roll timeline in the Visualizer tab "
            "(disable for a simple seek slider)"
        )
        self.piano_vis_check = QCheckBox("Piano Keys")
        self.piano_vis_check.setChecked(True)
        self.piano_vis_check.setToolTip("Show the piano key visualizer in the Visualizer tab")
        layout.addWidget(self.timeline_vis_check)
        layout.addWidget(self.piano_vis_check)

        layout.addSpacing(8)
        layout.addWidget(self._section_label("Appearance"))

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
        layout.addLayout(theme_row)

        layout.addStretch()
        return page

    def _make_files_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(16, 12, 16, 12)
        layout.setSpacing(6)

        layout.addWidget(self._section_label("Save Directory"))

        save_row = QHBoxLayout()
        save_row.setSpacing(8)
        self.save_path_input = QLineEdit()
        self.save_path_input.setReadOnly(True)
        self.save_path_input.setToolTip("Directory where humanized performance saves are stored")
        self.save_browse_btn = QPushButton("Browse")
        self.save_browse_btn.setToolTip("Choose where to save humanized performance files")
        save_row.addWidget(self.save_path_input, 1)
        save_row.addWidget(self.save_browse_btn)
        layout.addLayout(save_row)

        layout.addStretch()
        return page

    def _make_shortcut_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(16, 12, 16, 12)
        layout.setSpacing(6)

        layout.addWidget(self._section_label("Playback Toggle"))

        hk_row = QHBoxLayout()
        hk_row.setSpacing(8)
        self.hk_label = QLabel("Hotkey: ")
        self.hk_btn = QPushButton("Change")
        self.hk_btn.setToolTip("Click to bind a new hotkey for toggling playback")
        hk_row.addWidget(self.hk_label, 1)
        hk_row.addWidget(self.hk_btn)
        layout.addLayout(hk_row)

        layout.addStretch()
        return page

    def _make_system_page(self) -> QWidget:
        page = QWidget()
        layout = QVBoxLayout(page)
        layout.setContentsMargins(16, 12, 16, 12)
        layout.setSpacing(6)

        layout.addWidget(self._section_label("Updates"))
        self.check_update_btn = QPushButton("Check for updates")
        self.check_update_btn.setToolTip("Check GitHub for a newer version of HuMidi")
        layout.addWidget(self.check_update_btn)

        layout.addSpacing(12)
        layout.addWidget(self._section_label("Reset"))

        reset_desc = QLabel("Restore all playback and humanization settings to their defaults.")
        reset_desc.setProperty("role", "muted")
        reset_desc.setWordWrap(True)
        layout.addWidget(reset_desc)
        layout.addSpacing(4)
        self.reset_all_btn = QPushButton("Reset All Settings")
        self.reset_all_btn.setToolTip("Reset all playback and humanization settings to their default values")
        layout.addWidget(self.reset_all_btn)

        layout.addStretch()
        return page

    # ── Internal helpers ────────────────────────────────────────────────────────

    @staticmethod
    def _section_label(text: str) -> QLabel:
        lbl = QLabel(text)
        lbl.setProperty("role", "muted")
        return lbl

    def _switch_tab(self, idx: int) -> None:
        self._stack.setCurrentIndex(idx)
        for i, btn in enumerate(self._tab_btns):
            btn.setChecked(i == idx)

    # ── Public API ──────────────────────────────────────────────────────────────

    def _populate_theme_combo(self) -> None:
        active = ThemeManager.get_active_name()
        for name in ThemeManager.all_themes():
            self.theme_combo.addItem(name)
        idx = self.theme_combo.findText(active)
        if idx >= 0:
            self.theme_combo.setCurrentIndex(idx)

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
            'always_on_top':            self.always_top_check.isChecked(),
            'opacity':                  self.opacity_slider.value(),
            'show_timeline_visualizer': self.timeline_vis_check.isChecked(),
            'show_piano_visualizer':    self.piano_vis_check.isChecked(),
        }
