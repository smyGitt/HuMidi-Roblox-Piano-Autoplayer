from PyQt6.QtWidgets import QPushButton
from PyQt6.QtCore import Qt, QSize, pyqtProperty
from PyQt6.QtGui import QColor


class HuMidiButton(QPushButton):
    """Standard action button for the HuMidi UI.

    Enforces PointingHandCursor and WA_StyledBackground on every instance and
    accepts an optional tooltip. All visual styling comes from the central QSS;
    differentiate buttons with objectName or a variant property, never inline.

    When icon_name is given the button renders a Phosphor icon whose color is
    supplied by QSS via qproperty-iconColor; the glyph (icon_name) is a Python
    state, swapped with set_icon_name (e.g. play <-> pause).
    """

    def __init__(self, text: str = "", *, tooltip: str = "",
                 icon_name: str | None = None, icon_size: int = 22, parent=None):
        super().__init__(text, parent)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        # Prevent keyboard focus so pynput-injected Space key events (used for
        # the sustain pedal) cannot activate a focused transport button.
        self.setFocusPolicy(Qt.FocusPolicy.NoFocus)
        self._icon_name = icon_name
        self._icon_size = icon_size
        # Color slot (overwritten by QSS qproperty-iconColor on stylesheet apply).
        self._icon_color = QColor("#dcdcf0")
        if tooltip:
            self.setToolTip(tooltip)
        if icon_name:
            self._render_icon()

    # -- QSS-driven icon color -----------------------------------------------

    @pyqtProperty(QColor)
    def iconColor(self) -> QColor:
        return self._icon_color

    @iconColor.setter
    def iconColor(self, c: QColor) -> None:
        self._icon_color = c
        self._render_icon()

    def set_icon_name(self, name: str) -> None:
        """Swap the rendered glyph (a Python state, e.g. play <-> pause)."""
        if name != self._icon_name:
            self._icon_name = name
            self._render_icon()

    def _render_icon(self) -> None:
        if not self._icon_name:
            return
        from ui.widgets.ph_icon import ph_icon
        self.setIconSize(QSize(self._icon_size, self._icon_size))
        self.setIcon(ph_icon(self._icon_name, self._icon_color.name(), self._icon_size))
