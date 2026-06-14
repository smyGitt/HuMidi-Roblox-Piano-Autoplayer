from typing import Callable


class EventInjectableMixin:
    """Plain Python mixin that delegates Qt event handling to injected callbacks.

    Place before the Qt base in the MRO:
        class MyWidget(EventInjectableMixin, QLabel): ...

    Each overridden event runs all registered callbacks in insertion order, then
    calls super() to preserve the Qt event chain. Callbacks may be stacked by
    calling inject_events() multiple times.

    Callback signatures:
        hover_enter / hover_leave : Callable[[QWidget], None]
        press / release           : Callable[[QWidget, QMouseEvent], None]
    """

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._ev_hover_enter: list[Callable] = []
        self._ev_hover_leave: list[Callable] = []
        self._ev_press:       list[Callable] = []
        self._ev_release:     list[Callable] = []

    def inject_events(
        self,
        *,
        hover_enter: Callable | None = None,
        hover_leave: Callable | None = None,
        press:       Callable | None = None,
        release:     Callable | None = None,
    ) -> None:
        """Append one or more callbacks to the corresponding event slots."""
        if hover_enter: self._ev_hover_enter.append(hover_enter)
        if hover_leave: self._ev_hover_leave.append(hover_leave)
        if press:       self._ev_press.append(press)
        if release:     self._ev_release.append(release)

    def enterEvent(self, event):
        for fn in self._ev_hover_enter:
            fn(self)
        super().enterEvent(event)

    def leaveEvent(self, event):
        for fn in self._ev_hover_leave:
            fn(self)
        super().leaveEvent(event)

    def mousePressEvent(self, event):
        for fn in self._ev_press:
            fn(self, event)
        super().mousePressEvent(event)

    def mouseReleaseEvent(self, event):
        for fn in self._ev_release:
            fn(self, event)
        super().mouseReleaseEvent(event)
