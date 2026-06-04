"""
ThemeDialog — select from built-in presets, create / edit / delete custom themes.

Live-preview: clicking any theme in the list immediately applies it to the
parent window so you can see it in context.  Clicking Cancel reverts to
whatever was active when the dialog was opened.
"""

from __future__ import annotations
from dataclasses import replace

import json
from pathlib import Path

from PyQt6.QtWidgets import (
    QDialog, QHBoxLayout, QVBoxLayout,
    QLabel, QLineEdit, QPushButton, QScrollArea, QWidget,
    QDialogButtonBox, QFrame, QMessageBox, QFileDialog,
    QCheckBox, QSlider, QSpinBox, QDoubleSpinBox, QComboBox,
    QApplication, QColorDialog, QInputDialog,
)
from PyQt6.QtCore import (
    Qt, QSize, QByteArray, pyqtSignal as Signal, QEvent, QObject,
    QPropertyAnimation, QEasingCurve,
)
from PyQt6.QtGui import QColor, QIcon, QPainter, QPixmap, QCursor
from PyQt6.QtSvg import QSvgRenderer

from ui.theme import ThemeColors, ThemeManager, generate_stylesheet, BUILTIN_THEMES, _mix
from ui.widgets.ph_icon import ph_icon


# ── Inline SVG icons for export / import ─────────────────────────────────────

_SVG_EXPORT = (
    b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    b'<rect width="256" height="256" fill="none"/>'
    b'<path d="M200,224H56a8,8,0,0,1-8-8V40a8,8,0,0,1,8-8h96l56,56V216A8,8,0,0,1,200,224Z"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<polyline points="152 32 152 88 208 88"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="128" y1="120" x2="128" y2="184"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<polyline points="104 160 128 184 152 160"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'</svg>'
)

_SVG_IMPORT = (
    b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    b'<rect width="256" height="256" fill="none"/>'
    b'<path d="M200,224H56a8,8,0,0,1-8-8V40a8,8,0,0,1,8-8h96l56,56V216A8,8,0,0,1,200,224Z"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<polyline points="152 32 152 88 208 88"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<polyline points="104 144 128 120 152 144"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="128" y1="184" x2="128" y2="120"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'</svg>'
)


_SVG_NEW = (
    b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    b'<rect width="256" height="256" fill="none"/>'
    b'<rect x="40" y="40" width="176" height="176" rx="8" opacity="0.2"/>'
    b'<rect x="40" y="40" width="176" height="176" rx="8"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="88" y1="128" x2="168" y2="128"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="128" y1="88" x2="128" y2="168"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'</svg>'
)

_SVG_DELETE = (
    b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    b'<rect width="256" height="256" fill="none"/>'
    b'<path d="M200,56V208a8,8,0,0,1-8,8H64a8,8,0,0,1-8-8V56Z" opacity="0.2"/>'
    b'<line x1="216" y1="56" x2="40" y2="56"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="104" y1="104" x2="104" y2="168"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="152" y1="104" x2="152" y2="168"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<path d="M200,56V208a8,8,0,0,1-8,8H64a8,8,0,0,1-8-8V56"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<path d="M168,56V40a16,16,0,0,0-16-16H104A16,16,0,0,0,88,40V56"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'</svg>'
)


_SVG_INSPECT = (
    b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    b'<rect width="256" height="256" fill="none"/>'
    b'<circle cx="128" cy="128" r="48" opacity="0.2" fill="currentColor"/>'
    b'<circle cx="128" cy="128" r="48"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="128" y1="32" x2="128" y2="80"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="16"/>'
    b'<line x1="128" y1="176" x2="128" y2="224"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="16"/>'
    b'<line x1="32" y1="128" x2="80" y2="128"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="16"/>'
    b'<line x1="176" y1="128" x2="224" y2="128"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-width="16"/>'
    b'</svg>'
)

# Caret-right with end-bar: opens the color-swatch side panel.
_SVG_EXPAND_PANEL = (
    b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    b'<rect width="256" height="256" fill="none"/>'
    b'<polygon points="112 56 184 128 112 200 112 56" opacity="0.2"/>'
    b'<line x1="32" y1="128" x2="112" y2="128"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<polygon points="112 56 184 128 112 200 112 56"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="216" y1="40" x2="216" y2="216"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'</svg>'
)

# Caret-left with end-bar: closes the color-swatch side panel.
_SVG_COLLAPSE_PANEL = (
    b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    b'<rect width="256" height="256" fill="none"/>'
    b'<polygon points="144 56 72 128 144 200 144 56" opacity="0.2"/>'
    b'<line x1="224" y1="128" x2="144" y2="128"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<polygon points="144 56 72 128 144 200 144 56"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="40" y1="40" x2="40" y2="216"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'</svg>'
)

# Pencil-on-document: rename the selected theme.
_SVG_RENAME = (
    b'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    b'<rect width="256" height="256" fill="none"/>'
    b'<polygon points="128 160 96 160 96 128 168 56 200 88 128 160" opacity="0.2"/>'
    b'<polygon points="128 160 96 160 96 128 192 32 224 64 128 160"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<line x1="168" y1="56" x2="200" y2="88"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'<path d="M216,128v80a8,8,0,0,1-8,8H48a8,8,0,0,1-8-8V48a8,8,0,0,1,8-8h80"'
    b' fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    b'</svg>'
)

# Fully-expanded width of the swatch side panel in pixels.
_SWATCH_WIDTH = 280


def _svg_icon(svg_bytes: bytes, color: str, size: int = 16) -> QIcon:
    """Render an inline SVG (using currentColor) as a QIcon at the given logical size."""
    data = svg_bytes.replace(b"currentColor", color.encode())
    renderer = QSvgRenderer(QByteArray(data))
    phys = size * 2
    pix = QPixmap(phys, phys)
    pix.fill(Qt.GlobalColor.transparent)
    painter = QPainter(pix)
    painter.setRenderHint(QPainter.RenderHint.Antialiasing)
    painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
    renderer.render(painter)
    painter.end()
    icon = QIcon()
    icon.addPixmap(pix)
    return icon


# ── Colour labels (order matters — shown in the editor) ───────────────────

_COLOR_FIELDS = [
    ("bg_primary",    "Background"),
    ("bg_secondary",  "Surface"),
    ("bg_input",      "Input Fields"),
    ("accent",        "Accent"),
    ("text_primary",  "Text"),
    ("text_secondary","Muted Text"),
    ("border",        "Borders"),
    ("accent_play",   "Play Color"),
    ("accent_stop",   "Stop / Danger"),
    ("pedal_color",   "Pedal Color"),
]


