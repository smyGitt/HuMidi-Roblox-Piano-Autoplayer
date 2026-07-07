"""Apply/Discard toast that slides up from the bottom of PlaybackTab.

Shown when compiled events are stale relative to the current UI config.
The parent (PlaybackTab) is responsible for positioning and resizing on
parent resize events via _reposition_toast().
"""

from PySide6.QtCore import QEasingCurve, QPoint, QPropertyAnimation, Signal
from PySide6.QtWidgets import QFrame, QHBoxLayout, QLabel, QPushButton


_MSG = (
    "Press ‘Apply’ to apply current changes. "
    "This will require re-generation of affected events."
)


class ApplyToast(QFrame):
    """Sticky-bottom toast with Apply and Discard actions."""

    apply_clicked   = Signal()
    discard_clicked = Signal()

    HEIGHT = 56

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setProperty("variant", "toast")
        self.setFixedHeight(self.HEIGHT)
        self._visible = False
        self._setup_ui()
        self._slide_anim = QPropertyAnimation(self, b"pos")
        self._slide_anim.setDuration(220)
        self._slide_anim.setEasingCurve(QEasingCurve.Type.OutCubic)
        self._finished_connected_to_hide = False
        self._shake_anim = None
        self.hide()

    def _setup_ui(self) -> None:
        layout = QHBoxLayout(self)
        layout.setContentsMargins(16, 0, 16, 0)
        layout.setSpacing(12)

        self._msg_label = QLabel()
        self._msg_label.setWordWrap(False)
        layout.addWidget(self._msg_label, 1)

        self._apply_btn = QPushButton("Apply")
        self._apply_btn.setProperty("variant", "accent")
        self._apply_btn.clicked.connect(self.apply_clicked)
        layout.addWidget(self._apply_btn)

        self._discard_btn = QPushButton("Discard")
        self._discard_btn.clicked.connect(self.discard_clicked)
        layout.addWidget(self._discard_btn)

    # -- Message --------------------------------------------------------------

    def update_message(self, notes_dirty: bool, pedal_dirty_independent: bool) -> None:
        self._msg_label.setText(_MSG)

    # -- Visibility -----------------------------------------------------------

    def is_toast_visible(self) -> bool:
        return self._visible

    def show_sliding(self) -> None:
        if not self.parent():
            return
        pw, ph = self.parent().width(), self.parent().height()
        self.resize(pw, self.HEIGHT)
        hidden_y = ph
        shown_y  = ph - self.HEIGHT
        self.move(0, hidden_y)
        self.show()
        self._visible = True
        if self._finished_connected_to_hide:
            self._slide_anim.finished.disconnect(self.hide)
            self._finished_connected_to_hide = False
        self._slide_anim.stop()
        self._slide_anim.setStartValue(QPoint(0, hidden_y))
        self._slide_anim.setEndValue(QPoint(0, shown_y))
        self._slide_anim.start()

    def hide_sliding(self) -> None:
        if not self._visible:
            return
        self._visible = False
        if not self.parent():
            self.hide()
            return
        ph = self.parent().height()
        self._slide_anim.stop()
        self._slide_anim.setStartValue(QPoint(0, ph - self.HEIGHT))
        self._slide_anim.setEndValue(QPoint(0, ph))
        if not self._finished_connected_to_hide:
            self._slide_anim.finished.connect(self.hide)
            self._finished_connected_to_hide = True
        self._slide_anim.start()

    # -- Shake ----------------------------------------------------------------

    def shake(self) -> None:
        """Brief horizontal shake to draw attention when play is blocked."""
        if not self._visible or not self.parent():
            return
        ph = self.parent().height()
        base = QPoint(0, ph - self.HEIGHT)
        anim = QPropertyAnimation(self, b"pos")
        anim.setDuration(380)
        anim.setKeyValueAt(0.00, base)
        anim.setKeyValueAt(0.14, QPoint(-10, ph - self.HEIGHT))
        anim.setKeyValueAt(0.28, QPoint( 10, ph - self.HEIGHT))
        anim.setKeyValueAt(0.42, QPoint( -8, ph - self.HEIGHT))
        anim.setKeyValueAt(0.57, QPoint(  8, ph - self.HEIGHT))
        anim.setKeyValueAt(0.71, QPoint( -4, ph - self.HEIGHT))
        anim.setKeyValueAt(0.85, QPoint(  4, ph - self.HEIGHT))
        anim.setKeyValueAt(1.00, base)
        anim.start(QPropertyAnimation.DeletionPolicy.DeleteWhenStopped)
        self._shake_anim = anim
