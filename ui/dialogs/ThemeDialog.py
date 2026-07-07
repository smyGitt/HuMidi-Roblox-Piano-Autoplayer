"""
ThemeDialog -- select from built-in presets, create / edit / delete custom themes.

Live preview applies only to the preview panel inside this dialog. The main
window is updated only when the user clicks Save/OK via the theme_applied signal.
Cancelling requires no revert because the main window is never touched.
"""

from __future__ import annotations
from dataclasses import replace
import json
from pathlib import Path

from PySide6.QtWidgets import (
    QDialog, QHBoxLayout, QVBoxLayout, QGridLayout,
    QLabel, QLineEdit, QPushButton, QScrollArea, QWidget,
    QFrame, QMessageBox, QFileDialog, QSizePolicy,
    QSlider, QSpinBox, QDoubleSpinBox, QComboBox,
    QApplication, QColorDialog, QInputDialog,
)
from PySide6.QtCore import (
    Qt, QSize, Signal, QEvent, QObject,
    QPoint, QPropertyAnimation, QEasingCurve, QRect, QTimer,
)
from PySide6.QtGui import QColor, QPainter, QPainterPath, QPen, QPixmap, QCursor

from ui.theme import ThemeColors, ThemeManager, generate_stylesheet, BUILTIN_THEMES, _mix
from ui.widgets.ph_icon import ph_icon
from ui.widgets.ph_icon_label import PhIconLabel, IconProvider
from ui.widgets.toggle_switch import ToggleSwitch
from ui.widgets.slider_spinbox import NoScrollComboBox, NoScrollDoubleSpinBox


# ── Preview scale ─────────────────────────────────────────────────────────────

# All fixed pixel dimensions inside the preview window are half the real-UI value.
_PREV_SCALE: float = 1 / 2

# Fully-expanded width of the swatch side panel in pixels.
_SWATCH_WIDTH = 280


def _ps(n: int) -> int:
    """Scale a real-UI pixel dimension to the preview window size."""
    return int(n * _PREV_SCALE)


# ── Colour groups (order matters -- shown in the editor) ──────────────────────
# Each entry is a (key, label) color field or a str group header.

_COLOR_GROUPS: list[tuple[str, str] | str] = [
    "SURFACES",
    ("bg_primary",      "Background"),
    ("bg_secondary",    "Surface"),
    ("bg_input",        "Input Fields"),
    ("bg_button",       "Button"),
    "TEXT & BORDERS",
    ("text_primary",    "Text"),
    ("text_secondary",  "Muted Text"),
    ("border",          "Borders"),
    "ACCENTS",
    ("accent",          "Accent"),
    ("accent_controls", "Controls"),
    ("accent_save",     "Save Color"),
    "TOGGLE SWITCH",
    ("toggle_off",      "Off"),
    ("toggle_on",       "On"),
    ("knob_color",      "Knob"),
    "STATUS",
    ("accent_play",     "Play Color"),
    ("accent_stop",     "Stop / Danger"),
    ("accent_loaded",   "File Loaded"),
    ("pedal_color",     "Pedal Color"),
]

# Flat list of (key, label) pairs -- used by code that iterates fields only.
_COLOR_FIELDS = [entry for entry in _COLOR_GROUPS if isinstance(entry, tuple)]

# ── Widget-to-ThemeColors-field lookup tables (module-level; never change) ────

_BY_NAME: dict[str, tuple[str, str]] = {
    "pedal_swatch":         ("pedal_color",    "Pedal Color"),
    "border_swatch":        ("border",         "Borders"),
    "play_button":          ("accent_play",    "Play Color"),
    "stop_button":          ("accent_stop",    "Stop / Danger"),
    "save_button":          ("accent_save",    "Save Color"),
    "preview_window":       ("bg_primary",     "Background"),
    "collapsed_strip":      ("bg_secondary",   "Surface"),
    "sidebar":              ("bg_secondary",   "Surface"),
    "sidebar_logo_text":    ("text_primary",   "Text"),
    "file_strip":           ("bg_secondary",   "Surface"),
    "file_strip_tile":      ("accent",         "Accent"),
    "file_strip_tile_icon": ("accent",         "Accent"),
    "file_strip_name":      ("text_primary",   "Text"),
    "file_strip_meta":      ("text_secondary", "Muted Text"),
    "sub_tab_bar":          ("bg_primary",     "Background"),
    "section_card":         ("bg_secondary",   "Surface"),
    "part_card":            ("bg_input",       "Input Fields"),
    "save_card":            ("bg_input",       "Input Fields"),
    "part_card_title":      ("text_primary",   "Text"),
    "part_card_meta":       ("text_secondary", "Muted Text"),
    "transport_bar":        ("bg_secondary",   "Surface"),
    "time_start_label":     ("text_primary",   "Text"),
    "time_end_label":       ("text_secondary", "Muted Text"),
}

_BY_CLASS: dict[str, tuple[str, str]] = {
    "QLineEdit":      ("bg_input",        "Input Fields"),
    "QComboBox":      ("bg_input",        "Input Fields"),
    "QDoubleSpinBox": ("bg_input",        "Input Fields"),
    "QSpinBox":       ("bg_input",        "Input Fields"),
    "QSlider":        ("accent_controls", "Controls"),
    "ToggleSwitch":   ("knob_color",      "Knob"),
    "QPushButton":    ("bg_button",       "Button"),
    "QFrame":         ("bg_primary",      "Background"),
    "QWidget":        ("bg_primary",      "Background"),
}


