from PySide6.QtCore import Qt, QTimer, QByteArray, Property
from PySide6.QtGui import QBrush, QColor, QPainter, QPixmap, QTransform
from PySide6.QtSvg import QSvgRenderer
from PySide6.QtWidgets import QFrame, QLabel

# Hourglass animation frames (Phosphor-style duotone SVGs, 256x256 viewBox).
# Frame 0: sand full in top half (top polygon shaded, top line).
# Frame 1: sand at midpoint (equal halves, line dropping from center).
# Frame 2: sand full in bottom half (bottom polygon shaded, bottom line).
# Frame 3 is generated at runtime as frame 2 rotated 90 degrees, producing
# the "flip" transition back to frame 0.
_SVG_0 = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    '<rect width="256" height="256" fill="none"/>'
    '<polygon points="69.52 72 186.48 72 128 128 69.52 72" opacity="0.2" fill="currentColor"/>'
    '<path d="M50.36,53.66A8,8,0,0,1,56,40H200a8,8,0,0,1,5.66,13.66L128,128Z"'
    ' fill="none" stroke="currentColor" stroke-linecap="round"'
    ' stroke-linejoin="round" stroke-width="16"/>'
    '<path d="M50.36,202.34A8,8,0,0,0,56,216H200a8,8,0,0,0,5.66-13.66L128,128Z"'
    ' fill="none" stroke="currentColor" stroke-linecap="round"'
    ' stroke-linejoin="round" stroke-width="16"/>'
    '<line x1="69.52" y1="72" x2="186.48" y2="72" fill="none" stroke="currentColor"'
    ' stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    '</svg>'
)
_SVG_1 = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    '<rect width="256" height="256" fill="none"/>'
    '<polygon points="77.87 80 178.13 80 128 128 77.87 80" opacity="0.2" fill="currentColor"/>'
    '<path d="M50.36,53.66A8,8,0,0,1,56,40H200a8,8,0,0,1,5.66,13.66L128,128Z"'
    ' fill="none" stroke="currentColor" stroke-linecap="round"'
    ' stroke-linejoin="round" stroke-width="16"/>'
    '<path d="M50.36,202.34A8,8,0,0,0,56,216H200a8,8,0,0,0,5.66-13.66L128,128Z"'
    ' fill="none" stroke="currentColor" stroke-linecap="round"'
    ' stroke-linejoin="round" stroke-width="16"/>'
    '<line x1="128" y1="128" x2="128" y2="168" fill="none" stroke="currentColor"'
    ' stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    '<line x1="178.13" y1="80" x2="77.87" y2="80" fill="none" stroke="currentColor"'
    ' stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    '</svg>'
)
_SVG_2 = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">'
    '<rect width="256" height="256" fill="none"/>'
    '<path d="M50.36,202.34A8,8,0,0,0,56,216H200a8,8,0,0,0,5.66-13.66L178.13,176H77.87Z"'
    ' opacity="0.2" fill="currentColor"/>'
    '<path d="M50.36,53.66A8,8,0,0,1,56,40H200a8,8,0,0,1,5.66,13.66L128,128Z"'
    ' fill="none" stroke="currentColor" stroke-linecap="round"'
    ' stroke-linejoin="round" stroke-width="16"/>'
    '<path d="M50.36,202.34A8,8,0,0,0,56,216H200a8,8,0,0,0,5.66-13.66L128,128Z"'
    ' fill="none" stroke="currentColor" stroke-linecap="round"'
    ' stroke-linejoin="round" stroke-width="16"/>'
    '<line x1="178.13" y1="176" x2="77.87" y2="176" fill="none" stroke="currentColor"'
    ' stroke-linecap="round" stroke-linejoin="round" stroke-width="16"/>'
    '</svg>'
)

_ANIM_SVGS = (_SVG_0, _SVG_1, _SVG_2)


def _render_svg(svg_str: str, color: str) -> QPixmap:
    """Render an SVG string with currentColor replaced, at 2x (44x44) for HiDPI."""
    data = svg_str.encode().replace(b"currentColor", color.encode())
    renderer = QSvgRenderer(QByteArray(data))
    pix = QPixmap(44, 44)
    pix.fill(Qt.GlobalColor.transparent)
    p = QPainter(pix)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
    renderer.render(p)
    p.end()
    return pix


def _dot_pixmap(hex_color: str) -> QPixmap:
    """Render a filled circle at 2x (44x44) for HiDPI."""
    pix = QPixmap(44, 44)
    pix.fill(Qt.GlobalColor.transparent)
    p = QPainter(pix)
    p.setRenderHint(QPainter.RenderHint.Antialiasing)
    p.setBrush(QBrush(QColor(hex_color)))
    p.setPen(Qt.PenStyle.NoPen)
    # 28px diameter circle centered in 44px canvas
    p.drawEllipse(8, 8, 28, 28)
    p.end()
    return pix


