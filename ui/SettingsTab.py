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

        # --- Nav bar (sub_tab_bar / sub_tab_btn -- same style as PlaybackTab) ---
        nav_bar = QFrame()
        nav_bar.setObjectName("sub_tab_bar")
        nav_bar.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        nav_layout = QHBoxLayout(nav_bar)
        nav_layout.setContentsMargins(0, 0, 0, 0)
        nav_layout.setSpacing(0)

        self._tab_btns = []
        for i, name in enumerate(["Display", "Files", "Hotkey", "System"]):
            btn = QPushButton(name)
            btn.setObjectName("sub_tab_btn")
            btn.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
            btn.setCursor(Qt.CursorShape.PointingHandCursor)
            btn.setProperty("active", "false")
            btn.clicked.connect(lambda checked=False, idx=i: self._switch_tab(idx))
            nav_layout.addWidget(btn)
            self._tab_btns.append(btn)
        nav_layout.addStretch()
        outer.addWidget(nav_bar)

        # --- Content stack (fills remaining space, no card wrapper) ---
        self._stack = QStackedWidget()
        self._stack.addWidget(self._make_display_page())
        self._stack.addWidget(self._make_files_page())
        self._stack.addWidget(self._make_hotkey_page())
        self._stack.addWidget(self._make_system_page())
        outer.addWidget(self._stack, 1)

        self._switch_tab(0)

    # ── Page builders ───────────────────────────────────────────────────────────

    def _make_display_page(self) -> QWidget:
        page = QWidget()
        page_layout = QVBoxLayout(page)
        page_layout.setContentsMargins(12, 12, 12, 12)
        page_layout.setSpacing(0)

        card, body = make_card("")
        body.setSpacing(6)

        body.addWidget(self._section_label("Window"))
        self.always_top_check = QCheckBox("Always on Top")
        self.always_top_check.setToolTip("Keep this window above all other windows")
        body.addWidget(self.always_top_check)

        opacity_row = QHBoxLayout()
        opacity_row.setSpacing(8)
        opacity_row.addWidget(QLabel("Opacity"))
        self.opacity_slider = QSlider(Qt.Orientation.Horizontal)
        self.opacity_slider.setRange(20, 100)
        self.opacity_slider.setValue(100)
        self.opacity_slider.setToolTip("Adjust window transparency (20-100%)")
        opacity_row.addWidget(self.opacity_slider, 1)
        body.addLayout(opacity_row)

        body.addSpacing(8)
        body.addWidget(self._section_label("Visualizer"))
        self.timeline_vis_check = QCheckBox("Timeline")
        self.timeline_vis_check.setChecked(True)
        self.timeline_vis_check.setToolTip(
            "Show the piano-roll timeline in the Visualizer tab "
            "(disable for a simple seek slider)"
        )
        self.piano_vis_check = QCheckBox("Piano Keys")
        self.piano_vis_check.setChecked(True)
        self.piano_vis_check.setToolTip("Show the piano key visualizer in the Visualizer tab")
        body.addWidget(self.timeline_vis_check)
        body.addWidget(self.piano_vis_check)

        body.addSpacing(8)
        body.addWidget(self._section_label("Appearance"))
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
        body.addLayout(theme_row)

        page_layout.addWidget(card, 1)
        return page

    def _make_files_page(self) -> QWidget:
        page = QWidget()
        page_layout = QVBoxLayout(page)
        page_layout.setContentsMargins(12, 12, 12, 12)
        page_layout.setSpacing(0)

        card, body = make_card("")
        body.setSpacing(6)

        body.addWidget(self._section_label("Save Directory"))
        save_row = QHBoxLayout()
        save_row.setSpacing(8)
        self.save_path_input = QLineEdit()
        self.save_path_input.setReadOnly(True)
        self.save_path_input.setToolTip("Directory where humanized performance saves are stored")
        self.save_browse_btn = QPushButton("Browse")
        self.save_browse_btn.setToolTip("Choose where to save humanized performance files")
        save_row.addWidget(self.save_path_input, 1)
        save_row.addWidget(self.save_browse_btn)
        body.addLayout(save_row)

        page_layout.addWidget(card, 1)
        return page

    def _make_hotkey_page(self) -> QWidget:
        page = QWidget()
        page_layout = QVBoxLayout(page)
        page_layout.setContentsMargins(12, 12, 12, 12)
        page_layout.setSpacing(0)

        card, body = make_card("")
        body.setSpacing(6)

        body.addWidget(self._section_label("Playback Toggle"))
        hk_row = QHBoxLayout()
        hk_row.setSpacing(8)
        self.hk_label = QLabel("Hotkey: ")
        self.hk_btn = QPushButton("Change")
        self.hk_btn.setToolTip("Click to bind a new hotkey for toggling playback")
        hk_row.addWidget(self.hk_label, 1)
        hk_row.addWidget(self.hk_btn)
        body.addLayout(hk_row)

        page_layout.addWidget(card, 1)
        return page

    def _make_system_page(self) -> QWidget:
        page = QWidget()
        page_layout = QVBoxLayout(page)
        page_layout.setContentsMargins(12, 12, 12, 12)
        page_layout.setSpacing(0)

        card, body = make_card("")
        body.setSpacing(6)

        body.addWidget(self._section_label("Updates"))
        self.check_update_btn = QPushButton("Check for updates")
        self.check_update_btn.setToolTip("Check GitHub for a newer version of HuMidi")
        body.addWidget(self.check_update_btn)

        body.addSpacing(12)
        body.addWidget(self._section_label("Reset"))
        reset_desc = QLabel("Restore all playback and humanization settings to their defaults.")
        reset_desc.setProperty("role", "muted")
        reset_desc.setWordWrap(True)
        body.addWidget(reset_desc)
        body.addSpacing(4)
        self.reset_all_btn = QPushButton("Reset All Settings")
        self.reset_all_btn.setToolTip("Reset all playback and humanization settings to their default values")
        body.addWidget(self.reset_all_btn)

        page_layout.addWidget(card, 1)
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
            active = i == idx
            btn.setProperty("active", "true" if active else "false")
            btn.style().unpolish(btn)
            btn.style().polish(btn)
            btn.update()

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
