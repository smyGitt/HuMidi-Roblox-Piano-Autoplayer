import os
import sys
import webbrowser

from PyQt6.QtWidgets import (QWidget, QVBoxLayout, QHBoxLayout,
                             QCheckBox, QSlider, QLabel, QStackedWidget, QFrame,
                             QSizePolicy, QScrollArea, QApplication)
from PyQt6.QtCore import (Qt, QObject, QSize, QEvent, QTimer,
                          QVariantAnimation, QEasingCurve)
from PyQt6.QtGui import QColor, QCursor, QPixmap, QShortcut, QKeySequence

from ui.widgets import NavButton, DiscordNavButton, HuMidiButton, StatusIndicator
from ui.widgets.ph_icon import ph_icon
from ui.widgets.ph_icon_label import IconProvider
from ui.playback.PlaybackTab import PlaybackTab
from ui.settings.SettingsTab import SettingsTab
from ui.translator.TranslatorTab import TranslatorTab
from ui.visualizer.VisualizerTab import VisualizerTab
from ui.debug.DebugTab import DebugTab
from ui.license.LicenseTab import LicenseTab
from ui.theme import ThemeManager, generate_stylesheet


_W_SIDEBAR_COLLAPSED = 44   # label geometry starts at x=44; sidebar clips it to zero at this width
_W_SIDEBAR_EXPANDED  = 124


def _project_root() -> str:
    # Works both in development (ui/MainWindowUI.py -> project root) and
    # in PyInstaller bundles where sys._MEIPASS is the temp extraction dir.
    return getattr(sys, "_MEIPASS", os.path.join(os.path.dirname(__file__), ".."))


def _wrap_in_scroll(widget: QWidget) -> QScrollArea:
    scroll = QScrollArea()
    scroll.setWidgetResizable(True)
    scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
    scroll.setVerticalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAsNeeded)
    scroll.setWidget(widget)
    scroll.setFrameShape(QFrame.Shape.NoFrame)
    return scroll


class ElidingLabel(QLabel):
    """QLabel that truncates text with '...' when it doesn't fit."""
    def __init__(self, text="", parent=None):
        super().__init__(parent)
        self._full_text = text
        self.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Preferred)
        if text:
            self._update_elided()

    def setText(self, text):
        self._full_text = text
        self._update_elided()

    def resizeEvent(self, event):
        super().resizeEvent(event)
        self._update_elided()

    def _update_elided(self):
        width = self.contentsRect().width()
        if width <= 0:
            return
        elided = self.fontMetrics().elidedText(
            self._full_text, Qt.TextElideMode.ElideRight, width
        )
        super().setText(elided)


