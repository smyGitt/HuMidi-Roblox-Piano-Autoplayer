from PySide6.QtWidgets import QSlider, QDoubleSpinBox, QSpinBox, QComboBox, QAbstractSpinBox
from PySide6.QtCore import Qt


class NoScrollSlider(QSlider):
    """Horizontal QSlider whose value cannot be changed with the mouse wheel.

    Scrolling over a slider while panning a long settings page is an easy way to
    nudge a value by accident, so wheel events are swallowed (ignored, not
    consumed) and bubble up to the enclosing scroll area instead.
    """

    def wheelEvent(self, event):
        event.ignore()


class NoScrollSpinBox(QSpinBox):
    """QSpinBox whose value cannot be changed with the mouse wheel."""

    def wheelEvent(self, event):
        event.ignore()


class NoScrollDoubleSpinBox(QDoubleSpinBox):
    """QDoubleSpinBox whose value cannot be changed with the mouse wheel."""

    def wheelEvent(self, event):
        event.ignore()


class NoScrollComboBox(QComboBox):
    """QComboBox whose selection cannot be changed with the mouse wheel."""

    def wheelEvent(self, event):
        event.ignore()


def make_slider_spinbox(min_val, max_val, default_val,
                        text_suffix="", factor=10000.0, decimals=4):
    """Build a NoScrollSlider bound two-way to a buttonless NoScrollDoubleSpinBox value preview.

    The spinbox stores the real (unscaled) value; the slider works in integer
    units of ``value * factor``. Step arrows are removed (NoButtons) because the
    spinbox is a precise readout of the slider, not an independent stepper.
    """
    slider = NoScrollSlider(Qt.Orientation.Horizontal)
    slider.setRange(int(min_val * factor), int(max_val * factor))
    spinbox = NoScrollDoubleSpinBox()
    spinbox.setButtonSymbols(QAbstractSpinBox.ButtonSymbols.NoButtons)
    spinbox.setDecimals(decimals)
    spinbox.setRange(-2147483648, 2147483647)
    spinbox.setSingleStep(1.0 / factor)
    spinbox.setSuffix(text_suffix)
    slider.setValue(int(default_val * factor))
    spinbox.setValue(default_val)

    # Slider drives spinbox live, but only when the user is not actively typing
    # in the spinbox (focus guard prevents rewriting text mid-edit).
    slider.valueChanged.connect(
        lambda v: spinbox.setValue(v / factor) if not spinbox.hasFocus() else None
    )

    # Spinbox drives slider only once editing is committed (Enter or focus-out).
    # After clamping through the slider, the spinbox is also updated so it shows
    # the clamped value rather than the raw typed string.
    def _commit():
        slider.setValue(int(spinbox.value() * factor))
        spinbox.setValue(slider.value() / factor)

    spinbox.editingFinished.connect(_commit)
    return slider, spinbox