def _field_for_widget(widget: QWidget) -> tuple[str, str] | None:
    """Return (ThemeColors field, display label) for a preview widget, or None if unrecognised."""
    obj_name = widget.objectName()
    cls_name = type(widget).__name__

    _by_name: dict[str, tuple[str, str]] = {
        # Buttons and indicators
        "pedal_swatch":         ("pedal_color",    "Pedal Color"),
        "play_button":          ("accent_play",    "Play Color"),
        "stop_button":          ("accent_stop",    "Stop / Danger"),
        "save_button":          ("accent",         "Accent"),
        # Window chrome
        "preview_window":       ("bg_primary",     "Background"),
        "collapsed_strip":      ("bg_secondary",   "Surface"),
        "sidebar_logo_text":    ("text_primary",   "Text"),
        # File strip
        "file_strip":           ("bg_secondary",   "Surface"),
        "file_strip_tile":      ("accent",         "Accent"),
        "file_strip_tile_icon": ("accent",         "Accent"),
        "file_strip_name":      ("text_primary",   "Text"),
        "file_strip_meta":      ("text_secondary", "Muted Text"),
        # Sub-tab bar
        "sub_tab_bar":          ("bg_primary",     "Background"),
        # Cards
        "section_card":         ("bg_secondary",   "Surface"),
        "part_card":            ("bg_input",       "Input Fields"),
        "save_card":            ("bg_input",       "Input Fields"),
        "stats_tile":           ("bg_input",       "Input Fields"),
        "part_card_title":      ("text_primary",   "Text"),
        "part_card_meta":       ("text_secondary", "Muted Text"),
        "stats_tile_value":     ("text_primary",   "Text"),
        "stats_tile_label":     ("text_secondary", "Muted Text"),
        # Transport bar
        "transport_bar":        ("bg_secondary",   "Surface"),
        "time_start_label":     ("text_primary",   "Text"),
        "time_end_label":       ("text_secondary", "Muted Text"),
    }
    if obj_name in _by_name:
        return _by_name[obj_name]

    # Sub-tab buttons: active state shows the accent underline; inactive shows muted text color.
    if obj_name == "sub_tab_btn":
        if widget.property("active") == "true":
            return ("accent", "Accent")
        return ("text_secondary", "Muted Text")

    # Card-reset icon buttons render their glyph in the muted-text color.
    if widget.property("role") == "card_reset":
        return ("text_secondary", "Muted Text")

    if cls_name == "QLabel":
        role = widget.property("role")
        if role in ("muted", "placeholder", "section"):
            return ("text_secondary", "Muted Text")
        return ("text_primary", "Text")

    _by_class: dict[str, tuple[str, str]] = {
        "QLineEdit":      ("bg_input",     "Input Fields"),
        "QComboBox":      ("bg_input",     "Input Fields"),
        "QDoubleSpinBox": ("bg_input",     "Input Fields"),
        "QSpinBox":       ("bg_input",     "Input Fields"),
        "QSlider":        ("accent",       "Accent"),
        "QCheckBox":      ("accent",       "Accent"),
        "QPushButton":    ("bg_secondary", "Surface"),
        "QFrame":         ("bg_primary",   "Background"),
        "QWidget":        ("bg_primary",   "Background"),
    }
    return _by_class.get(cls_name)


# ── Color swatch widget ────────────────────────────────────────────────────

class _ColorSwatch(QWidget):
    """A coloured square button + hex text field, kept in sync."""

    colorChanged = Signal(str)   # emits lowercase hex like "#aabbcc"

    def __init__(self, parent=None):
        super().__init__(parent)
        self._color = "#000000"
        self._editable = True

        layout = QHBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)
        layout.setSpacing(6)

        self._swatch = QPushButton()
        self._swatch.setFixedSize(24, 24)
        self._swatch.setCursor(Qt.CursorShape.PointingHandCursor)
        self._swatch.clicked.connect(self._pick_color)

        self._hex = QLineEdit()
        self._hex.setMaxLength(7)
        self._hex.setPlaceholderText("#rrggbb")
        self._hex.textEdited.connect(self._on_hex_edited)

        layout.addWidget(self._swatch)
        layout.addWidget(self._hex)

    # ── Public ────────────────────────────────────────────────────────

    def set_color(self, hex_color: str, emit: bool = False) -> None:
        self._color = hex_color.lower()
        self._hex.blockSignals(True)
        self._hex.setText(hex_color)
        self._hex.blockSignals(False)
        self._refresh_swatch()
        if emit:
            self.colorChanged.emit(self._color)

    def color(self) -> str:
        return self._color

    def set_editable(self, editable: bool) -> None:
        self._editable = editable
        self._swatch.setEnabled(editable)
        self._hex.setReadOnly(not editable)

    # ── Internals ─────────────────────────────────────────────────────

    def _pick_color(self) -> None:
        if not self._editable:
            return
        initial = QColor(self._color)
        color = QColorDialog.getColor(initial, self, "Choose colour")
        if color.isValid():
            self.set_color(color.name(), emit=True)

    def _on_hex_edited(self, text: str) -> None:
        text = text.strip()
        if not text.startswith("#"):
            text = "#" + text
        if len(text) == 7:
            try:
                QColor(text)          # validates hex
                self._color = text.lower()
                self._refresh_swatch()
                self.colorChanged.emit(self._color)
            except Exception:
                pass

    def _refresh_swatch(self) -> None:
        c = self._color
        r, g, b = int(c[1:3], 16), int(c[3:5], 16), int(c[5:7], 16)
        lum = 0.299 * r + 0.587 * g + 0.114 * b
        border = "#555555" if lum > 127 else "#aaaaaa"
        self._swatch.setStyleSheet(
            f"QPushButton {{ background-color: {c}; border: 1px solid {border}; "
            f"border-radius: 12px; "
            f"min-width: 24px; max-width: 24px; min-height: 24px; max-height: 24px; padding: 0; }}"
            f"QPushButton:hover {{ border: 2px solid {border}; }}"
        )


# ── Inspect mode event filter ─────────────────────────────────────────────

