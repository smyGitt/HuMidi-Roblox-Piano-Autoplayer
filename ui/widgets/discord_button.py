import webbrowser

from PyQt6.QtWidgets import QFrame, QHBoxLayout, QLabel
from PyQt6.QtCore import Qt, QSize
from PyQt6.QtGui import QColor, QFont, QPainter, QPixmap

from ui.widgets.ph_icon import ph_icon


_ICON_SIZE = 22


class DiscordNavButton(QFrame):
    """Sidebar link button that opens a Discord URL in the browser on click."""

    def __init__(self, url: str, parent=None):
        super().__init__(parent)
        self.setObjectName("nav_btn")
        self._url = url
        self._color_dim = "#7878a0"
        self._color_hi  = "#dcdcf0"
        self._hovered = False

        self.setAttribute(Qt.WidgetAttribute.WA_StyledBackground, True)
        self.setCursor(Qt.CursorShape.PointingHandCursor)
        self.setFixedHeight(48)
        self.setProperty("active",  "false")
        self.setProperty("hovered", "false")

        hbox = QHBoxLayout(self)
        hbox.setContentsMargins(12, 11, 12, 11)
        hbox.setSpacing(10)

        self._icon_lbl = QLabel()
        self._icon_lbl.setFixedSize(QSize(_ICON_SIZE, _ICON_SIZE))
        self._icon_lbl.setScaledContents(True)
        self._icon_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._icon_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

        self._text_lbl = QLabel("Discord")
        self._text_lbl.setObjectName("nav_label")
        self._text_lbl.setAlignment(
            Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter
        )
        self._text_lbl.setProperty("highlighted", "false")
        self._text_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

        hbox.addWidget(self._icon_lbl)
        hbox.addWidget(self._text_lbl, 1)

        self._refresh_icon()

    def _refresh_icon(self) -> None:
        color = self._color_hi if self._hovered else self._color_dim
        pix = ph_icon("discord-logo", color, _ICON_SIZE).pixmap(_ICON_SIZE * 2, _ICON_SIZE * 2)
        self._icon_lbl.setPixmap(pix)

    def update_colors(self, dim: str, hi: str) -> None:
        self._color_dim = dim
        self._color_hi = hi
        self._refresh_icon()

    def mousePressEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            webbrowser.open(self._url)
        super().mousePressEvent(event)

    def enterEvent(self, event):
        self._hovered = True
        self.setProperty("hovered", "true")
        self._text_lbl.setProperty("highlighted", "true")
        self._text_lbl.style().unpolish(self._text_lbl)
        self._text_lbl.style().polish(self._text_lbl)
        self.style().unpolish(self)
        self.style().polish(self)
        self._refresh_icon()
        self.update()
        super().enterEvent(event)

    def leaveEvent(self, event):
        self._hovered = False
        self.setProperty("hovered", "false")
        self._text_lbl.setProperty("highlighted", "false")
        self._text_lbl.style().unpolish(self._text_lbl)
        self._text_lbl.style().polish(self._text_lbl)
        self.style().unpolish(self)
        self.style().polish(self)
        self._refresh_icon()
        self.update()
        super().leaveEvent(event)