class MainWindowUI(QObject):
    def __init__(self, main_window):
        super().__init__()
        self.main_window = main_window
        self.setup_ui()

    def setup_ui(self):
        main_widget = QWidget()
        main_widget.setObjectName("main_widget")
        self.main_window.setCentralWidget(main_widget)
        main_layout = QVBoxLayout(main_widget)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(0)

        self._is_collapsed = False
        self._expanded_size = QSize(700, 600)
        self.main_window.setMinimumWidth(700)
        self.main_window.setMinimumHeight(600)

        # -- Collapsed mini strip ---------------------------------------------
        self._collapsed_strip = QFrame()
        self._collapsed_strip.setObjectName("collapsed_strip")
        self._collapsed_strip.setVisible(False)
        cs_layout = QVBoxLayout(self._collapsed_strip)
        cs_layout.setContentsMargins(12, 6, 12, 6)
        cs_layout.setSpacing(4)

        # Row 1: filename
        self._collapsed_file_label = ElidingLabel("No file selected.")
        self._collapsed_file_label.setObjectName("file_path_label")
        cs_layout.addWidget(self._collapsed_file_label)

        # Row 2: humanize checkbox
        self._collapsed_humanize_check = QCheckBox("Humanize")
        self._collapsed_humanize_check.setToolTip("Enable or disable all humanization at once")
        cs_layout.addWidget(self._collapsed_humanize_check)

        # Row 3: load buttons (icons set in apply_theme)
        self._collapsed_load_btn = HuMidiButton(tooltip="Open a MIDI file for playback")
        self._collapsed_load_btn.setObjectName("cs_load_btn")
        self._collapsed_load_btn.setIconSize(QSize(16, 16))
        self._collapsed_load_saved_btn = HuMidiButton(tooltip="Load a saved playback")
        self._collapsed_load_saved_btn.setObjectName("cs_load_saved_btn")
        self._collapsed_load_saved_btn.setIconSize(QSize(16, 16))

        cs_row3 = QHBoxLayout()
        cs_row3.setSpacing(5)
        cs_row3.addWidget(self._collapsed_load_btn, 1)
        cs_row3.addWidget(self._collapsed_load_saved_btn, 1)
        cs_layout.addLayout(cs_row3)

        # Rows 4-6 receive reparented transport widgets on collapse
        self._cs_scrubber_row = QWidget()
        self._cs_scrubber_layout = QVBoxLayout(self._cs_scrubber_row)
        self._cs_scrubber_layout.setContentsMargins(0, 0, 0, 0)
        self._cs_scrubber_layout.setSpacing(2)
        cs_layout.addWidget(self._cs_scrubber_row)

        self._cs_playback_row = QWidget()
        self._cs_playback_layout = QHBoxLayout(self._cs_playback_row)
        self._cs_playback_layout.setContentsMargins(0, 0, 0, 0)
        self._cs_playback_layout.setSpacing(5)
        self._cs_playback_layout.addStretch()  # stretch between stop and save -- populated on collapse
        cs_layout.addWidget(self._cs_playback_row)

        self._cs_expand_row = QWidget()
        self._cs_expand_layout = QHBoxLayout(self._cs_expand_row)
        self._cs_expand_layout.setContentsMargins(0, 0, 0, 0)
        self._cs_expand_layout.setSpacing(0)
        cs_layout.addWidget(self._cs_expand_row)

        self._cs_layout = cs_layout
        main_layout.addWidget(self._collapsed_strip)

        # -- Body: sidebar + page stack ---------------------------------------
        self._body = QWidget()
        body_layout = QHBoxLayout(self._body)
        body_layout.setSpacing(0)

        sidebar = QFrame(self._body)
        sidebar.setObjectName("sidebar")
        sidebar.setFixedWidth(_W_SIDEBAR_COLLAPSED)
        sidebar_vbox = QVBoxLayout(sidebar)
        sidebar_vbox.setContentsMargins(0, 0, 0, 0)
        sidebar_vbox.setSpacing(0)

        # Logo row -- same fixed-geometry pattern as NavButton.
        # Icon at x=12, "HuMidi" text at x=44. Sidebar clips the text when
        # collapsed; both are fully visible when expanded.
        # Probe candidates in priority order; first match wins.
        _root = _project_root()
        for _candidate in (
            os.path.join(_root, "assets", "humidi_logo.png"),
            os.path.join(_root, "icon.png"),
            os.path.join(_root, "icon.ico"),
        ):
            if os.path.exists(_candidate):
                _logo_src = _candidate
                break
        else:
            _logo_src = os.path.join(_root, "icon.ico")
        _logo_raw = QPixmap(_logo_src)
        _dpr = QApplication.primaryScreen().devicePixelRatio()
        _logo_pix = _logo_raw.scaled(
            int(22 * _dpr), int(22 * _dpr),
            Qt.AspectRatioMode.KeepAspectRatio,
            Qt.TransformationMode.SmoothTransformation,
        )
        _logo_pix.setDevicePixelRatio(_dpr)
        logo_row = QFrame(sidebar)
        logo_row.setFixedHeight(48)

        logo_icon_lbl = QLabel(logo_row)
        logo_icon_lbl.setPixmap(_logo_pix)
        logo_icon_lbl.setGeometry(12, 13, 22, 22)
        logo_icon_lbl.setScaledContents(True)
        logo_icon_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        logo_icon_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

        logo_text_lbl = QLabel("Hu<i>Midi</i>", logo_row)
        logo_text_lbl.setObjectName("sidebar_logo_text")
        logo_text_lbl.setTextFormat(Qt.TextFormat.RichText)
        logo_text_lbl.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
        logo_text_lbl.setGeometry(44, 0, 200, 48)
        logo_text_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

        sidebar_vbox.addWidget(logo_row)

        self.tabs = QStackedWidget()
        self.tabs.currentChanged.connect(self._on_page_changed)

        _NAV_ITEMS = [
            ("music-note",   "Playback"),
            ("waveform",     "Visualizer"),
            ("translate",    "Translator"),
            ("gear-six",     "Settings"),
            ("bug",          "Debug"),
            ("certificate",  "About"),
        ]
        self._nav_btns: list[NavButton] = []
        for i, (icon_name, label) in enumerate(_NAV_ITEMS):
            btn = NavButton(icon_name, label)
            btn.clicked.connect(lambda idx=i: self._switch_page(idx))
            sidebar_vbox.addWidget(btn)
            self._nav_btns.append(btn)
            if i == 5:  # push Discord + GitHub to bottom edge after License
                sidebar_vbox.addStretch()
                self._status_indicator = StatusIndicator(sidebar)
                sidebar_vbox.addWidget(self._status_indicator)
                self._discord_btn = DiscordNavButton("https://discord.gg/bRaXP9gYZN")
                sidebar_vbox.addWidget(self._discord_btn)
                self._github_nav = NavButton("github-logo", "GitHub")
                self._github_nav.clicked.connect(
                    lambda: webbrowser.open("https://github.com/smyGitt/HuMidi")
                )
                sidebar_vbox.addWidget(self._github_nav)
        # Sidebar floats over the page stack; reserve its collapsed width as a left margin
        # so content is never obscured in the collapsed state.
        body_layout.setContentsMargins(_W_SIDEBAR_COLLAPSED, 0, 0, 0)
        body_layout.addWidget(self.tabs, 1)
        main_layout.addWidget(self._body, 1)

        self._sidebar = sidebar
        self._sidebar_expanded = False

        # Collapse delay timer -- fires when mouse leaves sidebar;
        # re-checks cursor position so moving to a child button doesn't collapse.
        self._sidebar_collapse_timer = QTimer(self)
        self._sidebar_collapse_timer.setSingleShot(True)
        self._sidebar_collapse_timer.setInterval(120)
        self._sidebar_collapse_timer.timeout.connect(self._check_sidebar_collapse)

        # Smooth width animation
        self._sidebar_anim = QVariantAnimation(self)
        self._sidebar_anim.setDuration(180)
        self._sidebar_anim.setEasingCurve(QEasingCurve.Type.OutCubic)
        self._sidebar_anim.valueChanged.connect(
            lambda v: self._sidebar.setFixedWidth(v)
        )
        sidebar.installEventFilter(self)
        self._body.installEventFilter(self)
        sidebar.raise_()

        # -- Pages ------------------------------------------------------------
        self.playback_tab   = PlaybackTab()
        self.visualizer_tab = VisualizerTab()
        self.translator_tab = TranslatorTab()
        self.settings_tab   = SettingsTab()
        self.debug_tab      = DebugTab()
        self.license_tab    = LicenseTab()

        self.tabs.addWidget(self.playback_tab)                      # 0
        self.tabs.addWidget(self.visualizer_tab)                   # 1
        self.tabs.addWidget(_wrap_in_scroll(self.translator_tab))  # 2
        self.tabs.addWidget(_wrap_in_scroll(self.settings_tab))    # 3
        self.tabs.addWidget(self.debug_tab)                        # 4
        self.tabs.addWidget(self.license_tab)                      # 5

        # Convenience aliases for frequently accessed sub-widgets
        self.log_output      = self.debug_tab.log_output
        self.timeline_widget = self.visualizer_tab.timeline_widget
        self.piano_widget    = self.visualizer_tab.piano_widget
        self.scroll_area     = self.visualizer_tab.scroll_area

        # -- Transport bar ----------------------------------------------------
        transport_bar = QFrame()
        transport_bar.setObjectName("transport_bar")
        transport_layout = QVBoxLayout(transport_bar)
        transport_layout.setContentsMargins(16, 10, 16, 10)
        transport_layout.setSpacing(6)

        # Scrubber row: [start_time | scrubber | end_time]
        scrubber_row = QWidget()
        scrubber_layout = QHBoxLayout(scrubber_row)
        scrubber_layout.setContentsMargins(0, 0, 0, 0)
        scrubber_layout.setSpacing(8)

        self.time_start_label = QLabel("00:00")
        self.time_start_label.setObjectName("time_start_label")
        self.time_start_label.setAlignment(
            Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter
        )
        self.time_start_label.setFixedWidth(38)

        self.scrubber_slider = QSlider(Qt.Orientation.Horizontal)
        self.scrubber_slider.setObjectName("scrubber_slider")
        self.scrubber_slider.setRange(0, 10000)
        self.scrubber_slider.sliderPressed.connect(self._on_scrubber_pressed)
        self.scrubber_slider.sliderMoved.connect(self._on_scrubber_moved)
        self.scrubber_slider.sliderReleased.connect(self._on_scrubber_released)
        self._scrubber_dragging = False

        self.time_end_label = QLabel("00:00")
        self.time_end_label.setObjectName("time_end_label")
        self.time_end_label.setAlignment(
            Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter
        )
        self.time_end_label.setFixedWidth(38)

        # Combined label kept for collapsed mode (hidden in expanded mode)
        self.time_label = QLabel("00:00 / 00:00")
        self.time_label.setObjectName("time_label")
        self.time_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.time_label.setVisible(False)

        scrubber_layout.addWidget(self.time_start_label)
        scrubber_layout.addWidget(self.scrubber_slider, 1)
        scrubber_layout.addWidget(self.time_end_label)
        transport_layout.addWidget(scrubber_row)

        # Button row
        self._btn_row_widget = QWidget()
        btn_row = QHBoxLayout(self._btn_row_widget)
        btn_row.setContentsMargins(0, 0, 0, 0)
        btn_row.setSpacing(5)

        self.play_button = HuMidiButton(tooltip="Start, pause, or resume playback.")
        self.play_button.setObjectName("play_button")

        self.stop_button = HuMidiButton(tooltip="Stop playback and return to the beginning.")
        self.stop_button.setObjectName("stop_button")

        self.save_button = HuMidiButton(
            tooltip="Save the current playback to a file so it can be replayed without re-processing the MIDI.",
        )
        self.save_button.setObjectName("save_button")

        btn_row.addWidget(self.play_button)
        btn_row.addWidget(self.stop_button)
        btn_row.addStretch()
        btn_row.addWidget(self.save_button)

        self.collapse_btn = HuMidiButton("▲  Collapse", tooltip="Collapse to mini mode (Ctrl+K)")
        self.collapse_btn.setObjectName("collapse_btn")
        self.collapse_btn.clicked.connect(self._toggle_collapsed)
        btn_row.addWidget(self.collapse_btn)

        transport_layout.addWidget(self._btn_row_widget)
        main_layout.addWidget(transport_bar)
        self._transport_bar = transport_bar
        self._transport_layout = transport_layout
        self._scrubber_row_widget = scrubber_row
        self._scrubber_layout = scrubber_layout

        self.play_button.setEnabled(False)
        self.stop_button.setEnabled(False)
        self.save_button.setEnabled(False)
        self.scrubber_slider.setEnabled(False)

        # Ctrl+K shortcut to toggle collapse
        self._collapse_shortcut = QShortcut(
            QKeySequence("Ctrl+K"), self.main_window
        )
        self._collapse_shortcut.activated.connect(self._toggle_collapsed)

        # -- Cross-cutting connections ----------------------------------------
        self.settings_tab.timeline_vis_check.toggled.connect(self._on_timeline_toggle)
        self.settings_tab.piano_vis_check.toggled.connect(self._on_piano_toggle)
        self.settings_tab.theme_combo.currentTextChanged.connect(self.apply_theme)
        self.settings_tab.theme_customize_btn.clicked.connect(self._open_theme_dialog)

        self._collapsed_humanize_check.toggled.connect(self._on_collapsed_humanize_toggled)
        self.playback_tab.select_all_humanization_check.toggled.connect(
            self._sync_collapsed_humanize
        )

        # Wire file strip action buttons
        self.playback_tab.file_strip.replace_requested.connect(
            lambda: self.playback_tab.browse_button.click()
        )

        self._switch_page(0)
        self._register_icon_labels()
        self.apply_theme(ThemeManager.get_active_name())

    # -- Icon provider registration -------------------------------------------

    def _register_icon_labels(self) -> None:
        """Register all PhIconLabel instances with IconProvider.

        Called once after the full UI is built. Each label receives a color_fn
        that derives its normal and hover hex values from a ThemeColors instance.
        Hover swap behavior is injected automatically by IconProvider.register().
        """
        provider = IconProvider.instance()

        # SettingsTab -- Files page section icons
        provider.register(
            self.settings_tab.save_dir_icon,
            lambda c: (c.text_secondary, c.text_primary),
        )
        provider.register(
            self.settings_tab.themes_file_icon,
            lambda c: (c.text_secondary, c.text_primary),
        )
        # SettingsTab -- edit-open action icons
        provider.register(
            self.settings_tab.save_edit_btn,
            lambda c: (c.text_secondary, c.accent),
        )
        provider.register(
            self.settings_tab.themes_edit_btn,
            lambda c: (c.text_secondary, c.accent),
        )

        # PlaybackTab -- file strip and drop zone decorative icons
        provider.register(
            self.playback_tab.file_strip.tile_icon,
            lambda c: (c.text_secondary, c.text_primary),
        )
        provider.register(
            self.playback_tab.drop_zone.drop_icon,
            lambda c: (c.text_secondary, c.text_primary),
        )

        # PlaybackTab -- card reset icons (danger: destructive action)
        provider.register(
            self.playback_tab.perf_reset_icon,
            lambda c: (c.text_secondary, c.accent_stop),
        )
        provider.register(
            self.playback_tab.opts_reset_icon,
            lambda c: (c.text_secondary, c.accent_stop),
        )
        provider.register(
            self.playback_tab.humanize_reset_icon,
            lambda c: (c.text_secondary, c.accent_stop),
        )
        provider.register(
            self.playback_tab.timing_reset_icon,
            lambda c: (c.text_secondary, c.accent_stop),
        )
        provider.register(
            self.playback_tab.hands_reset_icon,
            lambda c: (c.text_secondary, c.accent_stop),
        )

        # PlaybackTab -- saved songs panel title buttons
        provider.register(
            self.playback_tab.refresh_saved_songs_btn,
            lambda c: (c.text_secondary, c.text_primary),
        )
        provider.register(
            self.playback_tab.all_saves_btn,
            lambda c: (c.text_secondary, c.text_primary),
        )

    # -- Navigation -----------------------------------------------------------

    def _switch_page(self, index: int) -> None:
        self.tabs.setCurrentIndex(index)

    # -- Sidebar hover expand / collapse --------------------------------------

    def eventFilter(self, obj, event):
        if obj is self._sidebar:
            t = event.type()
            if t == QEvent.Type.Enter:
                self._sidebar_collapse_timer.stop()
                if not self._sidebar_expanded:
                    self._expand_sidebar()
            elif t == QEvent.Type.Leave:
                self._sidebar_collapse_timer.start()
        elif obj is self._body and event.type() == QEvent.Type.Resize:
            self._sidebar.setFixedHeight(self._body.height())
        return False

    def _expand_sidebar(self) -> None:
        self._sidebar_expanded = True
        self._sidebar.raise_()
        self._sidebar_anim.stop()
        self._sidebar_anim.setStartValue(self._sidebar.width())
        self._sidebar_anim.setEndValue(_W_SIDEBAR_EXPANDED)
        self._sidebar_anim.start()

    def _collapse_sidebar(self) -> None:
        self._sidebar_expanded = False
        self._sidebar_anim.stop()
        self._sidebar_anim.setStartValue(self._sidebar.width())
        self._sidebar_anim.setEndValue(_W_SIDEBAR_COLLAPSED)
        self._sidebar_anim.start()

    def _check_sidebar_collapse(self) -> None:
        cursor_local = self._sidebar.mapFromGlobal(QCursor.pos())
        if not self._sidebar.rect().contains(cursor_local):
            self._collapse_sidebar()

    # -------------------------------------------------------------------------

    def _on_page_changed(self, index: int) -> None:
        for i, btn in enumerate(self._nav_btns):
            btn.set_active(i == index)

    # -- Theme ----------------------------------------------------------------

    def apply_theme(self, name: str) -> None:
        themes = ThemeManager.all_themes()
        theme = themes.get(name)
        if theme is None:
            return
        ThemeManager.set_active_name(name)
        self.main_window.setStyleSheet(generate_stylesheet(theme))
        self._discord_btn.update_colors(theme.text_secondary, theme.text_primary)

        # Nav sidebar icons
        for btn in self._nav_btns:
            btn.update_icon_colors(theme.text_secondary, theme.text_primary)
        self._github_nav.update_icon_colors(theme.text_secondary, theme.text_primary)
        self._status_indicator.update_colors(theme.text_secondary, theme.accent_play, "#c44b4b", theme.accent_loaded)
        IconProvider.instance().notify_theme_changed(theme)

        # Collapsed strip load buttons
        _px = self._collapsed_load_btn.fontMetrics().height()
        self._collapsed_load_btn.setIconSize(QSize(_px, _px))
        self._collapsed_load_btn.setIcon(ph_icon("folder-open", theme.text_primary, _px))
        self._collapsed_load_saved_btn.setIconSize(QSize(_px, _px))
        self._collapsed_load_saved_btn.setIcon(ph_icon("floppy-disk", theme.text_primary, _px))

        # Transport button icons (stored for play/pause toggling in main.py)
        _ti = 22
        self._icon_play  = ph_icon("play",       theme.accent_play,  _ti)
        self._icon_pause = ph_icon("pause",      theme.accent_play,  _ti)
        self._icon_stop  = ph_icon("stop",       theme.accent_stop,  _ti)
        self._icon_save  = ph_icon("floppy-disk", theme.accent_save,   _ti)
        self.play_button.setIconSize(QSize(_ti, _ti))
        self.play_button.setIcon(self._icon_play)
        self.stop_button.setIconSize(QSize(_ti, _ti))
        self.stop_button.setIcon(self._icon_stop)
        self.save_button.setIconSize(QSize(_ti, _ti))
        self.save_button.setIcon(self._icon_save)
        self._icon_collapse = ph_icon("resize-collapse", theme.text_secondary, _ti)
        self._icon_expand   = ph_icon("resize-expand",   theme.text_secondary, _ti)
        self.collapse_btn.setIconSize(QSize(_ti, _ti))
        self.collapse_btn.setText("")
        icon = self._icon_expand if self._is_collapsed else self._icon_collapse
        self.collapse_btn.setIcon(icon)

        self.playback_tab._drop_card.set_colors(theme.border, theme.accent)
        self.playback_tab.redraw_saved_song_cards()

        self.timeline_widget.left_hand_color.setNamedColor(theme.accent)
        self.timeline_widget.left_hand_color.setAlpha(210)
        self.timeline_widget.right_hand_color.setNamedColor(theme.accent_play)
        self.timeline_widget.right_hand_color.setAlpha(210)
        self.timeline_widget.bg_color.setNamedColor(theme.bg_primary)
        pedal_q = QColor(theme.pedal_color)
        pedal_q.setAlpha(180)
        self.timeline_widget.pedal_color = pedal_q
        self.timeline_widget.cached_background = None
        self.timeline_widget.update()
        piano_pedal_q = QColor(theme.pedal_color)
        self.piano_widget.pedal_color = piano_pedal_q
        self.piano_widget.update()

    def _open_theme_dialog(self) -> None:
        from ui.dialogs.ThemeDialog import ThemeDialog
        dlg = ThemeDialog(self.main_window)
        dlg.theme_applied.connect(self._on_theme_dialog_accepted)
        dlg.exec()

    def _on_theme_dialog_accepted(self, name: str) -> None:
        self.settings_tab.refresh_theme_combo()
        self.apply_theme(name)

    # -- Visualizer helpers ---------------------------------------------------

    def _on_timeline_toggle(self, checked: bool) -> None:
        self.scroll_area.setVisible(checked)
        self._update_visualizer_availability()

    def _on_piano_toggle(self, checked: bool) -> None:
        self.piano_widget.setVisible(checked)
        self.timeline_widget.set_show_pedal(checked)
        self._update_visualizer_availability()

    def _update_visualizer_availability(self) -> None:
        both_off = (not self.settings_tab.timeline_vis_check.isChecked() and
                    not self.settings_tab.piano_vis_check.isChecked())
        self._nav_btns[1].setEnabled(not both_off)
        if both_off and self.tabs.currentIndex() == 1:
            self._switch_page(0)

    def update_progress(self, current_time, total_duration):
        if self.scroll_area.isVisible() and not self.timeline_widget.is_dragging:
            self.timeline_widget.set_position(current_time)
            if total_duration > 0:
                ratio = current_time / total_duration
                cursor_x = ratio * self.timeline_widget.width()
                target_scroll = cursor_x - (self.scroll_area.width() / 2)
                self.scroll_area.horizontalScrollBar().setValue(int(target_scroll))

        if not self._scrubber_dragging and not self.timeline_widget.is_dragging:
            self.scrubber_slider.blockSignals(True)
            if total_duration > 0:
                self.scrubber_slider.setValue(
                    int(current_time / total_duration * 10000)
                )
            self.scrubber_slider.blockSignals(False)

        self.update_time_label(current_time, total_duration)

    def reset_timeline_position(self) -> None:
        self.timeline_widget.current_time = 0.0
        self.scrubber_slider.blockSignals(True)
        self.scrubber_slider.setValue(0)
        self.scrubber_slider.blockSignals(False)

    def update_time_label(self, current, total) -> None:
        def fmt(s):
            m, sec = int(s // 60), int(s % 60)
            return f"{m:02d}:{sec:02d}"
        self.time_start_label.setText(fmt(current))
        self.time_end_label.setText(fmt(total))
        self.time_label.setText(f"{fmt(current)} / {fmt(total)}")

    # -- Scrubber -------------------------------------------------------------

    def _on_scrubber_pressed(self):
        self._scrubber_dragging = True

    def _on_scrubber_moved(self, value):
        if self.timeline_widget.total_duration > 0:
            t = (value / 10000.0) * self.timeline_widget.total_duration
            self.timeline_widget.current_time = t
            self.timeline_widget.scrub_position_changed.emit(t)
            self.update_time_label(t, self.timeline_widget.total_duration)

    def _on_scrubber_released(self):
        self._scrubber_dragging = False
        self.timeline_widget.seek_requested.emit(self.timeline_widget.current_time)

    # -- Collapse -------------------------------------------------------------

    def _toggle_collapsed(self) -> None:
        self._is_collapsed = not self._is_collapsed
        if self._is_collapsed:
            self._expanded_size = self.main_window.size()
            self._body.setVisible(False)
            self._collapsed_strip.setVisible(True)
            self.collapse_btn.setIcon(self._icon_expand)
            self.collapse_btn.setToolTip("Restore full window (Ctrl+K)")
            self.collapse_btn.setMinimumWidth(0)
            self.collapse_btn.setMaximumWidth(16777215)
            self.collapse_btn.setProperty("strip_mode", True)
            self.collapse_btn.style().unpolish(self.collapse_btn)
            self.collapse_btn.style().polish(self.collapse_btn)
            # Show combined time label in collapsed scrubber row
            self.time_start_label.setVisible(False)
            self.time_end_label.setVisible(False)
            self.time_label.setVisible(True)
            # Row 4: scrubber then combined time label stacked vertically
            self._cs_scrubber_layout.addWidget(self.scrubber_slider)
            self._cs_scrubber_layout.addWidget(self.time_label)
            # Row 5: play | stop | [stretch] | save -- same buttons as transport bar
            # _cs_playback_layout has a stretch at index 0 from setup_ui
            self._cs_playback_layout.insertWidget(0, self.play_button)
            self._cs_playback_layout.insertWidget(1, self.stop_button)
            self._cs_playback_layout.addWidget(self.save_button)
            # Row 6: expand button full width
            self._cs_expand_layout.addWidget(self.collapse_btn)
            self._transport_bar.setVisible(False)
            self.main_window.setMinimumWidth(0)
            self.main_window.setMinimumHeight(0)
            self.main_window.resize(270, 300)
        else:
            self._body.setVisible(True)
            self._collapsed_strip.setVisible(False)
            self.collapse_btn.setIcon(self._icon_collapse)
            self.collapse_btn.setToolTip("Collapse to mini mode (Ctrl+K)")
            self.collapse_btn.setProperty("strip_mode", False)
            self.collapse_btn.style().unpolish(self.collapse_btn)
            self.collapse_btn.style().polish(self.collapse_btn)
            # Restore combined time label visibility
            self.time_label.setVisible(False)
            self.time_start_label.setVisible(True)
            self.time_end_label.setVisible(True)
            # Restore all reparented widgets back into the transport bar
            self._scrubber_layout.insertWidget(1, self.scrubber_slider)
            btn_row_layout = self._btn_row_widget.layout()
            btn_row_layout.insertWidget(0, self.play_button)
            btn_row_layout.insertWidget(1, self.stop_button)
            # stretch spacer remains at index 2; restore save after it
            btn_row_layout.insertWidget(3, self.save_button)
            btn_row_layout.addWidget(self.collapse_btn)
            self._transport_bar.setVisible(True)
            self.main_window.setMinimumWidth(700)
            self.main_window.setMinimumHeight(600)
            self.main_window.resize(self._expanded_size)

    # -- Collapsed-strip humanize sync ----------------------------------------

    def _on_collapsed_humanize_toggled(self, checked: bool) -> None:
        sel = self.playback_tab.select_all_humanization_check
        sel.blockSignals(True)
        sel.setChecked(checked)
        sel.blockSignals(False)
        self.playback_tab._toggle_all(checked)

    def _sync_collapsed_humanize(self, checked: bool) -> None:
        self._collapsed_humanize_check.blockSignals(True)
        self._collapsed_humanize_check.setChecked(checked)
        self._collapsed_humanize_check.blockSignals(False)

    # -- Public API -----------------------------------------------------------

    def update_file_label(self, text: str, tooltip: str = "") -> None:
        self.playback_tab.update_file_label(text, tooltip)
        self._collapsed_file_label.setText(text)

    def set_controls_enabled(self, enabled: bool, ignore_if_loaded: bool = False) -> None:
        self.playback_tab.set_groups_enabled(
            enabled,
            skip_playback_humanization=(ignore_if_loaded and enabled)
        )

    def _set_save_enabled(self, val: bool) -> None:
        self.save_button.setEnabled(val)

    def reset_controls_to_default(self) -> None:
        self.playback_tab.reset_to_default()

    def load_config_to_ui(self, config: dict, save_dir: str) -> None:
        self.playback_tab.load_config(config)
        self.settings_tab.load_config(config, save_dir)

    def gather_playback_config(self) -> dict:
        cfg = self.playback_tab.gather_playback_config()
        cfg['use_ai_pedal'] = False  # AI pedal driven by pedal_style='ai', not this flag
        return cfg

    def gather_app_config(self) -> dict:
        return {**self.playback_tab.gather_app_config(), **self.settings_tab.gather_config()}

    def update_enabled_states(self) -> None:
        self.playback_tab.update_enabled_states()
