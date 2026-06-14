import weakref
from typing import Callable, TYPE_CHECKING

from PyQt6.QtWidgets import QLabel, QWidget
from PyQt6.QtCore import Qt, pyqtSignal as Signal
from PyQt6.QtGui import QPixmap, QMouseEvent

from ui.widgets.event_injectable import EventInjectableMixin

if TYPE_CHECKING:
    from ui.theme import ThemeColors


class PhIconLabel(EventInjectableMixin, QLabel):
    """QLabel that renders a Phosphor Duotone icon with injected hover behavior.

    Construct with an icon_name (Phosphor duotone stem, e.g. "folder-open") and
    a logical size in pixels. Register with IconProvider.instance().register() to
    attach hover color-swap and theme tracking automatically.

    Pass hover_icon_name to swap to a different icon stem on hover (e.g. show
    "folder-closed" at rest and "folder-open" on hover). When omitted, both
    states render the same icon_name with different colors.

    clicked is emitted on left-button release so callers can wire it like a
    QPushButton signal without subclassing or adding custom press handlers.
    """

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
        self._color_fn: Callable | None = None
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
        phys = self.size * 2
        n = ph_icon(self.icon_name, normal_hex, self.size).pixmap(phys, phys)
        n.setDevicePixelRatio(2.0)
        self.normal_pixmap = n
        hover_name = self.hover_icon_name if self.hover_icon_name is not None else self.icon_name
        h = ph_icon(hover_name, hover_hex, self.size).pixmap(phys, phys)
        h.setDevicePixelRatio(2.0)
        self.hover_pixmap = h
        self.setPixmap(self.normal_pixmap)

    def apply_colors(self, colors: "ThemeColors") -> None:
        """Derive hex pair from registered color_fn and call set_colors."""
        if self._color_fn is not None:
            normal, hover = self._color_fn(colors)
            self.set_colors(normal, hover)

    def mouseReleaseEvent(self, event: QMouseEvent) -> None:
        if event.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit()
        super().mouseReleaseEvent(event)


class IconProvider:
    """Singleton registry for PhIconLabel instances.

    Tracks every live PhIconLabel via weak references. A single call to
    notify_theme_changed(colors) pushes updated colors to all registered labels,
    replacing per-widget update_icon_color() calls scattered across MainWindowUI.

    Standard hover behavior (pixmap swap on enter/leave) is injected once inside
    register(). Custom per-instance behaviors are added via label.inject_events()
    at the call site after registration.
    """

    _instance: "IconProvider | None" = None

    def __init__(self) -> None:
        self._labels: weakref.WeakSet[PhIconLabel] = weakref.WeakSet()

    @classmethod
    def instance(cls) -> "IconProvider":
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    def register(
        self,
        label: PhIconLabel,
        color_fn: Callable[["ThemeColors"], tuple[str, str]],
    ) -> None:
        """Register label and inject standard hover swap behavior.

        color_fn receives a ThemeColors and returns (normal_hex, hover_hex).
        Called once at UI construction time, not on every theme change.
        """
        self._labels.add(label)
        label._color_fn = color_fn
        label.inject_events(
            hover_enter=lambda w: w.setPixmap(w.hover_pixmap)  if w.hover_pixmap  else None,
            hover_leave=lambda w: w.setPixmap(w.normal_pixmap) if w.normal_pixmap else None,
        )

    def notify_theme_changed(self, colors: "ThemeColors") -> None:
        """Push updated colors to all live registered labels."""
        for label in list(self._labels):
            label.apply_colors(colors)
