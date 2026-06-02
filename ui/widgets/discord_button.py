import webbrowser

from PyQt6.QtWidgets import QFrame, QHBoxLayout, QLabel
from PyQt6.QtCore import Qt, QByteArray
from PyQt6.QtGui import QColor, QFont, QPainter, QPixmap

try:
    from PyQt6.QtSvg import QSvgRenderer as _QSvgRenderer
    _HAS_SVG = True
except ImportError:
    _HAS_SVG = False

# Full Discord "Clyde" logo SVG (viewBox 0 0 127.14 96.36).
# The closing " of the d attribute and /> are split across the last two
# string literals so Python concatenation produces valid XML without
# embedding a literal quote character inside the path data itself.
_DISCORD_SVG_TMPL = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 127.14 96.36">'
    '<path fill="{color}" d="'
    "M107.7,8.07A105.15,105.15,0,0,0,81.47,0a72.06,72.06,0,0,0-3.36,6.83"
    "A97.68,97.68,0,0,0,49,6.83,72.37,72.37,0,0,0,45.64,0,105.89,105.89,0,0,0,19.39,8.09"
    "C2.79,32.65-1.71,56.6.54,80.21h0A105.73,105.73,0,0,0,32.71,96.36,77.7,77.7,0,0,0,39.6,85.25"
    "a68.42,68.42,0,0,1-10.85-5.18c.91-.66,1.8-1.34,2.66-2a75.57,75.57,0,0,0,64.32,0"
    "c.87.71,1.76,1.39,2.66,2a68.68,68.68,0,0,1-10.87,5.19,77,77,0,0,0,6.89,11.1"
    "A105.25,105.25,0,0,0,126.6,80.22h0C129.24,52.84,122.09,29.11,107.7,8.07Z"
    "M42.45,65.69C36.18,65.69,31,60,31,53s5-12.74,11.43-12.74S54,46,53.89,53,48.84,65.69,42.45,65.69Z"
    "m42.24,0C78.41,65.69,73.25,60,73.25,53s5-12.74,11.44-12.74S96.23,46,96.12,53,91.08,65.69,84.69,65.69Z"
    '"/></svg>'
)

# Segoe MDL2 "People" glyph used as fallback when QtSvg is unavailable.
_DISCORD_FALLBACK_GLYPH = ""


def _render_discord_svg(hex_color: str, width: int, height: int) -> QPixmap:
    svg_bytes = QByteArray(_DISCORD_SVG_TMPL.format(color=hex_color).encode())
    renderer = _QSvgRenderer(svg_bytes)
    pix = QPixmap(width, height)
    pix.fill(Qt.GlobalColor.transparent)
    p = QPainter(pix)
    renderer.render(p)
    p.end()
    return pix


def _render_discord_fallback(hex_color: str, pixel_size: int) -> QPixmap:
    pix = QPixmap(pixel_size, pixel_size)
    pix.fill(Qt.GlobalColor.transparent)
    p = QPainter(pix)
    f = QFont("Segoe MDL2 Assets")
    f.setPixelSize(pixel_size)
    p.setFont(f)
    p.setPen(QColor(hex_color))
    p.drawText(pix.rect(), Qt.AlignmentFlag.AlignCenter, _DISCORD_FALLBACK_GLYPH)
    p.end()
    return pix


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
        self._icon_lbl.setFixedWidth(26)
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
        if _HAS_SVG:
            pix = _render_discord_svg(color, 22, 16)
        else:
            pix = _render_discord_fallback(color, 20)
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
