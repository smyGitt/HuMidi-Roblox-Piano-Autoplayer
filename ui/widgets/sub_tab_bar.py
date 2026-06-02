from PyQt6.QtWidgets import QFrame, QHBoxLayout, QPushButton
from PyQt6.QtCore import Qt, pyqtSignal as Signal


class _SubTabButton(QPushButton):
    """Single button in the SubTabBar. Uses plain text because QPushButton
    does not render HTML in setText -- previous rich-text styling was
    showing up literally in the UI."""

    def __init__(self, ordinal: str, label: str, parent=None):
        super().__init__(parent)
        self.setObjectName("sub_tab_btn")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setProperty("active", "false")
        self._ordinal = ordinal
        self._label = label
        self.setText(f"{self._ordinal}  {self._label}")

    def set_active(self, active: bool) -> None:
        self.setProperty("active", "true" if active else "false")
        self.style().unpolish(self)
        self.style().polish(self)
        self.update()


class SubTabBar(QFrame):
    """Horizontal manuscript-style sub-tab bar with roman ordinals."""

    tab_changed = Signal(int)

    TABS = [
        ("I",   "File"),
        ("II",  "Playback"),
        ("III", "Humanize"),
    ]

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setObjectName("sub_tab_bar")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)

        hbox = QHBoxLayout(self)
        hbox.setContentsMargins(0, 0, 0, 0)
        hbox.setSpacing(0)

        self._btns: list[_SubTabButton] = []
        for i, (ordinal, label) in enumerate(self.TABS):
            btn = _SubTabButton(ordinal, label)
            btn.clicked.connect(lambda checked=False, idx=i: self._on_tab(idx))
            hbox.addWidget(btn)
            self._btns.append(btn)

        hbox.addStretch(1)
        self._btns[0].set_active(True)

    def _on_tab(self, index: int) -> None:
        for i, btn in enumerate(self._btns):
            btn.set_active(i == index)
        self.tab_changed.emit(index)

    def set_active(self, index: int) -> None:
        self._on_tab(index)
