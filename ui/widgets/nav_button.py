from PyQt6.QtWidgets import QFrame, QLabel
from PyQt6.QtCore import Qt, QEvent, pyqtProperty, pyqtSignal as Signal
from PyQt6.QtGui import QPixmap, QColor


class NavButton(QFrame):
    """Sidebar nav item: Phosphor Duotone icon on the left, label on the right.

    Children use fixed geometry instead of a QHBoxLayout so their positions
    never change during sidebar width animation. The sidebar clips their
    rendering at its current boundary, producing a clean slide-in/out effect.

    Icon colors are supplied by QSS via qproperty-* (iconColorNormal,
    iconColorActive); both pixmap states are re-rendered whenever a color slot
    changes.
    """

    clicked = Signal()

    _ICON_SIZE = 22
    _ICON_X    = 12
    _ICON_Y    = 13   # (48 - 22) // 2
    _LABEL_X   = 44   # _ICON_X + _ICON_SIZE + 10 (gap)
    _LABEL_W   = 200

    def __init__(self, icon_name: str, label: str, parent=None):
        super().__init__(parent)
        self._icon_name = icon_name
        self._pix_normal: QPixmap | None = None
        self._pix_active: QPixmap | None = None
        # Color slots (overwritten by QSS qproperty-* on stylesheet apply).
        self._icon_normal = QColor("#7878a0")
        self._icon_active = QColor("#dcdcf0")

        self.setObjectName("nav_btn")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setFixedHeight(48)
        self.setProperty("active",  "false")
        self.setProperty("hovered", "false")

        self._icon_lbl = QLabel(self)
        self._icon_lbl.setObjectName("nav_icon")
        self._icon_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._icon_lbl.setGeometry(self._ICON_X, self._ICON_Y, self._ICON_SIZE, self._ICON_SIZE)
        self._icon_lbl.setScaledContents(True)
        self._icon_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

        self._text_lbl = QLabel(label, self)
        self._text_lbl.setObjectName("nav_label")
        self._text_lbl.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
        self._text_lbl.setGeometry(self._LABEL_X, 0, self._LABEL_W, 48)
        self._text_lbl.setProperty("highlighted", "false")
        self._text_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

    # -- QSS-driven color slots ----------------------------------------------

    @pyqtProperty(QColor)
    def iconColorNormal(self) -> QColor:
        return self._icon_normal

    @iconColorNormal.setter
    def iconColorNormal(self, c: QColor) -> None:
        self._icon_normal = c
        self._render_icons()

    @pyqtProperty(QColor)
    def iconColorActive(self) -> QColor:
        return self._icon_active

    @iconColorActive.setter
    def iconColorActive(self, c: QColor) -> None:
        self._icon_active = c
        self._render_icons()

    def _render_icons(self) -> None:
        """Re-render both pixmap states from the current color slots."""
        from ui.widgets.ph_icon import ph_icon
        sz = self._ICON_SIZE
        self._pix_normal = ph_icon(self._icon_name, self._icon_normal.name(), sz).pixmap(sz * 2, sz * 2)
        self._pix_active = ph_icon(self._icon_name, self._icon_active.name(), sz).pixmap(sz * 2, sz * 2)
        is_hi = self.property("active") == "true" or self.property("hovered") == "true"
        self._icon_lbl.setPixmap(self._pix_active if is_hi else self._pix_normal)

    def mousePressEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit()
        super().mousePressEvent(event)

    def enterEvent(self, event):
        self._update("hovered", True)
        super().enterEvent(event)

    def leaveEvent(self, event):
        self._update("hovered", False)
        super().leaveEvent(event)

    def set_active(self, active: bool) -> None:
        self._update("active", active)

    def changeEvent(self, event: QEvent) -> None:
        if event.type() == QEvent.Type.EnabledChange and not self.isEnabled():
            self.setProperty("active",  "false")
            self.setProperty("hovered", "false")
            self._text_lbl.setProperty("highlighted", "false")
            self._text_lbl.style().unpolish(self._text_lbl)
            self._text_lbl.style().polish(self._text_lbl)
            self.style().unpolish(self)
            self.style().polish(self)
            if self._pix_normal is not None:
                self._icon_lbl.setPixmap(self._pix_normal)
            self.update()
        super().changeEvent(event)

    def _update(self, key: str, value: bool) -> None:
        self.setProperty(key, "true" if value else "false")
        active  = self.property("active")  == "true"
        hovered = self.property("hovered") == "true"
        hi = active or hovered
        hi_str = "true" if hi else "false"
        self._text_lbl.setProperty("highlighted", hi_str)
        self._text_lbl.style().unpolish(self._text_lbl)
        self._text_lbl.style().polish(self._text_lbl)
        self.style().unpolish(self)
        self.style().polish(self)
        if self._pix_normal is not None and self._pix_active is not None:
            self._icon_lbl.setPixmap(self._pix_active if hi else self._pix_normal)
        self.update()
