from PyQt6.QtWidgets import QFrame, QVBoxLayout, QHBoxLayout, QLabel
from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QPainter, QPen, QColor


class AnimatedDashedCard(QFrame):
    """QFrame subclass that draws an animated (marching-ants) dashed border.

    Used by make_card(dashed_border=True). The border is drawn entirely in
    paintEvent; the CSS rule for section_card_dashed provides only the
    background color. Call set_colors() on each theme change, and
    set_drag_active() to switch to the accent-colored drag-over state.
    """

    _DASH_PATTERN = [6.0, 4.0]  # dash length, gap length in pen-width units
    _PATTERN_CYCLE = 10.0       # sum of _DASH_PATTERN
    _BORDER_RADIUS = 5
    _PEN_WIDTH = 1.5

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setObjectName("section_card_dashed")
        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self._dash_offset = 0.0
        self._border_color = QColor("#32324a")
        self._drag_border  = QColor("#5b8dee")
        self._drag_bg      = QColor(91, 141, 238, 30)
        self._drag_active  = False

        self._timer = QTimer(self)
        self._timer.timeout.connect(self._tick)

    def set_colors(self, border: str, accent: str) -> None:
        self._border_color = QColor(border)
        self._drag_border  = QColor(accent)
        drag_bg = QColor(accent)
        drag_bg.setAlphaF(0.12)
        self._drag_bg = drag_bg
        self.update()

    def set_drag_active(self, active: bool) -> None:
        if self._drag_active == active:
            return
        self._drag_active = active
        if active:
            self._timer.start(40)
        else:
            self._timer.stop()
            self._dash_offset = 0.0
        self.update()

    def _tick(self) -> None:
        self._dash_offset = (self._dash_offset + 0.5) % self._PATTERN_CYCLE
        self.update()

    def paintEvent(self, event) -> None:
        super().paintEvent(event)
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)
        r = self.rect().adjusted(1, 1, -1, -1)
        if self._drag_active:
            painter.setBrush(self._drag_bg)
            painter.setPen(Qt.PenStyle.NoPen)
            painter.drawRoundedRect(r, self._BORDER_RADIUS, self._BORDER_RADIUS)
        pen = QPen(self._drag_border if self._drag_active else self._border_color)
        pen.setWidthF(self._PEN_WIDTH)
        pen.setStyle(Qt.PenStyle.CustomDashLine)
        pen.setDashPattern(self._DASH_PATTERN)
        pen.setDashOffset(self._dash_offset)
        painter.setPen(pen)
        painter.setBrush(Qt.BrushStyle.NoBrush)
        painter.drawRoundedRect(r, self._BORDER_RADIUS, self._BORDER_RADIUS)


def make_card(
    title: str,
    title_buttons=None,
    header_widgets=None,
    footer_widgets=None,
    outer_margins=None,
    row_h_pad: int = 0,
    widget=None,
    dashed_border: bool = False,
) -> tuple:
    """Return a styled section card (QFrame) and its content QVBoxLayout.

    title_buttons   -- widgets placed inline-right of the title label
    header_widgets  -- widgets in a row below the title, above the body
    footer_widgets  -- widgets in a row below the body
    outer_margins   -- (l, t, r, b) override for the card's outer layout;
                       defaults to (14, 6, 14, 7)
    row_h_pad       -- extra horizontal padding applied to the title, header,
                       and footer rows (useful when outer_margins strips the
                       card's default horizontal padding for edge-to-edge body
                       content, but title/footer still need inset)
    widget          -- if provided, added to the body layout automatically
    dashed_border   -- when True, sets dashed_border="true" and drag_active="false"
                       properties so the card renders with a dashed border and
                       responds to drag-over state via CSS
    """
    card = AnimatedDashedCard() if dashed_border else QFrame()
    if not dashed_border:
        card.setObjectName("section_card")
    outer = QVBoxLayout(card)
    outer.setContentsMargins(*(outer_margins if outer_margins is not None else (14, 6, 14, 7)))
    outer.setSpacing(4)

    if title:
        lbl = QLabel(title)
        lbl.setProperty("role", "section")
        if title_buttons:
            title_row = QHBoxLayout()
            title_row.setContentsMargins(row_h_pad, 0, row_h_pad, 0)
            title_row.setSpacing(6)
            title_row.addWidget(lbl)
            title_row.addStretch()
            for btn in title_buttons:
                title_row.addWidget(btn)
            outer.addLayout(title_row)
        else:
            lbl.setContentsMargins(row_h_pad, 0, row_h_pad, 0)
            outer.addWidget(lbl)

    if header_widgets:
        header_row = QHBoxLayout()
        header_row.setContentsMargins(row_h_pad, 0, row_h_pad, 0)
        header_row.setSpacing(6)
        for w in header_widgets:
            header_row.addWidget(w)
        outer.addLayout(header_row)

    body = QVBoxLayout()
    body.setContentsMargins(0, 0, 0, 0)
    body.setSpacing(4)
    outer.addLayout(body, 1)

    if widget is not None:
        body.addWidget(widget)

    if footer_widgets:
        footer_row = QHBoxLayout()
        footer_row.setContentsMargins(row_h_pad, 0, row_h_pad, 0)
        footer_row.setSpacing(6)
        for w in footer_widgets:
            footer_row.addWidget(w)
        outer.addLayout(footer_row)

    return card, body
