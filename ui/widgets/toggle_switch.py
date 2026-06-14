import weakref

from PyQt6.QtWidgets import QAbstractButton, QSizePolicy
from PyQt6.QtCore import Qt, QPropertyAnimation, QEasingCurve, pyqtProperty, pyqtSignal, QRectF, QSize
from PyQt6.QtGui import QPainter, QColor


def _parse_hex(h: str) -> tuple[int, int, int]:
    h = h.lstrip("#")
    return int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16)


class ToggleSwitch(QAbstractButton):
    """Animated toggle switch that replaces QCheckBox throughout the UI.

    Draws a sliding knob over a color-interpolating track using QPainter.
    The track fades from track_off to track_on as the knob slides, driven by
    a single QPropertyAnimation on _anim_pos (0.0=off, 1.0=on).

    Auto-registers with ToggleSwitchProvider on construction so theme color
    pushes reach every live instance without manual registration calls.

    API is compatible with QCheckBox: isChecked(), setChecked(), toggled(bool),
    stateChanged(int), text(), setText(), setEnabled(), blockSignals().
    """

    stateChanged = pyqtSignal(int)

    _TRACK_H = 16
    _KNOB_MARGIN = 2
    _LABEL_SPACING = 8

    def __init__(self, text: str = "", parent=None) -> None:
        super().__init__(text=text, parent=parent)
        self.setCheckable(True)
        self.setSizePolicy(QSizePolicy.Policy.Fixed, QSizePolicy.Policy.Fixed)

        self._pos: float = 0.0

        self._track_off = "#7878a0"
        self._track_on  = "#5b8dee"
        self._knob      = "#e8e8f0"
        self._text_col  = "#dcdcf0"
        self._dis_text  = "#555566"
        self._dis_track = "#32324a"

        self._anim = QPropertyAnimation(self, b"_anim_pos", self)
        self._anim.setDuration(180)
        self._anim.setEasingCurve(QEasingCurve.Type.InOutQuad)

        self.toggled.connect(self._on_toggled)
        self.toggled.connect(lambda checked: self.stateChanged.emit(2 if checked else 0))

        ToggleSwitchProvider.instance().register(self)

    @pyqtProperty(float)
    def _anim_pos(self) -> float:
        return self._pos

    @_anim_pos.setter
    def _anim_pos(self, value: float) -> None:
        self._pos = value
        self.update()

    def setChecked(self, checked: bool) -> None:
        # Snap pos before super() so the toggled handler skips the animation.
        self._pos = 1.0 if checked else 0.0
        super().setChecked(checked)

    def _on_toggled(self, checked: bool) -> None:
        target = 1.0 if checked else 0.0
        if abs(self._pos - target) < 0.01:
            return
        self._anim.stop()
        self._anim.setStartValue(self._pos)
        self._anim.setEndValue(target)
        self._anim.start()

    def update_colors(
        self,
        track_off: str,
        track_on: str,
        knob: str,
        text_col: str,
        dis_text: str,
        dis_track: str,
    ) -> None:
        self._track_off = track_off
        self._track_on  = track_on
        self._knob      = knob
        self._text_col  = text_col
        self._dis_text  = dis_text
        self._dis_track = dis_track
        self.update()

    def _track_w(self) -> int:
        return int(self._TRACK_H * 1.75)

    def sizeHint(self) -> QSize:
        tw = self._track_w()
        th = self._TRACK_H
        label = self.text()
        if label:
            fm = self.fontMetrics()
            w = tw + self._LABEL_SPACING + fm.horizontalAdvance(label) + 2
            h = max(th, fm.height()) + 2
            return QSize(w, h)
        return QSize(tw + 2, th + 2)

    def paintEvent(self, event) -> None:
        p = QPainter(self)
        p.setRenderHint(QPainter.RenderHint.Antialiasing)

        tw = self._track_w()
        th = self._TRACK_H
        kd = th - 2 * self._KNOB_MARGIN
        enabled = self.isEnabled()
        t = self._pos

        if not enabled:
            track_color = QColor(self._dis_track)
        else:
            r1, g1, b1 = _parse_hex(self._track_off)
            r2, g2, b2 = _parse_hex(self._track_on)
            track_color = QColor(
                int(r1 + (r2 - r1) * t),
                int(g1 + (g2 - g1) * t),
                int(b1 + (b2 - b1) * t),
            )

        knob_color = QColor(self._knob)
        if not enabled:
            knob_color.setAlphaF(0.45)

        cy = (self.height() - th) // 2

        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(track_color)
        p.drawRoundedRect(QRectF(0, cy, tw, th), th / 2, th / 2)

        travel = tw - kd - 2 * self._KNOB_MARGIN
        kx = self._KNOB_MARGIN + t * travel
        ky = cy + self._KNOB_MARGIN
        p.setBrush(knob_color)
        p.drawEllipse(QRectF(kx, ky, kd, kd))

        label = self.text()
        if label:
            p.setPen(QColor(self._dis_text if not enabled else self._text_col))
            lx = tw + self._LABEL_SPACING
            p.drawText(
                QRectF(lx, 0, self.width() - lx, self.height()),
                Qt.AlignmentFlag.AlignVCenter | Qt.AlignmentFlag.AlignLeft,
                label,
            )

        p.end()


class ToggleSwitchProvider:
    """Singleton registry that pushes theme colors to all live ToggleSwitch instances."""

    _instance: "ToggleSwitchProvider | None" = None

    def __init__(self) -> None:
        self._switches: weakref.WeakSet[ToggleSwitch] = weakref.WeakSet()

    @classmethod
    def instance(cls) -> "ToggleSwitchProvider":
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    def register(self, switch: ToggleSwitch) -> None:
        self._switches.add(switch)

    def notify_theme_changed(
        self,
        track_off: str,
        track_on: str,
        knob: str,
        text_col: str,
        dis_text: str,
        dis_track: str,
    ) -> None:
        for sw in list(self._switches):
            sw.update_colors(track_off, track_on, knob, text_col, dis_text, dis_track)