def _field_for_widget(widget: QWidget) -> tuple[str, str] | None:
    """Return (ThemeColors field, display label) for a preview widget, or None."""
    obj_name = widget.objectName()
    cls_name = type(widget).__name__

    if obj_name in _BY_NAME:
        return _BY_NAME[obj_name]

    # Nav text labels are always empty in the collapsed preview sidebar.
    if obj_name == "nav_label":
        return None

    if obj_name == "nav_btn":
        if widget.property("active") == "true":
            return ("accent", "Accent")
        return ("bg_secondary", "Surface")

    if obj_name == "nav_icon":
        p = widget.parent()
        if p and (p.property("active") == "true" or p.property("hovered") == "true"):
            return ("text_primary", "Text")
        return ("text_secondary", "Muted Text")

    if obj_name == "sub_tab_btn":
        if widget.property("active") == "true":
            return ("accent", "Accent")
        return ("text_secondary", "Muted Text")

    if widget.property("variant") == "card_reset":
        return ("text_secondary", "Muted Text")

    if cls_name == "QLabel":
        role = widget.property("variant")
        if role in ("muted", "placeholder", "section"):
            return ("text_secondary", "Muted Text")
        return ("text_primary", "Text")

    return _BY_CLASS.get(cls_name)


# ── Unique-name helper ────────────────────────────────────────────────────────

def _unique_copy_name(base_name: str, existing: set[str]) -> str:
    """Return '<base_name> Copy' (or '<base_name> Copy N') not in existing."""
    candidate = f"{base_name} Copy"
    n = 2
    while candidate in existing:
        candidate = f"{base_name} Copy {n}"
        n += 1
    return candidate


# ── Color swatch widget ───────────────────────────────────────────────────────

class _ColorSwatch(QWidget):
    """A coloured square button + hex text field, kept in sync."""

    colorChanged = Signal(str)

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
        color = QColorDialog.getColor(QColor(self._color), self, "Choose colour")
        if color.isValid():
            self.set_color(color.name(), emit=True)

    def _on_hex_edited(self, text: str) -> None:
        text = text.strip()
        if not text.startswith("#"):
            text = "#" + text
        if len(text) == 7 and QColor(text).isValid():
            self._color = text.lower()
            self._refresh_swatch()
            self.colorChanged.emit(self._color)

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


# ── Inspect mode event filter ─────────────────────────────────────────────────

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


# ── Hover highlight overlay ───────────────────────────────────────────────────

class _HoverOverlay(QWidget):
    """Transparent overlay drawn over the preview panel in inspect mode.

    Renders animated marching-ants dashed borders around all preview widgets
    that share the same ThemeColors field as the currently hovered widget.
    The overlay is mouse-transparent so all events pass through to children.
    """

    _DASH_PATTERN = [6.0, 4.0]
    _PATTERN_CYCLE = 10.0
    _BORDER_RADIUS = 5
    _PEN_WIDTH = 1.5

    def __init__(self, parent: QWidget, accent: str):
        super().__init__(parent)
        self.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        self.setAttribute(Qt.WidgetAttribute.WA_NoSystemBackground)
        self.setAutoFillBackground(False)
        self._rects: list[QRect] = []
        self._accent = QColor(accent)
        self._dash_offset = 0.0
        self._timer = QTimer(self)
        self._timer.timeout.connect(self._tick)
        self._timer.start(40)
        self.resize(parent.size())
        self.raise_()

    def set_accent(self, color: str) -> None:
        self._accent = QColor(color)

    def set_rects(self, rects: list[QRect]) -> None:
        self._rects = rects
        self.update()

    def _tick(self) -> None:
        p = self.parentWidget()
        if p and self.size() != p.size():
            self.resize(p.size())
            self.raise_()
        if self._rects:
            self._dash_offset = (self._dash_offset + 0.5) % self._PATTERN_CYCLE
            self.update()

    def paintEvent(self, event) -> None:
        if not self._rects:
            return
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        path = QPainterPath()
        for r in self._rects:
            ar = r.adjusted(1, 1, -1, -1)
            sub = QPainterPath()
            sub.addRoundedRect(
                float(ar.x()), float(ar.y()), float(ar.width()), float(ar.height()),
                self._BORDER_RADIUS, self._BORDER_RADIUS,
            )
            path = path.united(sub)
        fill = QColor(self._accent)
        fill.setAlphaF(0.12)
        pen = QPen(self._accent)
        pen.setWidthF(self._PEN_WIDTH)
        pen.setStyle(Qt.PenStyle.CustomDashLine)
        pen.setDashPattern(self._DASH_PATTERN)
        pen.setDashOffset(self._dash_offset)
        painter.setBrush(fill)
        painter.setPen(Qt.PenStyle.NoPen)
        painter.drawPath(path)
        painter.setPen(pen)
        painter.setBrush(Qt.BrushStyle.NoBrush)
        painter.drawPath(path)


# ── Theme dialog ──────────────────────────────────────────────────────────────

