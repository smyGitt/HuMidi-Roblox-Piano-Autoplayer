from PySide6.QtWidgets import QLabel
from PySide6.QtCore import Qt, QSize


class ElidedLabel(QLabel):
    """QLabel that elides text with '...' when it exceeds the available width.

    Set text via set_full_text() instead of setText() so the full string is
    retained for re-elision on resize. Reading .text() returns the elided
    display string; call .full_text to get the original.

    sizeHint and minimumSizeHint both return width=0 so the layout never
    tries to allocate more horizontal space than is actually available.
    The label takes whatever width the parent gives it and elides accordingly.
    """

    def __init__(self, text: str = "", parent=None):
        super().__init__(parent)
        self._full_text = text
        self.setWordWrap(False)
        self.setMinimumWidth(0)

    def sizeHint(self) -> QSize:
        return QSize(0, super().sizeHint().height())

    def minimumSizeHint(self) -> QSize:
        return QSize(0, super().minimumSizeHint().height())

    @property
    def full_text(self) -> str:
        return self._full_text

    def set_full_text(self, text: str) -> None:
        self._full_text = text
        self._update_elided()

    def resizeEvent(self, event):
        super().resizeEvent(event)
        self._update_elided()

    def _update_elided(self) -> None:
        elided = self.fontMetrics().elidedText(
            self._full_text, Qt.TextElideMode.ElideRight, self.width()
        )
        super().setText(elided)
