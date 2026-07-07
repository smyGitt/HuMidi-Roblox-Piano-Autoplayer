import os
import sys

from PySide6.QtCore import QByteArray, Qt
from PySide6.QtGui import QIcon, QPainter, QPixmap
from PySide6.QtSvg import QSvgRenderer


def _assets_dir() -> str:
    base = getattr(sys, "_MEIPASS", os.path.join(os.path.dirname(__file__), "..", ".."))
    return os.path.join(base, "assets", "icons", "duotone")


def ph_icon(name: str, color: str, size: int = 20) -> QIcon:
    """Render a Phosphor Duotone SVG icon with the given hex color.

    Loads `{name}-duotone.svg` from assets/icons/duotone/. All icons use
    fill="currentColor"; the opacity="0.2" path produces the duotone background
    layer automatically after color substitution.
    Renders at 2x internally (size*2 physical pixels, no DPR metadata) so
    callers that extract via .pixmap(size*2, size*2) and display via a
    setScaledContents label get a crisp downsample on all screen densities.
    """
    svg_path = os.path.join(_assets_dir(), f"{name}-duotone.svg")
    with open(svg_path, "rb") as fh:
        svg_bytes = fh.read().replace(b"currentColor", color.encode())
    renderer = QSvgRenderer(QByteArray(svg_bytes))
    phys = size * 2
    pix = QPixmap(phys, phys)
    pix.fill(Qt.GlobalColor.transparent)
    painter = QPainter(pix)
    painter.setRenderHint(QPainter.RenderHint.Antialiasing)
    painter.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
    renderer.render(painter)
    painter.end()
    icon = QIcon()
    icon.addPixmap(pix)
    return icon