class ThemeDialog(QDialog):
    """
    Lets the user pick a built-in theme or create / edit / delete custom ones.

    Live preview is scoped to the preview panel inside this dialog; the main
    window is unchanged until the user accepts. Cancelling requires no revert.
    """

    theme_applied = Signal(str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self._previous_name = ThemeManager.get_active_name()
        self._current_theme: ThemeColors | None = None
        self._pending_save = False
        self._active_color_dlg: QColorDialog | None = None
        self._hover_overlay: _HoverOverlay | None = None
        self._hover_poll_timer: QTimer | None = None
        self._last_polled_widget: QWidget | None = None

        self.setWindowTitle("Theme Manager")
        self.setMinimumSize(550, 580)
        self._build_ui()

        self._inspect_filter = _InspectFilter(self._preview_panel, self)
        self._inspect_filter.widget_right_clicked.connect(self._on_widget_picked)
        QApplication.instance().installEventFilter(self._inspect_filter)
        self.finished.connect(self._cleanup_inspect)

        provider = IconProvider.instance()
        for _icon in (
            self._expand_panel_btn, self._collapse_panel_btn,
            self._new_btn, self._del_btn, self._rename_btn,
            self._export_btn, self._import_btn,
        ):
            provider.register(_icon)

        self._apply_own_stylesheet()
        self._sync_toolbar_btn_widths()
        self._populate_list()

    # ── Layout ────────────────────────────────────────────────────────

    def _build_ui(self) -> None:
        outer = QVBoxLayout(self)
        outer.setContentsMargins(12, 12, 12, 12)
        outer.setSpacing(8)

        # Top toolbar: theme selector + action buttons
        toolbar = QHBoxLayout()
        toolbar.setSpacing(4)

        self._expand_panel_btn = PhIconLabel("palette", size=16, allow_vertical_expansion=True)
        self._expand_panel_btn.setProperty("variant", "icon_btn")
        self._expand_panel_btn.setToolTip("Open color editor")
        self._expand_panel_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._expand_panel_btn.clicked.connect(lambda: self._toggle_swatch_panel(True))
        toolbar.addWidget(self._expand_panel_btn)

        self._collapse_panel_btn = PhIconLabel("palette", size=16, allow_vertical_expansion=True)
        self._collapse_panel_btn.setProperty("variant", "icon_btn")
        self._collapse_panel_btn.setToolTip("Collapse color editor")
        self._collapse_panel_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._collapse_panel_btn.clicked.connect(lambda: self._toggle_swatch_panel(False))
        self._collapse_panel_btn.setVisible(False)
        toolbar.addWidget(self._collapse_panel_btn)

        self._combo = NoScrollComboBox()
        self._combo.setSizeAdjustPolicy(
            NoScrollComboBox.SizeAdjustPolicy.AdjustToMinimumContentsLengthWithIcon
        )
        self._combo.setMinimumContentsLength(0)
        self._combo.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Fixed)
        self._combo.currentIndexChanged.connect(self._on_row_changed)
        toolbar.addWidget(self._combo, 1)

        self._new_btn = PhIconLabel("new-theme", size=16, allow_vertical_expansion=True)
        self._new_btn.setProperty("variant", "icon_btn")
        self._new_btn.setToolTip("Duplicate selected theme as a new custom preset")
        self._new_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._new_btn.clicked.connect(self._on_new)

        self._del_btn = PhIconLabel("delete-theme", size=16, allow_vertical_expansion=True)
        self._del_btn.setProperty("variant", "icon_btn")
        self._del_btn.setToolTip("Delete this custom theme (built-in themes cannot be deleted)")
        self._del_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._del_btn.setEnabled(False)
        self._del_btn.clicked.connect(self._on_delete)

        self._rename_btn = PhIconLabel("rename-theme", size=16, allow_vertical_expansion=True)
        self._rename_btn.setProperty("variant", "icon_btn")
        self._rename_btn.setToolTip("Rename this custom theme")
        self._rename_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._rename_btn.setEnabled(False)
        self._rename_btn.clicked.connect(self._on_rename)

        toolbar.addWidget(self._new_btn)
        toolbar.addWidget(self._del_btn)
        toolbar.addWidget(self._rename_btn)

        t_sep = QFrame()
        t_sep.setFrameShape(QFrame.Shape.VLine)
        t_sep.setObjectName("v_sep")
        toolbar.addWidget(t_sep)

        self._export_btn = PhIconLabel("export-theme", size=16, allow_vertical_expansion=True)
        self._export_btn.setProperty("variant", "icon_btn")
        self._export_btn.setToolTip("Export theme to JSON file")
        self._export_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._export_btn.setEnabled(False)
        self._export_btn.clicked.connect(self._on_export)

        self._import_btn = PhIconLabel("import-theme", size=16, allow_vertical_expansion=True)
        self._import_btn.setProperty("variant", "icon_btn")
        self._import_btn.setToolTip("Import theme from JSON file")
        self._import_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._import_btn.clicked.connect(self._on_import)

        toolbar.addWidget(self._export_btn)
        toolbar.addWidget(self._import_btn)
        outer.addLayout(toolbar)

        top_sep = QFrame()
        top_sep.setObjectName("h_sep")
        top_sep.setFrameShape(QFrame.Shape.HLine)
        outer.addWidget(top_sep)

        # Body: animated swatch panel + preview panel
        body = QHBoxLayout()
        body.setContentsMargins(0, 0, 0, 0)
        body.setSpacing(0)

        self._swatch_panel_widget = self._build_swatch_panel()
        self._swatch_panel_widget.setMinimumWidth(0)
        self._swatch_panel_widget.setMaximumWidth(0)
        body.addWidget(self._swatch_panel_widget)

        self._preview_panel = self._build_preview_panel()
        body.addWidget(self._preview_panel, 1)

        outer.addLayout(body, 1)

        # Bottom bar
        bot_sep = QFrame()
        bot_sep.setObjectName("h_sep")
        bot_sep.setFrameShape(QFrame.Shape.HLine)
        outer.addWidget(bot_sep)

        bottom = QHBoxLayout()
        bottom.setSpacing(6)

        self._revert_btn = QPushButton("Revert")
        self._revert_btn.setEnabled(False)
        self._revert_btn.setToolTip("Discard unsaved edits")
        self._revert_btn.clicked.connect(self._on_revert)
        bottom.addWidget(self._revert_btn)

        bottom.addStretch()

        cancel_btn = QPushButton("Cancel")
        cancel_btn.clicked.connect(self.reject)
        bottom.addWidget(cancel_btn)

        self._save_btn = QPushButton("Save")
        self._save_btn.setObjectName("save_button")
        self._save_btn.setToolTip("Save edits and apply theme")
        self._save_btn.clicked.connect(self._on_accept)
        bottom.addWidget(self._save_btn)

        outer.addLayout(bottom)

        # Animation: slide the swatch panel in/out
        self._swatch_anim = QPropertyAnimation(self._swatch_panel_widget, b"maximumWidth")
        self._swatch_anim.setDuration(220)
        self._swatch_anim.finished.connect(self._on_swatch_anim_finished)

    def _build_swatch_panel(self) -> QWidget:
        """Slide-in color-swatch panel. Width is animated between 0 and _SWATCH_WIDTH."""
        container = QWidget()
        outer_h = QHBoxLayout(container)
        outer_h.setContentsMargins(0, 0, 0, 0)
        outer_h.setSpacing(0)

        content = QWidget()
        content_v = QVBoxLayout(content)
        content_v.setContentsMargins(0, 0, 0, 0)
        content_v.setSpacing(0)

        header_row = QWidget()
        header_h = QHBoxLayout(header_row)
        header_h.setContentsMargins(8, 6, 8, 6)
        header_h.setSpacing(6)
        colors_lbl = QLabel("COLORS")
        colors_lbl.setProperty("variant", "section")
        header_h.addWidget(colors_lbl)
        header_h.addStretch()
        content_v.addWidget(header_row)

        hdr_sep = QFrame()
        hdr_sep.setObjectName("h_sep")
        hdr_sep.setFrameShape(QFrame.Shape.HLine)
        content_v.addWidget(hdr_sep)

        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        swatch_container = QWidget()
        swatch_grid = QGridLayout(swatch_container)
        swatch_grid.setContentsMargins(4, 6, 4, 6)
        swatch_grid.setHorizontalSpacing(8)
        swatch_grid.setVerticalSpacing(10)
        swatch_grid.setColumnStretch(0, 0)  # label column, fixed
        swatch_grid.setColumnStretch(1, 1)  # swatch widget spans col 1+2; hex field stretches

        self._swatches: dict[str, _ColorSwatch] = {}
        grid_row = 0
        for entry in _COLOR_GROUPS:
            if isinstance(entry, str):
                group_lbl = QLabel(entry)
                group_lbl.setProperty("variant", "section")
                group_lbl.setContentsMargins(0, 6 if grid_row == 0 else 10, 0, 2)
                swatch_grid.addWidget(group_lbl, grid_row, 0, 1, 2)
                grid_row += 1
            else:
                key, label = entry
                sw = _ColorSwatch()
                sw.colorChanged.connect(lambda _hex, k=key: self._on_color_changed(k, _hex))
                row_label = QLabel(label)
                row_label.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
                swatch_grid.addWidget(row_label, grid_row, 0)
                swatch_grid.addWidget(sw,        grid_row, 1)
                self._swatches[key] = sw
                grid_row += 1

        swatch_grid.setRowStretch(grid_row, 1)
        scroll.setWidget(swatch_container)
        content_v.addWidget(scroll, 1)

        outer_h.addWidget(content, 1)

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

        self._builtin_label = QLabel("Built-in (editing creates a copy)")
        self._builtin_label.setProperty("variant", "placeholder")
        self._builtin_label.setVisible(False)
        outer.addWidget(self._builtin_label)

        hdr_row = QHBoxLayout()
        hdr_row.setContentsMargins(0, 0, 0, 0)
        hdr_row.setSpacing(6)
        preview_lbl = QLabel("PREVIEW")
        preview_lbl.setProperty("variant", "section")
        hdr_row.addWidget(preview_lbl)
        hdr_row.addStretch()
        self._inspect_hint = QLabel("")
        self._inspect_hint.setProperty("variant", "muted")
        hdr_row.addWidget(self._inspect_hint)
        self._inspect_btn = QPushButton("  Right click edit mode")
        self._inspect_btn.setCheckable(True)
        self._inspect_btn.setIconSize(QSize(16, 16))
        self._inspect_btn.setObjectName("inspect_btn")
        self._inspect_btn.setToolTip("Inspect: right-click any preview element to pick its color")
        self._inspect_btn.toggled.connect(self._toggle_inspect)
        hdr_row.addWidget(self._inspect_btn)
        outer.addLayout(hdr_row)

        win = QFrame()
        win.setObjectName("preview_window")
        win.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        win_v = QVBoxLayout(win)
        win_v.setContentsMargins(0, 0, 0, 0)
        win_v.setSpacing(0)

        win_v.addWidget(self._build_prev_title_bar())

        middle = QWidget()
        middle.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        middle_h = QHBoxLayout(middle)
        middle_h.setContentsMargins(0, 0, 0, 0)
        middle_h.setSpacing(0)
        middle_h.addWidget(self._build_prev_sidebar())
        content_col = QWidget()
        content_col.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        content_v = QVBoxLayout(content_col)
        content_v.setContentsMargins(0, 0, 0, 0)
        content_v.setSpacing(0)
        content_v.addWidget(self._build_prev_file_strip())
        content_v.addWidget(self._build_prev_sub_tab_bar())
        content_v.addWidget(self._build_prev_body(), 1)
        middle_h.addWidget(content_col, 1)
        win_v.addWidget(middle, 1)

        win_v.addWidget(self._build_prev_transport_bar())

        self._preview_window = win
        outer.addWidget(win, 1)

        swatch_strip = QHBoxLayout()
        swatch_strip.setContentsMargins(0, 4, 0, 0)
        swatch_strip.setSpacing(8)

        pedal_swatch = QFrame()
        pedal_swatch.setObjectName("pedal_swatch")
        pedal_swatch.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        pedal_swatch.setFixedSize(14, 14)
        pedal_lbl = QLabel("Pedal Color")
        pedal_lbl.setProperty("variant", "muted")
        swatch_strip.addWidget(pedal_swatch)
        swatch_strip.addWidget(pedal_lbl)

        swatch_strip.addSpacing(12)

        border_swatch = QFrame()
        border_swatch.setObjectName("border_swatch")
        border_swatch.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        border_swatch.setFixedSize(14, 14)
        border_lbl = QLabel("Borders")
        border_lbl.setProperty("variant", "muted")
        swatch_strip.addWidget(border_swatch)
        swatch_strip.addWidget(border_lbl)

        swatch_strip.addStretch()
        outer.addLayout(swatch_strip)
        return panel

    def _build_prev_sidebar(self) -> QFrame:
        sidebar = QFrame()
        sidebar.setObjectName("sidebar")
        sidebar.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        sidebar.setFixedWidth(_ps(44))  # real sidebar is 44px collapsed

        v = QVBoxLayout(sidebar)
        v.setContentsMargins(0, 0, 0, 0)
        v.setSpacing(0)

        logo_row = QFrame()
        logo_row.setFixedHeight(_ps(48))
        logo_row.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self._prev_sidebar_logo = QLabel(logo_row)
        self._prev_sidebar_logo.setObjectName("nav_icon")
        self._prev_sidebar_logo.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._prev_sidebar_logo.setGeometry(_ps(12), _ps(14), _ps(22), _ps(22))
        self._prev_sidebar_logo.setScaledContents(True)
        self._prev_sidebar_logo.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        v.addWidget(logo_row)

        _nav_items = [
            ("music-note", True),
            ("waveform",   False),
            ("gear-six",   False),
            ("bug",        False),
        ]
        self._prev_nav_icon_labels: list[tuple[QLabel, str, bool]] = []
        for icon_name, is_active in _nav_items:
            btn_frame = QFrame()
            btn_frame.setObjectName("nav_btn")
            btn_frame.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
            btn_frame.setFixedHeight(_ps(48))
            btn_frame.setProperty("active",  "true" if is_active else "false")
            btn_frame.setProperty("hovered", "false")

            icon_lbl = QLabel(btn_frame)
            icon_lbl.setObjectName("nav_icon")
            icon_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
            icon_lbl.setGeometry(_ps(12), _ps(14), _ps(22), _ps(22))
            icon_lbl.setScaledContents(True)
            icon_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

            text_lbl = QLabel("", btn_frame)
            text_lbl.setObjectName("nav_label")
            text_lbl.setGeometry(_ps(44), 0, _ps(200), _ps(48))
            text_lbl.setProperty("highlighted", "true" if is_active else "false")
            text_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

            v.addWidget(btn_frame)
            self._prev_nav_icon_labels.append((icon_lbl, icon_name, is_active))

        v.addStretch()
        return sidebar

    def _build_prev_title_bar(self) -> QFrame:
        title_bar = QFrame()
        title_bar.setObjectName("collapsed_strip")
        title_bar.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        title_h = QHBoxLayout(title_bar)
        title_h.setContentsMargins(5, 3, 4, 3)
        title_h.setSpacing(2)
        app_name = QLabel("Hu<i>Midi</i>")
        app_name.setTextFormat(Qt.TextFormat.RichText)
        app_name.setObjectName("sidebar_logo_text")
        title_h.addWidget(app_name)
        title_h.addStretch()
        for sym in ("–", "□", "×"):  # en-dash, square, multiplication sign
            lbl = QLabel(sym)
            lbl.setProperty("variant", "muted")
            title_h.addWidget(lbl)
        return title_bar

    def _build_prev_file_strip(self) -> QFrame:
        file_strip = QFrame()
        file_strip.setObjectName("file_strip")
        file_strip.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        fs_h = QHBoxLayout(file_strip)
        fs_h.setContentsMargins(5, 3, 5, 3)
        fs_h.setSpacing(4)

        tile = QFrame()
        tile.setObjectName("file_strip_tile")
        tile.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        tile.setFixedSize(14, 14)
        tile_layout = QVBoxLayout(tile)
        tile_layout.setContentsMargins(0, 0, 0, 0)
        self._prev_file_tile_icon = QLabel()
        self._prev_file_tile_icon.setObjectName("file_strip_tile_icon")
        self._prev_file_tile_icon.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._prev_file_tile_icon.setFixedSize(QSize(8, 8))
        self._prev_file_tile_icon.setScaledContents(True)
        tile_layout.addWidget(self._prev_file_tile_icon, alignment=Qt.AlignmentFlag.AlignCenter)
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
        body_v.setContentsMargins(5, 5, 5, 3)
        body_v.setSpacing(4)
        body_v.addWidget(self._build_prev_performance_card())
        body_v.addWidget(self._build_prev_options_card())
        body_v.addWidget(self._build_prev_track_row())
        body_v.addStretch()
        return body

    def _build_prev_performance_card(self) -> QFrame:
        card = QFrame()
        card.setObjectName("section_card")
        card.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        v = QVBoxLayout(card)
        v.setContentsMargins(6, 3, 6, 4)
        v.setSpacing(2)

        title_row = QHBoxLayout()
        title_row.setSpacing(3)
        title = QLabel("PERFORMANCE")
        title.setProperty("variant", "section")
        title_row.addWidget(title)
        title_row.addStretch()
        self._prev_reset_btn = QPushButton()
        self._prev_reset_btn.setProperty("variant", "card_reset")
        self._prev_reset_btn.setFixedSize(14, 14)
        self._prev_reset_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        title_row.addWidget(self._prev_reset_btn)
        v.addLayout(title_row)

        tempo_row = QHBoxLayout()
        tempo_row.setSpacing(8)
        tempo_labels = QVBoxLayout()
        tempo_labels.setSpacing(1)
        tempo_labels.setContentsMargins(0, 0, 0, 0)
        tempo_lbl = QLabel("Tempo")
        tempo_desc = QLabel("% of original")
        tempo_desc.setProperty("variant", "muted")
        tempo_labels.addWidget(tempo_lbl)
        tempo_labels.addWidget(tempo_desc)
        tempo_row.addLayout(tempo_labels)
        self._prev_slider = QSlider(Qt.Orientation.Horizontal)
        self._prev_slider.setRange(0, 200)
        self._prev_slider.setValue(100)
        tempo_row.addWidget(self._prev_slider, 1)
        self._prev_spin = NoScrollDoubleSpinBox()
        self._prev_spin.setRange(0, 200)
        self._prev_spin.setValue(100.0)
        self._prev_spin.setSuffix(" %")
        self._prev_spin.setFixedWidth(38)
        tempo_row.addWidget(self._prev_spin)
        v.addLayout(tempo_row)

        pedal_row = QHBoxLayout()
        pedal_row.setSpacing(8)
        pedal_labels = QVBoxLayout()
        pedal_labels.setSpacing(1)
        pedal_labels.setContentsMargins(0, 0, 0, 0)
        pedal_lbl = QLabel("Pedal")
        pedal_desc = QLabel("generation algorithm")
        pedal_desc.setProperty("variant", "muted")
        pedal_labels.addWidget(pedal_lbl)
        pedal_labels.addWidget(pedal_desc)
        pedal_row.addLayout(pedal_labels)
        self._prev_combo = NoScrollComboBox()
        self._prev_combo.addItems(["Auto (Default)", "PedalAI", "Harmonic"])
        pedal_row.addWidget(self._prev_combo, 1)
        v.addLayout(pedal_row)
        return card

    def _build_prev_options_card(self) -> QFrame:
        card = QFrame()
        card.setObjectName("section_card")
        card.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        v = QVBoxLayout(card)
        v.setContentsMargins(6, 3, 6, 4)
        v.setSpacing(2)
        title = QLabel("OPTIONS")
        title.setProperty("variant", "section")
        v.addWidget(title)
        self._prev_check = ToggleSwitch("88-Key Layout")
        self._prev_check.setChecked(True)
        v.addWidget(self._prev_check)
        check_desc = QLabel("Map to the full 88-key piano")
        check_desc.setProperty("variant", "muted")
        check_desc.setContentsMargins(36, 0, 0, 0)
        v.addWidget(check_desc)
        return card

    def _build_prev_track_row(self) -> QWidget:
        container = QWidget()
        row = QHBoxLayout(container)
        row.setContentsMargins(0, 0, 0, 0)
        row.setSpacing(8)

        part_card = QFrame()
        part_card.setObjectName("part_card")
        part_card.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        pc_v = QVBoxLayout(part_card)
        pc_v.setContentsMargins(4, 3, 4, 3)
        pc_v.setSpacing(1)
        pc_title = QLabel("Track 1")
        pc_title.setObjectName("part_card_title")
        pc_meta = QLabel("C4-C6  Right")
        pc_meta.setObjectName("part_card_meta")
        pc_v.addWidget(pc_title)
        pc_v.addWidget(pc_meta)
        row.addWidget(part_card, 1)

        save_card = QFrame()
        save_card.setObjectName("save_card")
        save_card.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        sc_v = QVBoxLayout(save_card)
        sc_v.setContentsMargins(4, 3, 4, 3)
        sc_v.setSpacing(1)
        sc_title = QLabel("Demo.json")
        sc_title.setObjectName("part_card_title")
        sc_meta = QLabel("2 days ago")
        sc_meta.setObjectName("part_card_meta")
        sc_v.addWidget(sc_title)
        sc_v.addWidget(sc_meta)
        row.addWidget(save_card, 1)

        return container

    def _build_prev_transport_bar(self) -> QFrame:
        bar = QFrame()
        bar.setObjectName("transport_bar")
        bar.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        v = QVBoxLayout(bar)
        v.setContentsMargins(6, 4, 6, 4)
        v.setSpacing(3)

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

        action_row = QHBoxLayout()
        action_row.setSpacing(5)
        self._prev_play_btn = QPushButton()
        self._prev_play_btn.setObjectName("play_button")
        self._prev_play_btn.setIconSize(QSize(11, 11))
        self._prev_play_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        self._prev_stop_btn = QPushButton()
        self._prev_stop_btn.setObjectName("stop_button")
        self._prev_stop_btn.setIconSize(QSize(11, 11))
        self._prev_stop_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        action_row.addWidget(self._prev_play_btn)
        action_row.addWidget(self._prev_stop_btn)
        action_row.addStretch()
        self._prev_save_btn = QPushButton()
        self._prev_save_btn.setObjectName("save_button")
        self._prev_save_btn.setIconSize(QSize(11, 11))
        self._prev_save_btn.setCursor(Qt.CursorShape.PointingHandCursor)
        action_row.addWidget(self._prev_save_btn)
        v.addLayout(action_row)
        return bar

    # ── Panel animation ───────────────────────────────────────────────

    def _toggle_swatch_panel(self, expand: bool) -> None:
        """Slide the color-swatch panel in (expand=True) or out."""
        self._swatch_anim.stop()
        current = self._swatch_panel_widget.maximumWidth()
        if current > _SWATCH_WIDTH:
            current = _SWATCH_WIDTH if expand else 0
        end = _SWATCH_WIDTH if expand else 0
        if current == end:
            self._expand_panel_btn.setVisible(not expand)
            self._collapse_panel_btn.setVisible(expand)
            return
        self._swatch_anim.setStartValue(current)
        self._swatch_anim.setEndValue(end)
        self._swatch_anim.setEasingCurve(
            QEasingCurve.Type.OutCubic if expand else QEasingCurve.Type.InCubic
        )
        if expand:
            self._expand_panel_btn.setVisible(False)
            self._collapse_panel_btn.setVisible(True)
        self._swatch_anim.start()

    def _on_swatch_anim_finished(self) -> None:
        if self._swatch_panel_widget.maximumWidth() == 0:
            self._expand_panel_btn.setVisible(True)
            self._collapse_panel_btn.setVisible(False)

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

        self._builtin_label.setVisible(theme.builtin)
        for key, _lbl in _COLOR_FIELDS:
            self._swatches[key].set_color(getattr(theme, key))
            self._swatches[key].set_editable(True)

        self._del_btn.setEnabled(not theme.builtin)
        self._rename_btn.setEnabled(not theme.builtin)
        self._export_btn.setEnabled(True)
        self._revert_btn.setEnabled(False)

        self._preview(theme)

    def _on_color_changed(self, field: str, value: str) -> None:
        if self._current_theme is None:
            return
        if self._current_theme.builtin:
            base = self._current_theme
            candidate = _unique_copy_name(base.name, set(ThemeManager.all_themes().keys()))
            ThemeManager.save_custom(replace(base, name=candidate, builtin=False))
            self._populate_list(select_name=candidate)
        self._current_theme = replace(self._current_theme, **{field: value})
        self._mark_dirty()
        self._preview(self._current_theme)

    def _mark_dirty(self, *_) -> None:
        self._pending_save = True
        self._revert_btn.setEnabled(True)

    def _on_new(self) -> None:
        base = self._current_theme or list(BUILTIN_THEMES.values())[0]
        candidate = _unique_copy_name(base.name, set(ThemeManager.all_themes().keys()))
        ThemeManager.save_custom(replace(base, name=candidate, builtin=False))
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
            if ThemeManager.get_active_name() == name:
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
            QMessageBox.warning(self, "Name in Use", f'A theme named "{new_name}" already exists.')
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
        ThemeManager.save_custom(self._current_theme)
        self._pending_save = False
        self._revert_btn.setEnabled(False)
        self._populate_list(select_name=self._current_theme.name)

    def _on_revert(self) -> None:
        if self._current_theme is None:
            return
        themes = ThemeManager.all_themes()
        original = themes.get(self._current_theme.name)
        if original:
            self._current_theme = original
            for key, _lbl in _COLOR_FIELDS:
                self._swatches[key].set_color(getattr(original, key))
            self._preview(original)
        self._pending_save = False
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
            self._on_save()
        if self._current_theme:
            ThemeManager.set_active_name(self._current_theme.name)
            self.theme_applied.emit(self._current_theme.name)
        self.accept()

    # ── Helpers ───────────────────────────────────────────────────────

    def _refresh_io_icons(self, colors: ThemeColors) -> None:
        # Toolbar PhIconLabels are recolored by the dialog stylesheet via
        # qproperty-*. Only the inspect button (a plain QPushButton) needs an
        # explicit icon render here.
        self._inspect_btn.setIcon(ph_icon("inspect-mode", colors.text_secondary))

    def _preview(self, theme: ThemeColors) -> None:
        """Apply the previewed theme to the preview panel only."""
        play_bg  = _mix(theme.bg_primary, theme.accent_play, 0.15)
        play_hov = _mix(theme.bg_primary, theme.accent_play, 0.28)
        play_bdr = _mix(theme.accent_play, theme.bg_secondary, 0.40)
        stop_bg  = _mix(theme.bg_primary, theme.accent_stop, 0.15)
        stop_hov = _mix(theme.bg_primary, theme.accent_stop, 0.28)
        stop_bdr = _mix(theme.accent_stop, theme.bg_secondary, 0.40)
        ss = generate_stylesheet(theme)
        self._preview_panel.setStyleSheet(
            ss
            + f"\nQFrame#preview_window {{ border: 1px solid {theme.border}; border-radius: 4px;"
              f" background-color: {theme.bg_primary}; font-size: 9px; }}"
            + f"\nQFrame#preview_window * {{ font-size: 9px; }}"
            + f"\nQPushButton#play_button {{ background-color: {play_bg}; border-color: {play_bdr}; }}"
            + f"\nQPushButton#play_button:hover {{ background-color: {play_hov}; }}"
            + f"\nQPushButton#stop_button {{ background-color: {stop_bg}; border-color: {stop_bdr}; }}"
            + f"\nQPushButton#stop_button:hover {{ background-color: {stop_hov}; }}"
            + f"\nQFrame#pedal_swatch {{ background-color: {theme.pedal_color}; border-radius: 4px; }}"
            + f"\nQFrame#border_swatch {{ background-color: {theme.border}; border-radius: 2px; }}"
        )
        _ti = _ps(22)   # transport icon: half of 22px real size
        self._prev_play_btn.setIcon(ph_icon("play",        theme.accent_play, _ti))
        self._prev_stop_btn.setIcon(ph_icon("stop",        theme.accent_stop, _ti))
        self._prev_save_btn.setIcon(ph_icon("floppy-disk", theme.accent_save, _ti))
        self._prev_reset_btn.setIcon(ph_icon("arrow-counter-clockwise", theme.text_secondary, 7))
        self._prev_reset_btn.setIconSize(QSize(7, 7))
        self._prev_file_tile_icon.setPixmap(ph_icon("music-note", theme.accent, 8).pixmap(16, 16))
        _ni = 9  # nav/logo icon: 9px logical in collapsed-sidebar preview
        self._prev_sidebar_logo.setPixmap(
            ph_icon("music-note", theme.text_primary, _ni).pixmap(_ni * 2, _ni * 2)
        )
        for icon_lbl, icon_name, is_active in self._prev_nav_icon_labels:
            color = theme.text_primary if is_active else theme.text_secondary
            icon_lbl.setPixmap(ph_icon(icon_name, color, _ni).pixmap(_ni * 2, _ni * 2))
        # _prev_check (ToggleSwitch) is re-colored by the preview stylesheet via
        # qproperty-*; no manual color push is needed here.
        if self._hover_overlay is not None:
            self._hover_overlay.set_accent(theme.accent)

    def _apply_own_stylesheet(self) -> None:
        active = ThemeManager.get_active()
        self.setStyleSheet(generate_stylesheet(active))
        self._refresh_io_icons(active)

    def _sync_toolbar_btn_widths(self) -> None:
        h = self._combo.sizeHint().height()
        for btn in (
            self._expand_panel_btn, self._collapse_panel_btn,
            self._new_btn, self._del_btn, self._rename_btn,
            self._export_btn, self._import_btn,
        ):
            btn.setFixedWidth(h)

    def _cleanup_inspect(self) -> None:
        QApplication.instance().removeEventFilter(self._inspect_filter)
        if self._hover_poll_timer is not None:
            self._hover_poll_timer.stop()
            self._hover_poll_timer = None
        if self._active_color_dlg is not None:
            self._active_color_dlg.close()
            self._active_color_dlg = None
        if self._hover_overlay is not None:
            self._hover_overlay.deleteLater()
            self._hover_overlay = None

    def _toggle_inspect(self, active: bool) -> None:
        self._inspect_filter.set_active(active)
        if active:
            self._inspect_hint.setText("Right-click any element to pick its color")
            shape = Qt.CursorShape.CrossCursor
            accent = (self._current_theme.accent if self._current_theme
                      else ThemeManager.get_active().accent)
            self._hover_overlay = _HoverOverlay(self._preview_panel, accent)
            self._hover_overlay.show()
            self._last_polled_widget = None
            self._hover_poll_timer = QTimer(self)
            self._hover_poll_timer.timeout.connect(self._poll_inspect_hover)
            self._hover_poll_timer.start(40)
        else:
            self._inspect_hint.setText("")
            shape = Qt.CursorShape.ArrowCursor
            if self._hover_poll_timer is not None:
                self._hover_poll_timer.stop()
                self._hover_poll_timer = None
            self._last_polled_widget = None
            if self._active_color_dlg is not None:
                self._active_color_dlg.close()
                self._active_color_dlg = None
            if self._hover_overlay is not None:
                self._hover_overlay.deleteLater()
                self._hover_overlay = None
        cursor = QCursor(shape)
        self._preview_panel.setCursor(cursor)
        for child in self._preview_panel.findChildren(QWidget):
            child.setCursor(cursor)

    def _poll_inspect_hover(self) -> None:
        widget = QApplication.widgetAt(QCursor.pos())
        if widget is self._last_polled_widget:
            return
        self._last_polled_widget = widget
        self._on_widget_hovered(widget)

    def _is_in_preview_window(self, widget: QWidget) -> bool:
        w = widget
        while w is not None:
            if w is self._preview_window:
                return True
            w = w.parent()
        return False

    def _on_widget_hovered(self, widget: QWidget | None) -> None:
        if self._hover_overlay is None:
            return
        if widget is None or not self._is_in_preview_window(widget):
            self._hover_overlay.set_rects([])
            return
        result = None
        w: QWidget | None = widget
        while w is not None and self._is_in_preview_window(w):
            result = _field_for_widget(w)
            if result is not None:
                break
            w = w.parent()
        if result is None:
            self._hover_overlay.set_rects([])
            return
        field = result[0]
        panel = self._preview_panel
        rects: list[QRect] = []
        candidates = [self._preview_window, *self._preview_window.findChildren(QWidget)]
        for child in candidates:
            if isinstance(child, _HoverOverlay):
                continue
            cls_name = type(child).__name__
            if cls_name in ("QFrame", "QWidget") and not child.objectName():
                continue
            child_result = _field_for_widget(child)
            if child_result is not None and child_result[0] == field:
                if child.isVisible() and child.width() > 0 and child.height() > 0:
                    mapped = child.mapTo(panel, QPoint(0, 0))
                    rects.append(QRect(mapped, child.size()))
        self._hover_overlay.set_rects(rects)

    def _on_widget_picked(self, widget: QWidget, global_pos) -> None:
        if self._current_theme is None:
            return
        if self._current_theme.builtin:
            base = self._current_theme
            candidate = _unique_copy_name(base.name, set(ThemeManager.all_themes().keys()))
            ThemeManager.save_custom(replace(base, name=candidate, builtin=False))
            self._populate_list(select_name=candidate)
            self._inspect_hint.setText(f'Duplicated as "{candidate}" -- editing below')

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