class StatusIndicator(QFrame):
    """Sidebar status indicator: icon slot + text label, same geometry as NavButton.

    States:
      UNLOADED -- red dot, shown when no file is loaded and playback is unavailable.
      LOADING  -- animated hourglass cycling through 4 frames (3 SVG frames plus
                  the last frame rotated 90 degrees to create a flip effect).
      READY    -- themed green dot, shown when a file is compiled and ready to play.

    Colors are supplied by QSS via qproperty-* (iconColor, readyColor,
    unloadColor, loadedColor); the status label color is styled via QSS.
    The icon slot is 22x22 at x=12 and the text label spans x=44 to x=244,
    matching NavButton geometry exactly so sidebar clipping is the only visibility
    mechanism needed.
    """

    UNLOADED = "unloaded"
    LOADING  = "loading"
    LOADED   = "loaded"
    READY    = "ready"

    _ANIM_MS = 350

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setObjectName("status_indicator")
        self.setFixedHeight(48)

        self._state        = self.UNLOADED
        # Color slots (overwritten by QSS qproperty-* on stylesheet apply).
        self._icon_color   = QColor("#888888")
        self._ready_color  = QColor("#52b752")
        self._unload_color = QColor("#c44b4b")
        self._loaded_color = QColor("#c9a535")

        self._frames: list[QPixmap] = []
        self._frame_idx = 0

        self._timer = QTimer(self)
        self._timer.setInterval(self._ANIM_MS)
        self._timer.timeout.connect(self._advance)

        self._icon_lbl = QLabel(self)
        self._icon_lbl.setObjectName("status_icon")
        self._icon_lbl.setGeometry(12, 13, 22, 22)
        self._icon_lbl.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._icon_lbl.setScaledContents(True)
        self._icon_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

        self._text_lbl = QLabel("NO FILE", self)
        self._text_lbl.setObjectName("status_label")
        self._text_lbl.setGeometry(44, 0, 200, 48)
        self._text_lbl.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignVCenter)
        self._text_lbl.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)

        self._icon_lbl.setPixmap(_dot_pixmap(self._unload_color.name()))

    # -- Public API -----------------------------------------------------------

    @property
    def state(self) -> str:
        return self._state

    # -- QSS-driven color slots ----------------------------------------------

    @Property(QColor)
    def iconColor(self) -> QColor:
        return self._icon_color

    @iconColor.setter
    def iconColor(self, c: QColor) -> None:
        self._icon_color = c
        self._apply_visuals()

    @Property(QColor)
    def readyColor(self) -> QColor:
        return self._ready_color

    @readyColor.setter
    def readyColor(self, c: QColor) -> None:
        self._ready_color = c
        self._apply_visuals()

    @Property(QColor)
    def unloadColor(self) -> QColor:
        return self._unload_color

    @unloadColor.setter
    def unloadColor(self, c: QColor) -> None:
        self._unload_color = c
        self._apply_visuals()

    @Property(QColor)
    def loadedColor(self) -> QColor:
        return self._loaded_color

    @loadedColor.setter
    def loadedColor(self, c: QColor) -> None:
        self._loaded_color = c
        self._apply_visuals()

    def _apply_visuals(self) -> None:
        """Rebuild animation frames and re-render the current state dot/icon."""
        self._frames = self._build_frames(self._icon_color.name())
        if self._state == self.LOADING and self._frames:
            self._icon_lbl.setPixmap(self._frames[self._frame_idx])
        elif self._state == self.READY:
            self._icon_lbl.setPixmap(_dot_pixmap(self._ready_color.name()))
        elif self._state == self.LOADED:
            self._icon_lbl.setPixmap(_dot_pixmap(self._loaded_color.name()))
        else:
            self._icon_lbl.setPixmap(_dot_pixmap(self._unload_color.name()))

    def set_state(self, state: str, text: str | None = None) -> None:
        """Transition to state and optionally update the text label."""
        self._state = state
        if text is not None:
            self._text_lbl.setText(text)
        if state == self.LOADING:
            if self._frames:
                self._frame_idx = 0
                self._icon_lbl.setPixmap(self._frames[0])
                self._timer.start()
            else:
                self._icon_lbl.setPixmap(_dot_pixmap(self._unload_color.name()))
        elif state == self.READY:
            self._timer.stop()
            self._icon_lbl.setPixmap(_dot_pixmap(self._ready_color.name()))
        elif state == self.LOADED:
            self._timer.stop()
            self._icon_lbl.setPixmap(_dot_pixmap(self._loaded_color.name()))
        else:  # UNLOADED
            self._timer.stop()
            self._icon_lbl.setPixmap(_dot_pixmap(self._unload_color.name()))

    def set_text(self, text: str) -> None:
        """Update the label text without changing state."""
        self._text_lbl.setText(text)

    # -- Internal -------------------------------------------------------------

    def _advance(self) -> None:
        if not self._frames:
            return
        self._frame_idx = (self._frame_idx + 1) % len(self._frames)
        self._icon_lbl.setPixmap(self._frames[self._frame_idx])

    @staticmethod
    def _build_frames(color: str) -> list[QPixmap]:
        """Return 4 animation frames: SVG 0, 1, 2, then SVG 2 rotated 90 degrees."""
        frames = [_render_svg(svg, color) for svg in _ANIM_SVGS]
        # Rotate the last frame 90 degrees clockwise to create the flip-back effect.
        rotated = frames[-1].transformed(
            QTransform().rotate(90),
            Qt.TransformationMode.SmoothTransformation,
        )
        # QPixmap.transformed() may produce a different size for non-square; ensure 44x44.
        if rotated.size().width() != 44 or rotated.size().height() != 44:
            rotated = rotated.scaled(
                44, 44,
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation,
            )
        frames.append(rotated)
        return frames
