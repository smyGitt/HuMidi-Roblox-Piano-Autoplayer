from PyQt6.QtWidgets import QFrame, QLabel
from PyQt6.QtCore import Qt, QEvent, QSize, pyqtSignal as Signal
from PyQt6.QtGui import QPixmap


class NavButton(QFrame):
    """Sidebar nav item: Phosphor Duotone icon on the left, label on the right.

    Children use fixed geometry instead of a QHBoxLayout so their positions
    never change during sidebar width animation. The sidebar clips their
    rendering at its current boundary, producing a clean slide-in/out effect.

    Call update_icon_colors(normal_hex, active_hex, size) after construction
    (and again on every theme change) to supply rendered pixmaps. Until that
    method is called the icon slot is empty.
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

    def update_icon_colors(self, normal_hex: str, active_hex: str,
                           size: int | None = None) -> None:
        """Re-render both pixmap states using the given hex color strings."""
        from ui.widgets.ph_icon import ph_icon
        sz = size or self._ICON_SIZE
        self._pix_normal = ph_icon(self._icon_name, normal_hex, sz).pixmap(sz * 2, sz * 2)
        self._pix_active = ph_icon(self._icon_name, active_hex, sz).pixmap(sz * 2, sz * 2)
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
