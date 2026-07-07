from PySide6.QtWidgets import QAbstractButton, QSizePolicy
from PySide6.QtCore import Qt, QPropertyAnimation, QEasingCurve, Property, Signal, QRectF, QSize
from PySide6.QtGui import QPainter, QColor


class ToggleSwitch(QAbstractButton):
    """Animated toggle switch that replaces QCheckBox throughout the UI.

    Draws a sliding knob over a color-interpolating track using QPainter.
    The track fades from trackOff to trackOn as the knob slides, driven by
    a single QPropertyAnimation on _anim_pos (0.0=off, 1.0=on).

    All colors are supplied by QSS via qproperty-* (trackOff, trackOn, knob,
    textColor, disText, disTrack); paintEvent only reads the stored QColors.

    API is compatible with QCheckBox: isChecked(), setChecked(), toggled(bool),
    stateChanged(int), text(), setText(), setEnabled(), blockSignals().
    """

    stateChanged = Signal(int)

    _TRACK_H = 16
    _KNOB_MARGIN = 2
    _LABEL_SPACING = 8

    def __init__(self, text: str = "", parent=None) -> None:
        super().__init__(text=text, parent=parent)
        self.setCheckable(True)
        self.setSizePolicy(QSizePolicy.Policy.Fixed, QSizePolicy.Policy.Fixed)

        self._pos: float = 0.0

        # Color slots (overwritten by QSS qproperty-* on stylesheet apply).
        self._track_off = QColor("#7878a0")
        self._track_on  = QColor("#5b8dee")
        self._knob      = QColor("#e8e8f0")
        self._text_col  = QColor("#dcdcf0")
        self._dis_text  = QColor("#555566")
        self._dis_track = QColor("#32324a")

        self._anim = QPropertyAnimation(self, b"_anim_pos", self)
        self._anim.setDuration(180)
        self._anim.setEasingCurve(QEasingCurve.Type.InOutQuad)

        self.toggled.connect(self._on_toggled)
        self.toggled.connect(lambda checked: self.stateChanged.emit(2 if checked else 0))

    @Property(float)
    def _anim_pos(self) -> float:
        return self._pos

    @_anim_pos.setter
    def _anim_pos(self, value: float) -> None:
        self._pos = value
        self.update()

    # -- QSS-driven color slots ----------------------------------------------

    @Property(QColor)
    def trackOff(self) -> QColor:
        return self._track_off

    @trackOff.setter
    def trackOff(self, c: QColor) -> None:
        self._track_off = c
        self.update()

    @Property(QColor)
    def trackOn(self) -> QColor:
        return self._track_on

    @trackOn.setter
    def trackOn(self, c: QColor) -> None:
        self._track_on = c
        self.update()

    @Property(QColor)
    def knob(self) -> QColor:
        return self._knob

    @knob.setter
    def knob(self, c: QColor) -> None:
        self._knob = c
        self.update()

    @Property(QColor)
    def textColor(self) -> QColor:
        return self._text_col

    @textColor.setter
    def textColor(self, c: QColor) -> None:
        self._text_col = c
        self.update()

    @Property(QColor)
    def disText(self) -> QColor:
        return self._dis_text

    @disText.setter
    def disText(self, c: QColor) -> None:
        self._dis_text = c
        self.update()

    @Property(QColor)
    def disTrack(self) -> QColor:
        return self._dis_track

    @disTrack.setter
    def disTrack(self, c: QColor) -> None:
        self._dis_track = c
        self.update()

    # -- State ----------------------------------------------------------------

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
            c1, c2 = self._track_off, self._track_on
            track_color = QColor(
                int(c1.red()   + (c2.red()   - c1.red())   * t),
                int(c1.green() + (c2.green() - c1.green()) * t),
                int(c1.blue()  + (c2.blue()  - c1.blue())  * t),
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
            p.setPen(self._dis_text if not enabled else self._text_col)
            lx = tw + self._LABEL_SPACING
            p.drawText(
                QRectF(lx, 0, self.width() - lx, self.height()),
                Qt.AlignmentFlag.AlignVCenter | Qt.AlignmentFlag.AlignLeft,
                label,
            )

        p.end()
