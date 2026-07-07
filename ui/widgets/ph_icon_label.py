from PySide6.QtWidgets import QLabel, QWidget
from PySide6.QtCore import Qt, Property, Signal
from PySide6.QtGui import QPixmap, QColor, QMouseEvent

from ui.widgets.event_injectable import EventInjectableMixin


class PhIconLabel(EventInjectableMixin, QLabel):
    """QLabel that renders a Phosphor Duotone icon with injected hover behavior.

    Construct with an icon_name (Phosphor duotone stem, e.g. "folder-open") and
    a logical size in pixels. Register with IconProvider.instance().register() to
    attach the hover color-swap behavior.

    Icon colors are supplied by QSS via qproperty-* (iconColor, iconHoverColor);
    set a [variant="icon_accent"] / [variant="icon_danger"] property to pick a
    different hover color. Both pixmap states are re-rendered whenever a color
    slot changes, so re-applying the stylesheet re-themes the icon.

    Pass hover_icon_name to swap to a different icon stem on hover (e.g. show
    "folder-closed" at rest and "folder-open" on hover). When omitted, both
    states render the same icon_name with different colors.

    clicked is emitted on left-button release so callers can wire it like a
    QPushButton signal without subclassing or adding custom press handlers.
    """

    _INSET = 2

    clicked = Signal()

    def __init__(
        self,
        icon_name: str,
        size: int = 20,
        parent: QWidget | None = None,
        *,
        hover_icon_name: str | None = None,
        allow_vertical_expansion: bool = False,
    ):
        super().__init__(parent)
        self.icon_name = icon_name
        self.hover_icon_name = hover_icon_name
        self.size = size
        self.normal_pixmap: QPixmap | None = None
        self.hover_pixmap:  QPixmap | None = None
        # Color slots (overwritten by QSS qproperty-* on stylesheet apply).
        self._icon_color = QColor("#7878a0")
        self._icon_hover_color = QColor("#dcdcf0")
        self.setAttribute(Qt.WidgetAttribute.WA_Hover, True)
        self.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.setScaledContents(False)
        if allow_vertical_expansion:
            self.setFixedWidth(size)
        else:
            self.setFixedSize(size, size)

    def set_colors(self, normal_hex: str, hover_hex: str) -> None:
        """Re-render both pixmaps from hex strings and refresh the display."""
        from ui.widgets.ph_icon import ph_icon
        render = self.size - 2 * self._INSET
        phys = render * 2
        n = ph_icon(self.icon_name, normal_hex, render).pixmap(phys, phys)
        n.setDevicePixelRatio(2.0)
        self.normal_pixmap = n
        hover_name = self.hover_icon_name if self.hover_icon_name is not None else self.icon_name
        h = ph_icon(hover_name, hover_hex, render).pixmap(phys, phys)
        h.setDevicePixelRatio(2.0)
        self.hover_pixmap = h
        self.setPixmap(self.normal_pixmap)

    # -- QSS-driven color slots ----------------------------------------------

    @Property(QColor)
    def iconColor(self) -> QColor:
        return self._icon_color

    @iconColor.setter
    def iconColor(self, c: QColor) -> None:
        self._icon_color = c
        self.set_colors(self._icon_color.name(), self._icon_hover_color.name())

    @Property(QColor)
    def iconHoverColor(self) -> QColor:
        return self._icon_hover_color

    @iconHoverColor.setter
    def iconHoverColor(self, c: QColor) -> None:
        self._icon_hover_color = c
        self.set_colors(self._icon_color.name(), self._icon_hover_color.name())

    def mouseReleaseEvent(self, event: QMouseEvent) -> None:
        if event.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit()
        super().mouseReleaseEvent(event)


class IconProvider:
    """Singleton that injects the standard hover behavior into PhIconLabel.

    Icon colors are driven by QSS via qproperty-* (see PhIconLabel), so this
    provider no longer broadcasts colors; its sole job is to inject the
    pixmap-swap hover behavior once at registration time. Custom per-instance
    behaviors are added via label.inject_events() at the call site afterward.
    """

    _instance: "IconProvider | None" = None

    @classmethod
    def instance(cls) -> "IconProvider":
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    def register(self, label: PhIconLabel) -> None:
        """Inject the standard hover swap behavior (pixmap swap on enter/leave)."""
        label.inject_events(
            hover_enter=lambda w: w.setPixmap(w.hover_pixmap)  if w.hover_pixmap  else None,
            hover_leave=lambda w: w.setPixmap(w.normal_pixmap) if w.normal_pixmap else None,
        )