class _InspectFilter(QObject):
    """App-level event filter: intercepts right-click-release inside the preview panel."""

    widget_right_clicked = Signal(object, object)  # (QWidget, QPoint global)

    def __init__(self, root: QWidget, parent=None):
        super().__init__(parent)
        self._root = root
        self._active = False

    def set_active(self, active: bool) -> None:
        self._active = active

    def _inside_root(self, widget: QWidget) -> bool:
        w = widget
        while w is not None:
            if w is self._root:
                return True
            w = w.parent()
        return False

    def eventFilter(self, obj, event) -> bool:
        if not self._active or not isinstance(obj, QWidget):
            return False
        if event.type() != QEvent.Type.MouseButtonRelease:
            return False
        if event.button() != Qt.MouseButton.RightButton:
            return False
        if not self._inside_root(obj):
            return False
        self.widget_right_clicked.emit(obj, event.globalPosition().toPoint())
        return True


# ── Theme dialog ───────────────────────────────────────────────────────────

class ThemeDialog(QDialog):
    """
    Lets the user pick a built-in theme or create / edit / delete custom ones.

    The parent window's stylesheet is updated live as themes are selected.
    Cancelling the dialog reverts the stylesheet to what it was on open.
    """

    theme_applied = Signal(str)   # emits name when user accepts

    def __init__(self, main_window, parent=None):
        super().__init__(parent)
        self.main_window = main_window
        self._previous_name = ThemeManager.get_active_name()
        self._current_theme: ThemeColors | None = None
        self._pending_save = False
        self._active_color_dlg: QColorDialog | None = None

        self.setWindowTitle("Theme Manager")
        self.setMinimumSize(600, 540)
        self._build_ui()

        self._inspect_filter = _InspectFilter(self._preview_panel, self)
        self._inspect_filter.widget_right_clicked.connect(self._on_widget_picked)
        QApplication.instance().installEventFilter(self._inspect_filter)
        self.finished.connect(self._cleanup_inspect)

        self._apply_own_stylesheet()
        self._sync_toolbar_btn_sizes()
        self._populate_list()

    # ── Layout ────────────────────────────────────────────────────────

    def _build_ui(self) -> None:
        outer = QVBoxLayout(self)
        outer.setContentsMargins(12, 12, 12, 12)
        outer.setSpacing(8)

        # -- Top toolbar: theme selector + action buttons -----------------
        toolbar = QHBoxLayout()
        toolbar.setSpacing(4)

        self._expand_panel_btn = QPushButton()
        self._expand_panel_btn.setFixedSize(28, 28)
        self._expand_panel_btn.setIconSize(QSize(16, 16))
        self._expand_panel_btn.setProperty("role", "icon_btn")
        self._expand_panel_btn.setToolTip("Open color editor")
        self._expand_panel_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._expand_panel_btn.clicked.connect(lambda: self._toggle_swatch_panel(True))
        toolbar.addWidget(self._expand_panel_btn, 0, Qt.AlignmentFlag.AlignVCenter)

        self._combo = QComboBox()
        self._combo.setSizeAdjustPolicy(
            QComboBox.SizeAdjustPolicy.AdjustToMinimumContentsLengthWithIcon
        )
        self._combo.currentIndexChanged.connect(self._on_row_changed)
        toolbar.addWidget(self._combo, 1, Qt.AlignmentFlag.AlignVCenter)

        self._new_btn = QPushButton()
        self._new_btn.setFixedSize(28, 28)
        self._new_btn.setIconSize(QSize(16, 16))
        self._new_btn.setProperty("role", "icon_btn")
        self._new_btn.setToolTip("Duplicate selected theme as a new custom preset")
        self._new_btn.clicked.connect(self._on_new)

        self._del_btn = QPushButton()
        self._del_btn.setFixedSize(28, 28)
        self._del_btn.setIconSize(QSize(16, 16))
        self._del_btn.setProperty("role", "icon_btn")
        self._del_btn.setToolTip("Delete this custom theme (built-in themes cannot be deleted)")
        self._del_btn.setEnabled(False)
        self._del_btn.clicked.connect(self._on_delete)

        self._rename_btn = QPushButton()
        self._rename_btn.setFixedSize(28, 28)
        self._rename_btn.setIconSize(QSize(16, 16))
        self._rename_btn.setProperty("role", "icon_btn")
        self._rename_btn.setToolTip("Rename this custom theme")
        self._rename_btn.setEnabled(False)
        self._rename_btn.clicked.connect(self._on_rename)

        toolbar.addWidget(self._new_btn, 0, Qt.AlignmentFlag.AlignVCenter)
        toolbar.addWidget(self._del_btn, 0, Qt.AlignmentFlag.AlignVCenter)
        toolbar.addWidget(self._rename_btn, 0, Qt.AlignmentFlag.AlignVCenter)

        t_sep = QFrame()
        t_sep.setFrameShape(QFrame.Shape.VLine)
        t_sep.setObjectName("v_sep")
        toolbar.addWidget(t_sep)

        self._export_btn = QPushButton()
        self._export_btn.setFixedSize(28, 28)
        self._export_btn.setIconSize(QSize(16, 16))
        self._export_btn.setProperty("role", "icon_btn")
        self._export_btn.setToolTip("Export theme to JSON file")
        self._export_btn.setEnabled(False)
        self._export_btn.clicked.connect(self._on_export)

        self._import_btn = QPushButton()
        self._import_btn.setFixedSize(28, 28)
        self._import_btn.setIconSize(QSize(16, 16))
        self._import_btn.setProperty("role", "icon_btn")
        self._import_btn.setToolTip("Import theme from JSON file")
        self._import_btn.clicked.connect(self._on_import)

        toolbar.addWidget(self._export_btn, 0, Qt.AlignmentFlag.AlignVCenter)
        toolbar.addWidget(self._import_btn, 0, Qt.AlignmentFlag.AlignVCenter)
        outer.addLayout(toolbar)

        top_sep = QFrame()
        top_sep.setObjectName("h_sep")
        top_sep.setFrameShape(QFrame.Shape.HLine)
        outer.addWidget(top_sep)

        # -- Body: animated swatch panel + preview panel ------------------
        body = QHBoxLayout()
        body.setContentsMargins(0, 0, 0, 0)
        body.setSpacing(0)

        self._swatch_panel_widget = self._build_swatch_panel()
        self._swatch_panel_widget.setMinimumWidth(0)
        self._swatch_panel_widget.setMaximumWidth(0)  # starts collapsed
        body.addWidget(self._swatch_panel_widget)

        self._preview_panel = self._build_preview_panel()
        body.addWidget(self._preview_panel, 1)

        outer.addLayout(body, 1)

        # -- Bottom bar: save/revert + OK/Cancel --------------------------
        bot_sep = QFrame()
        bot_sep.setObjectName("h_sep")
        bot_sep.setFrameShape(QFrame.Shape.HLine)
        outer.addWidget(bot_sep)

        bottom = QHBoxLayout()
        bottom.setSpacing(6)

        self._save_btn = QPushButton("Save Changes")
        self._save_btn.setObjectName("save_button")
        self._save_btn.setEnabled(False)
        self._save_btn.setToolTip("Persist edits to this custom theme")
        self._save_btn.clicked.connect(self._on_save)

        self._revert_btn = QPushButton("Revert")
        self._revert_btn.setEnabled(False)
        self._revert_btn.setToolTip("Discard unsaved edits")
        self._revert_btn.clicked.connect(self._on_revert)

        bottom.addWidget(self._save_btn)
        bottom.addWidget(self._revert_btn)
        bottom.addStretch()

        bbox = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        ok_btn = bbox.button(QDialogButtonBox.StandardButton.Ok)
        if ok_btn:
            ok_btn.setObjectName("save_button")
        bbox.accepted.connect(self._on_accept)
        bbox.rejected.connect(self._on_cancel)
        bottom.addWidget(bbox)

        outer.addLayout(bottom)

        # -- Animation: slide the swatch panel in/out ---------------------
        self._swatch_anim = QPropertyAnimation(
            self._swatch_panel_widget, b"maximumWidth"
        )
        self._swatch_anim.setDuration(220)
        self._swatch_anim.finished.connect(self._on_swatch_anim_finished)

    def _build_swatch_panel(self) -> QWidget:
        """Slide-in color-swatch panel. Width is animated between 0 and _SWATCH_WIDTH."""
        # Outer: [content (stretch) | v_sep] so the separator travels with the panel.
        container = QWidget()
        outer_h = QHBoxLayout(container)
        outer_h.setContentsMargins(0, 0, 0, 0)
        outer_h.setSpacing(0)

        content = QWidget()
        content_v = QVBoxLayout(content)
        content_v.setContentsMargins(0, 0, 0, 0)
        content_v.setSpacing(0)

        # Header: collapse button + COLORS label
        header = QWidget()
        header_h = QHBoxLayout(header)
        header_h.setContentsMargins(8, 6, 8, 6)
        header_h.setSpacing(6)

        self._collapse_panel_btn = QPushButton()
        self._collapse_panel_btn.setFixedSize(28, 28)
        self._collapse_panel_btn.setIconSize(QSize(16, 16))
        self._collapse_panel_btn.setProperty("role", "icon_btn")
        self._collapse_panel_btn.setToolTip("Collapse color editor")
        self._collapse_panel_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._collapse_panel_btn.clicked.connect(
            lambda: self._toggle_swatch_panel(False)
        )
        header_h.addWidget(self._collapse_panel_btn)

        colors_lbl = QLabel("COLORS")
        colors_lbl.setProperty("role", "section")
        header_h.addWidget(colors_lbl, 1)
        content_v.addWidget(header)

        hdr_sep = QFrame()
        hdr_sep.setObjectName("h_sep")
        hdr_sep.setFrameShape(QFrame.Shape.HLine)
        content_v.addWidget(hdr_sep)

        # Swatch scroll area
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        swatch_container = QWidget()
        swatch_vbox = QVBoxLayout(swatch_container)
        swatch_vbox.setContentsMargins(4, 6, 4, 6)
        swatch_vbox.setSpacing(10)

        self._swatches: dict[str, _ColorSwatch] = {}
        for key, label in _COLOR_FIELDS:
            sw = _ColorSwatch()
            sw.colorChanged.connect(lambda _hex, k=key: self._on_color_changed(k, _hex))
            row = QHBoxLayout()
            row.setSpacing(8)
            row_label = QLabel(label)
            row_label.setFixedWidth(100)
            row_label.setAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter)
            row.addWidget(row_label)
            row.addWidget(sw, 1)
            swatch_vbox.addLayout(row)
            self._swatches[key] = sw

        swatch_vbox.addStretch()
        scroll.setWidget(swatch_container)
        content_v.addWidget(scroll, 1)

        outer_h.addWidget(content, 1)

        # Separator on the right edge (travels with the panel during animation)
        v_sep = QFrame()
        v_sep.setObjectName("v_sep")
        v_sep.setFrameShape(QFrame.Shape.VLine)
        outer_h.addWidget(v_sep)

        return container

    def _build_preview_panel(self) -> QFrame:
        panel = QFrame()
        panel.setObjectName("preview_panel")
        panel.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        panel.setMinimumWidth(320)

        outer = QVBoxLayout(panel)
        outer.setContentsMargins(4, 0, 4, 4)
        outer.setSpacing(6)

        # Builtin badge
        self._builtin_label = QLabel("Built-in (read only)")
        self._builtin_label.setProperty("role", "placeholder")
        self._builtin_label.setVisible(False)
        outer.addWidget(self._builtin_label)

        # PREVIEW section label + inspect hint + inspect toggle on the same row
        hdr_row = QHBoxLayout()
        hdr_row.setContentsMargins(0, 0, 0, 0)
        hdr_row.setSpacing(6)
        preview_lbl = QLabel("PREVIEW")
        preview_lbl.setProperty("role", "section")
        hdr_row.addWidget(preview_lbl)
        hdr_row.addStretch()
        self._inspect_hint = QLabel("")
        self._inspect_hint.setProperty("role", "muted")
        hdr_row.addWidget(self._inspect_hint)
        self._inspect_btn = QPushButton()
        self._inspect_btn.setCheckable(True)
        self._inspect_btn.setFixedSize(28, 28)
        self._inspect_btn.setIconSize(QSize(16, 16))
        self._inspect_btn.setObjectName("inspect_btn")
        self._inspect_btn.setToolTip("Inspect: right-click any preview element to pick its color")
        self._inspect_btn.toggled.connect(self._toggle_inspect)
        hdr_row.addWidget(self._inspect_btn)
        outer.addLayout(hdr_row)

        # Fake app window: simplified PlaybackTab simulation. Right-click any
        # element in inspect mode to pick the matching ThemeColors field.
        win = QFrame()
        win.setObjectName("preview_window")
        win.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        win_v = QVBoxLayout(win)
        win_v.setContentsMargins(0, 0, 0, 0)
        win_v.setSpacing(0)

        win_v.addWidget(self._build_prev_title_bar())
        win_v.addWidget(self._build_prev_file_strip())
        win_v.addWidget(self._build_prev_sub_tab_bar())
        win_v.addWidget(self._build_prev_body(), 1)
        win_v.addWidget(self._build_prev_transport_bar())

        outer.addWidget(win, 1)
        return panel

    def _build_prev_title_bar(self) -> QFrame:
        title_bar = QFrame()
        title_bar.setObjectName("collapsed_strip")
        title_bar.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        title_h = QHBoxLayout(title_bar)
        title_h.setContentsMargins(10, 6, 8, 6)
        title_h.setSpacing(4)
        app_name = QLabel("Hu<i>Midi</i>")
        app_name.setTextFormat(Qt.TextFormat.RichText)
        app_name.setObjectName("sidebar_logo_text")
        title_h.addWidget(app_name)
        title_h.addStretch()
        # En-dash, square, multiplication sign (NOT em-dash).
        for sym in ("–", "□", "×"):
            lbl = QLabel(sym)
            lbl.setProperty("role", "muted")
            title_h.addWidget(lbl)
        return title_bar

    def _build_prev_file_strip(self) -> QFrame:
        file_strip = QFrame()
        file_strip.setObjectName("file_strip")
        file_strip.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        fs_h = QHBoxLayout(file_strip)
        fs_h.setContentsMargins(10, 6, 10, 6)
        fs_h.setSpacing(8)

        tile = QFrame()
        tile.setObjectName("file_strip_tile")
        tile.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        tile.setFixedSize(28, 28)
        tile_layout = QVBoxLayout(tile)
        tile_layout.setContentsMargins(0, 0, 0, 0)
        self._prev_file_tile_icon = QLabel()
        self._prev_file_tile_icon.setObjectName("file_strip_tile_icon")
        self._prev_file_tile_icon.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._prev_file_tile_icon.setFixedSize(QSize(16, 16))
        self._prev_file_tile_icon.setScaledContents(True)
        tile_layout.addWidget(self._prev_file_tile_icon,
                              alignment=Qt.AlignmentFlag.AlignCenter)
        fs_h.addWidget(tile)

        info_col = QWidget()
        info_v = QVBoxLayout(info_col)
        info_v.setContentsMargins(0, 0, 0, 0)
        info_v.setSpacing(1)
        fs_name = QLabel("Demo.mid")
        fs_name.setObjectName("file_strip_name")
        fs_meta = QLabel("4 TRACKS / 2:34")
        fs_meta.setObjectName("file_strip_meta")
        info_v.addWidget(fs_name)
        info_v.addWidget(fs_meta)
        fs_h.addWidget(info_col, 1)
        return file_strip

    def _build_prev_sub_tab_bar(self) -> QFrame:
        sub_tab_bar = QFrame()
        sub_tab_bar.setObjectName("sub_tab_bar")
        sub_tab_bar.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        stb_h = QHBoxLayout(sub_tab_bar)
        stb_h.setContentsMargins(0, 0, 0, 0)
        stb_h.setSpacing(0)
        for i, lbl_txt in enumerate(["File", "Playback", "Humanize"]):
            btn = QPushButton(lbl_txt)
            btn.setObjectName("sub_tab_btn")
            btn.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
            btn.setProperty("active", "true" if i == 1 else "false")
            btn.setCursor(Qt.CursorShape.PointingHandCursor)
            stb_h.addWidget(btn)
        stb_h.addStretch(1)
        return sub_tab_bar

    def _build_prev_body(self) -> QWidget:
        body = QWidget()
        body_v = QVBoxLayout(body)
        body_v.setContentsMargins(10, 10, 10, 6)
        body_v.setSpacing(8)

        body_v.addWidget(self._build_prev_performance_card())
        body_v.addWidget(self._build_prev_options_card())
        body_v.addLayout(self._build_prev_track_row())
        body_v.addStretch()
        return body

    def _build_prev_performance_card(self) -> QFrame:
        card = QFrame()
        card.setObjectName("section_card")
        card.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        v = QVBoxLayout(card)
        v.setContentsMargins(12, 6, 12, 8)
        v.setSpacing(4)

        # Title row + reset button
        title_row = QHBoxLayout()
        title_row.setSpacing(6)
        title = QLabel("PERFORMANCE")
        title.setProperty("role", "section")
        title_row.addWidget(title)
        title_row.addStretch()
        self._prev_reset_btn = QPushButton()
        self._prev_reset_btn.setProperty("role", "card_reset")
        self._prev_reset_btn.setFixedSize(28, 28)
        self._prev_reset_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        title_row.addWidget(self._prev_reset_btn)
        v.addLayout(title_row)

        # Tempo: label + slider + spinbox
        tempo_row = QHBoxLayout()
        tempo_row.setSpacing(8)
        tempo_labels = QVBoxLayout()
        tempo_labels.setSpacing(1)
        tempo_labels.setContentsMargins(0, 0, 0, 0)
        tempo_lbl = QLabel("Tempo")
        tempo_desc = QLabel("% of original")
        tempo_desc.setProperty("role", "muted")
        tempo_labels.addWidget(tempo_lbl)
        tempo_labels.addWidget(tempo_desc)
        tempo_row.addLayout(tempo_labels)
        self._prev_slider = QSlider(Qt.Orientation.Horizontal)
        self._prev_slider.setRange(0, 200)
        self._prev_slider.setValue(100)
        tempo_row.addWidget(self._prev_slider, 1)
        self._prev_spin = QDoubleSpinBox()
        self._prev_spin.setRange(0, 200)
        self._prev_spin.setValue(100.0)
        self._prev_spin.setSuffix(" %")
        self._prev_spin.setFixedWidth(76)
        tempo_row.addWidget(self._prev_spin)
        v.addLayout(tempo_row)

        # Pedal: label + combobox
        pedal_row = QHBoxLayout()
        pedal_row.setSpacing(8)
        pedal_labels = QVBoxLayout()
        pedal_labels.setSpacing(1)
        pedal_labels.setContentsMargins(0, 0, 0, 0)
        pedal_lbl = QLabel("Pedal")
        pedal_desc = QLabel("generation algorithm")
        pedal_desc.setProperty("role", "muted")
        pedal_labels.addWidget(pedal_lbl)
        pedal_labels.addWidget(pedal_desc)
        pedal_row.addLayout(pedal_labels)
        self._prev_combo = QComboBox()
        self._prev_combo.addItems(["Auto (Default)", "PedalAI", "Harmonic"])
        pedal_row.addWidget(self._prev_combo, 1)
        v.addLayout(pedal_row)
        return card

    def _build_prev_options_card(self) -> QFrame:
        card = QFrame()
        card.setObjectName("section_card")
        card.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        v = QVBoxLayout(card)
        v.setContentsMargins(12, 6, 12, 8)
        v.setSpacing(4)

        title = QLabel("OPTIONS")
        title.setProperty("role", "section")
        v.addWidget(title)

        self._prev_check = QCheckBox("88-Key Layout")
        self._prev_check.setChecked(True)
        v.addWidget(self._prev_check)
        check_desc = QLabel("Map to the full 88-key piano")
        check_desc.setProperty("role", "muted")
        check_desc.setContentsMargins(25, 0, 0, 0)
        v.addWidget(check_desc)
        return card

    def _build_prev_track_row(self) -> QHBoxLayout:
        row = QHBoxLayout()
        row.setSpacing(8)

        # Part card: simulates a LOADED track entry.
        part_card = QFrame()
        part_card.setObjectName("part_card")
        part_card.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        pc_v = QVBoxLayout(part_card)
        pc_v.setContentsMargins(8, 5, 8, 5)
        pc_v.setSpacing(1)
        pc_title = QLabel("Track 1")
        pc_title.setObjectName("part_card_title")
        pc_meta = QLabel("C4-C6  Right")
        pc_meta.setObjectName("part_card_meta")
        pc_v.addWidget(pc_title)
        pc_v.addWidget(pc_meta)
        row.addWidget(part_card, 1)

        # Save card: simulates a SAVED SONGS entry.
        save_card = QFrame()
        save_card.setObjectName("save_card")
        save_card.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        sc_v = QVBoxLayout(save_card)
        sc_v.setContentsMargins(8, 5, 8, 5)
        sc_v.setSpacing(1)
        sc_title = QLabel("Demo.json")
        sc_title.setObjectName("part_card_title")
        sc_meta = QLabel("2 days ago")
        sc_meta.setObjectName("part_card_meta")
        sc_v.addWidget(sc_title)
        sc_v.addWidget(sc_meta)
        row.addWidget(save_card, 1)
        return row

    def _build_prev_transport_bar(self) -> QFrame:
        bar = QFrame()
        bar.setObjectName("transport_bar")
        bar.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        v = QVBoxLayout(bar)
        v.setContentsMargins(12, 8, 12, 8)
        v.setSpacing(6)

        # Scrubber row
        scrub_row = QHBoxLayout()
        scrub_row.setSpacing(8)
        time_start = QLabel("00:00")
        time_start.setObjectName("time_start_label")
        time_end = QLabel("03:42")
        time_end.setObjectName("time_end_label")
        self._prev_scrubber = QSlider(Qt.Orientation.Horizontal)
        self._prev_scrubber.setRange(0, 100)
        self._prev_scrubber.setValue(35)
        scrub_row.addWidget(time_start)
        scrub_row.addWidget(self._prev_scrubber, 1)
        scrub_row.addWidget(time_end)
        v.addLayout(scrub_row)

        # Action row: play, stop, save, pedal indicator.
        action_row = QHBoxLayout()
        action_row.setSpacing(5)
        self._prev_play_btn = QPushButton()
        self._prev_play_btn.setObjectName("play_button")
        self._prev_play_btn.setIconSize(QSize(22, 22))
        self._prev_play_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._prev_stop_btn = QPushButton()
        self._prev_stop_btn.setObjectName("stop_button")
        self._prev_stop_btn.setIconSize(QSize(22, 22))
        self._prev_stop_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        action_row.addWidget(self._prev_play_btn)
        action_row.addWidget(self._prev_stop_btn)
        action_row.addStretch()
        self._prev_save_btn = QPushButton()
        self._prev_save_btn.setObjectName("save_button")
        self._prev_save_btn.setIconSize(QSize(22, 22))
        self._prev_save_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        action_row.addWidget(self._prev_save_btn)
        action_row.addSpacing(8)
        pedal_swatch = QFrame()
        pedal_swatch.setObjectName("pedal_swatch")
        pedal_swatch.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        pedal_swatch.setFixedSize(18, 18)
        pedal_lbl = QLabel("PEDAL")
        pedal_lbl.setProperty("role", "muted")
        action_row.addWidget(pedal_swatch)
        action_row.addWidget(pedal_lbl)
        v.addLayout(action_row)
        return bar

    # ── Panel animation ───────────────────────────────────────────────

    def _toggle_swatch_panel(self, expand: bool) -> None:
        """Slide the color-swatch panel in (expand=True) or out."""
        self._swatch_anim.stop()
        current = self._swatch_panel_widget.maximumWidth()
        # Guard: if Qt's default QWIDGETSIZE_MAX was never overridden, treat as full.
        if current > _SWATCH_WIDTH:
            current = _SWATCH_WIDTH if expand else 0
        end = _SWATCH_WIDTH if expand else 0
        if current == end:
            # Already at the target state; still fire the post-animation logic.
            if not expand:
                self._expand_panel_btn.setVisible(True)
            return
        self._swatch_anim.setStartValue(current)
        self._swatch_anim.setEndValue(end)
        self._swatch_anim.setEasingCurve(
            QEasingCurve.Type.OutCubic if expand else QEasingCurve.Type.InCubic
        )
        if expand:
            self._expand_panel_btn.setVisible(False)
        self._swatch_anim.start()

    def _on_swatch_anim_finished(self) -> None:
        """Show the expand button once the swatch panel has fully collapsed."""
        if self._swatch_panel_widget.maximumWidth() == 0:
            self._expand_panel_btn.setVisible(True)

    # ── Population ────────────────────────────────────────────────────

    def _populate_list(self, select_name: str | None = None) -> None:
        self._combo.blockSignals(True)
        self._combo.clear()
        themes = ThemeManager.all_themes()
        active = ThemeManager.get_active_name()
        target_row = 0
        muted = QColor(ThemeManager.get_active().text_secondary)
        for i, (name, t) in enumerate(themes.items()):
            self._combo.addItem(name)
            if t.builtin:
                self._combo.model().item(i).setForeground(muted)
            if name == (select_name or active):
                target_row = i
        self._combo.blockSignals(False)
        self._combo.setCurrentIndex(target_row)

    # ── Slots ─────────────────────────────────────────────────────────

    def _on_row_changed(self, row: int) -> None:
        if row < 0:
            return
        name = self._combo.itemText(row)
        themes = ThemeManager.all_themes()
        theme = themes.get(name)
        if theme is None:
            return
        self._current_theme = theme
        self._pending_save = False

        # Populate editor
        self._builtin_label.setVisible(theme.builtin)
        for key, _lbl in _COLOR_FIELDS:
            self._swatches[key].set_color(getattr(theme, key))
            self._swatches[key].set_editable(not theme.builtin)

        self._del_btn.setEnabled(not theme.builtin)
        self._rename_btn.setEnabled(not theme.builtin)
        self._export_btn.setEnabled(True)
        self._save_btn.setEnabled(False)
        self._revert_btn.setEnabled(False)

        # Live preview
        self._preview(theme)

    def _on_color_changed(self, field: str, value: str) -> None:
        if self._current_theme is None:
            return
        self._current_theme = replace(self._current_theme, **{field: value})
        self._mark_dirty()
        self._preview(self._current_theme)

    def _mark_dirty(self, *_) -> None:
        self._pending_save = True
        self._save_btn.setEnabled(True)
        self._revert_btn.setEnabled(True)

    def _on_new(self) -> None:
        """Duplicate the selected theme as a new custom theme."""
        base = self._current_theme or list(BUILTIN_THEMES.values())[0]
        # Find a unique name
        existing = set(ThemeManager.all_themes().keys())
        candidate = f"{base.name} Copy"
        n = 2
        while candidate in existing:
            candidate = f"{base.name} Copy {n}"
            n += 1
        new_theme = replace(base, name=candidate, builtin=False)
        ThemeManager.save_custom(new_theme)
        self._populate_list(select_name=candidate)

    def _on_delete(self) -> None:
        if self._current_theme is None or self._current_theme.builtin:
            return
        name = self._current_theme.name
        reply = QMessageBox.question(
            self, "Delete Theme",
            f'Delete custom theme "{name}"?',
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
        )
        if reply == QMessageBox.StandardButton.Yes:
            ThemeManager.delete_custom(name)
            # Revert preview to first available theme
            active = ThemeManager.get_active_name()
            if active == name:
                ThemeManager.set_active_name("Dark")
                self._preview(BUILTIN_THEMES["Dark"])
            self._populate_list()

    def _on_rename(self) -> None:
        if self._current_theme is None or self._current_theme.builtin:
            return
        current_name = self._current_theme.name
        new_name, ok = QInputDialog.getText(
            self, "Rename Theme", "New theme name:", text=current_name
        )
        if not ok:
            return
        new_name = new_name.strip()
        if not new_name or new_name == current_name:
            return
        existing = set(ThemeManager.all_themes().keys())
        existing.discard(current_name)
        if new_name in existing:
            QMessageBox.warning(
                self, "Name in Use",
                f'A theme named "{new_name}" already exists.'
            )
            return
        updated = replace(self._current_theme, name=new_name, builtin=False)
        ThemeManager.delete_custom(current_name)
        if ThemeManager.get_active_name() == current_name:
            ThemeManager.set_active_name(new_name)
        ThemeManager.save_custom(updated)
        self._current_theme = updated
        self._pending_save = False
        self._populate_list(select_name=new_name)

    def _on_save(self) -> None:
        if self._current_theme is None or self._current_theme.builtin:
            return
        new_name = self._current_theme.name
        old_name = new_name
        updated = replace(self._current_theme, name=new_name, builtin=False)
        # If renamed, delete old entry first
        if old_name != new_name:
            ThemeManager.delete_custom(old_name)
            if ThemeManager.get_active_name() == old_name:
                ThemeManager.set_active_name(new_name)
        ThemeManager.save_custom(updated)
        self._current_theme = updated
        self._pending_save = False
        self._save_btn.setEnabled(False)
        self._revert_btn.setEnabled(False)
        self._populate_list(select_name=new_name)

    def _on_revert(self) -> None:
        if self._current_theme is None:
            return
        # Re-load from disk
        themes = ThemeManager.all_themes()
        original = themes.get(self._current_theme.name)
        if original:
            self._current_theme = original
            for key, _lbl in _COLOR_FIELDS:
                self._swatches[key].set_color(getattr(original, key))
            self._preview(original)
        self._pending_save = False
        self._save_btn.setEnabled(False)
        self._revert_btn.setEnabled(False)

    def _on_export(self) -> None:
        if self._current_theme is None:
            return
        safe_name = self._current_theme.name.replace("/", "-").replace("\\", "-")
        default_path = str(Path.home() / f"{safe_name}.json")
        path, _ = QFileDialog.getSaveFileName(
            self, "Export Theme", default_path, "JSON files (*.json)"
        )
        if not path:
            return
        try:
            with open(path, "w", encoding="utf-8") as f:
                json.dump(self._current_theme.to_dict(), f, indent=2)
        except OSError as e:
            QMessageBox.warning(self, "Export Failed", str(e))

    def _on_import(self) -> None:
        path, _ = QFileDialog.getOpenFileName(
            self, "Import Theme", str(Path.home()), "JSON files (*.json)"
        )
        if not path:
            return
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
            theme = ThemeColors.from_dict(data)
        except Exception as e:
            QMessageBox.warning(self, "Import Failed", f"Could not read theme file:\n{e}")
            return
        # Ensure the name is unique among existing themes
        existing = set(ThemeManager.all_themes().keys())
        name = theme.name or Path(path).stem
        candidate = name
        n = 2
        while candidate in existing:
            candidate = f"{name} {n}"
            n += 1
        theme = replace(theme, name=candidate, builtin=False)
        ThemeManager.save_custom(theme)
        self._populate_list(select_name=candidate)

    def _on_accept(self) -> None:
        if self._pending_save:
            reply = QMessageBox.question(
                self, "Unsaved Changes",
                "You have unsaved edits. Save them before applying?",
                QMessageBox.StandardButton.Save |
                QMessageBox.StandardButton.Discard |
                QMessageBox.StandardButton.Cancel,
            )
            if reply == QMessageBox.StandardButton.Cancel:
                return
            if reply == QMessageBox.StandardButton.Save:
                self._on_save()

        if self._current_theme:
            ThemeManager.set_active_name(self._current_theme.name)
            self.theme_applied.emit(self._current_theme.name)
        self.accept()

    def _on_cancel(self) -> None:
        self.reject()

    # ── Helpers ───────────────────────────────────────────────────────

    def _refresh_io_icons(self, color: str) -> None:
        self._new_btn.setIcon(_svg_icon(_SVG_NEW, color))
        self._del_btn.setIcon(_svg_icon(_SVG_DELETE, color))
        self._rename_btn.setIcon(_svg_icon(_SVG_RENAME, color))
        self._export_btn.setIcon(_svg_icon(_SVG_EXPORT, color))
        self._import_btn.setIcon(_svg_icon(_SVG_IMPORT, color))
        self._inspect_btn.setIcon(_svg_icon(_SVG_INSPECT, color))
        self._expand_panel_btn.setIcon(_svg_icon(_SVG_EXPAND_PANEL, color))
        self._collapse_panel_btn.setIcon(_svg_icon(_SVG_COLLAPSE_PANEL, color))

    def _preview(self, theme: ThemeColors) -> None:
        """Apply the previewed theme to the preview panel only. Dialog chrome and main window are not touched."""
        play_bg  = _mix(theme.bg_primary, theme.accent_play, 0.15)
        play_hov = _mix(theme.bg_primary, theme.accent_play, 0.28)
        play_bdr = _mix(theme.accent_play, theme.bg_secondary, 0.40)
        stop_bg  = _mix(theme.bg_primary, theme.accent_stop, 0.15)
        stop_hov = _mix(theme.bg_primary, theme.accent_stop, 0.28)
        stop_bdr = _mix(theme.accent_stop, theme.bg_secondary, 0.40)
        save_bg  = _mix(theme.bg_primary, theme.accent, 0.15)
        save_hov = _mix(theme.bg_primary, theme.accent, 0.28)
        save_bdr = _mix(theme.accent, theme.bg_secondary, 0.40)
        ss = generate_stylesheet(theme)
        self._preview_panel.setStyleSheet(
            ss
            + f"\nQFrame#preview_window {{ border: 1px solid {theme.border}; border-radius: 4px; background-color: {theme.bg_primary}; }}"
            + f"\nQPushButton#play_button {{ background-color: {play_bg}; border-color: {play_bdr}; }}"
            + f"\nQPushButton#play_button:hover {{ background-color: {play_hov}; }}"
            + f"\nQPushButton#stop_button {{ background-color: {stop_bg}; border-color: {stop_bdr}; }}"
            + f"\nQPushButton#stop_button:hover {{ background-color: {stop_hov}; }}"
            + f"\nQPushButton#save_button {{ background-color: {save_bg}; border-color: {save_bdr}; }}"
            + f"\nQPushButton#save_button:hover {{ background-color: {save_hov}; }}"
            + f"\nQFrame#pedal_swatch {{ background-color: {theme.pedal_color}; border-radius: 4px; }}"
        )
        _ti = 22
        self._prev_play_btn.setIcon(ph_icon("play",        theme.accent_play, _ti))
        self._prev_stop_btn.setIcon(ph_icon("stop",        theme.accent_stop, _ti))
        self._prev_save_btn.setIcon(ph_icon("floppy-disk", theme.accent,      _ti))
        # Reset button (card_reset role) renders its glyph in the muted-text color.
        self._prev_reset_btn.setIcon(
            ph_icon("arrow-counter-clockwise", theme.text_secondary, 14)
        )
        self._prev_reset_btn.setIconSize(QSize(14, 14))
        # File strip tile uses an accent-colored music-note icon.
        self._prev_file_tile_icon.setPixmap(
            ph_icon("music-note", theme.accent, 16).pixmap(32, 32)
        )

    def _sync_toolbar_btn_sizes(self) -> None:
        h = self._combo.sizeHint().height()
        for btn in (
            self._expand_panel_btn, self._new_btn, self._del_btn,
            self._rename_btn, self._export_btn, self._import_btn,
        ):
            btn.setFixedSize(h, h)

    def _apply_own_stylesheet(self) -> None:
        active = ThemeManager.get_active()
        self.setStyleSheet(generate_stylesheet(active))
        self._refresh_io_icons(active.text_secondary)

    def _cleanup_inspect(self) -> None:
        QApplication.instance().removeEventFilter(self._inspect_filter)
        if self._active_color_dlg is not None:
            self._active_color_dlg.close()
            self._active_color_dlg = None

    def _toggle_inspect(self, active: bool) -> None:
        self._inspect_filter.set_active(active)
        if active:
            self._inspect_hint.setText("Right-click any element to pick its color")
            shape = Qt.CursorShape.CrossCursor
        else:
            self._inspect_hint.setText("")
            shape = Qt.CursorShape.ArrowCursor
            if self._active_color_dlg is not None:
                self._active_color_dlg.close()
                self._active_color_dlg = None
        cursor = QCursor(shape)
        self._preview_panel.setCursor(cursor)
        for child in self._preview_panel.findChildren(QWidget):
            child.setCursor(cursor)

    def _on_widget_picked(self, widget: QWidget, global_pos) -> None:
        if self._current_theme is None:
            return
        if self._current_theme.builtin:
            base = self._current_theme
            existing = set(ThemeManager.all_themes().keys())
            candidate = f"{base.name} Copy"
            n = 2
            while candidate in existing:
                candidate = f"{base.name} Copy {n}"
                n += 1
            ThemeManager.save_custom(replace(base, name=candidate, builtin=False))
            self._populate_list(select_name=candidate)
            # _on_row_changed fires synchronously; _current_theme is now the duplicate
            self._inspect_hint.setText(f'Duplicated as "{candidate}" — editing below')

        result = _field_for_widget(widget)
        if result is None:
            self._inspect_hint.setText("No color field for this element")
            return

        field, label = result
        original_hex = getattr(self._current_theme, field)

        if self._active_color_dlg is not None:
            self._active_color_dlg.close()
            self._active_color_dlg = None

        self._inspect_hint.setText(f"Editing: {label}")

        dlg = QColorDialog(QColor(original_hex), self)
        dlg.setWindowTitle(f"Pick color: {label}")
        dlg.setOption(QColorDialog.ColorDialogOption.DontUseNativeDialog, True)
        self._active_color_dlg = dlg

        theme_snapshot = self._current_theme

        def _live(color: QColor) -> None:
            if self._current_theme is not None:
                self._preview(replace(self._current_theme, **{field: color.name()}))

        def _accept() -> None:
            final_hex = dlg.currentColor().name()
            self._on_color_changed(field, final_hex)
            self._swatches[field].set_color(final_hex)
            self._inspect_hint.setText(f"Applied: {label}")
            self._active_color_dlg = None

        def _reject() -> None:
            self._current_theme = theme_snapshot
            self._preview(theme_snapshot)
            self._inspect_hint.setText("Cancelled")
            self._active_color_dlg = None

        dlg.currentColorChanged.connect(_live)
        dlg.accepted.connect(_accept)
        dlg.rejected.connect(_reject)

        screen = QApplication.screenAt(global_pos) or QApplication.primaryScreen()
        if screen:
            sg = screen.availableGeometry()
            x = min(global_pos.x() + 12, sg.right() - 450)
            y = min(global_pos.y() + 12, sg.bottom() - 380)
            dlg.move(max(sg.left(), x), max(sg.top(), y))

        dlg.show()
