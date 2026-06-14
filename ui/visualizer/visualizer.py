from PyQt6.QtWidgets import QWidget, QSizePolicy
from PyQt6.QtCore import Qt, QRectF, QPointF, pyqtSignal as Signal
from PyQt6.QtGui import QPainter, QBrush, QColor, QPen, QPixmap
from typing import List
from core.models import Note
from core.core import TempoMap

class PianoWidget(QWidget):
    def __init__(self, parent=None):
        super().__init__(parent)
        self.setFixedHeight(92)
        self.setMinimumWidth(500)
        self.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Fixed)
        self.active_pitches = set()
        self.min_pitch = 21
        self.max_pitch = 108
        self.white_keys_count = 52
        self.black_keys = {1, 3, 6, 8, 10}
        self.pedal_active = False
        self.show_pedal = True
        self.pedal_color = QColor(232, 160, 32)   # amber

    def set_active_pitches(self, pitches: list):
        self.active_pitches = set(pitches)
        self.update()

    def clear(self):
        self.active_pitches.clear()
        self.update()

    def set_pedal_active(self, active: bool):
        if self.pedal_active != active:
            self.pedal_active = active
            self.update()

    def set_show_pedal(self, visible: bool):
        if self.show_pedal != visible:
            self.show_pedal = visible
            self.setFixedHeight(92 if visible else 80)
            self.update()

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        width = self.width()
        height = self.height()
        # Key area excludes pedal strip at bottom
        key_area_height = (height - 12) if self.show_pedal else height
        key_width = width / self.white_keys_count
        black_key_width = key_width * 0.65
        black_key_height = key_area_height * 0.6

        white_brush = QBrush(QColor(230, 230, 240))
        black_brush = QBrush(QColor(28, 28, 46))
        active_brush = QBrush(QColor(78, 203, 141))   # theme accent green

        white_idx = 0
        white_key_rects = {}

        for p in range(self.min_pitch, self.max_pitch + 1):
            if (p % 12) in self.black_keys: continue
            x = white_idx * key_width
            rect = QRectF(x, 0, key_width, key_area_height)
            white_key_rects[p] = rect

            brush = active_brush if p in self.active_pitches else white_brush
            painter.setBrush(brush)
            painter.setPen(QPen(QColor(50, 50, 70), 1))
            painter.drawRect(rect)
            white_idx += 1

        for p in range(self.min_pitch, self.max_pitch + 1):
            if (p % 12) not in self.black_keys: continue
            prev_white = p - 1
            if prev_white not in white_key_rects: continue

            ref_rect = white_key_rects[prev_white]
            x = ref_rect.right() - (black_key_width / 2)
            rect = QRectF(x, 0, black_key_width, black_key_height)

            brush = active_brush if p in self.active_pitches else black_brush
            painter.setBrush(brush)
            painter.setPen(QPen(QColor(15, 15, 30), 1))
            painter.drawRect(rect)

        if self.show_pedal:
            strip_rect = QRectF(0, height - 12, width, 12)
            if self.pedal_active:
                painter.fillRect(strip_rect, self.pedal_color)
            else:
                dim = QColor(self.pedal_color)
                dim.setAlpha(40)
                painter.fillRect(strip_rect, dim)


class TimelineWidget(QWidget):
    seek_requested = Signal(float)
    scrub_position_changed = Signal(float)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setSizePolicy(QSizePolicy.Policy.Fixed, QSizePolicy.Policy.Expanding)
        self.notes = []
        self.total_duration = 1.0
        self.current_time = 0.0
        self.is_dragging = False
        self.pixels_per_second = 50
        self.tempo_map = None
        self.pedal_intervals = []        # List of (start_sec, end_sec) tuples
        self.show_pedal = True

        self.cached_background = None

        self.bg_color = QColor(24, 24, 40)
        self.left_hand_color = QColor(91, 141, 238, 210)   # theme accent blue
        self.right_hand_color = QColor(78, 203, 141, 210)  # theme accent green
        self.unknown_color = QColor(100, 100, 140, 160)
        self.cursor_color = QColor(220, 220, 240)
        self.measure_line_color = QColor(255, 255, 255, 30)
        self.pedal_color = QColor(232, 160, 32, 180)       # amber, semi-transparent

    def set_data(self, notes: List[Note], duration: float, tempo_map: TempoMap = None):
        self.notes = notes
        self.total_duration = max(duration, 0.1)
        self.tempo_map = tempo_map

        new_width = int(self.total_duration * self.pixels_per_second)
        new_width = max(new_width, 800)
        new_width = min(new_width, 16384)
        self.setFixedWidth(new_width)

        self.cached_background = None
        self.update()

    def set_pedal_intervals(self, intervals: list):
        self.pedal_intervals = intervals
        self.cached_background = None
        self.update()

    def set_show_pedal(self, visible: bool):
        if self.show_pedal != visible:
            self.show_pedal = visible
            self.cached_background = None
            self.update()

    def set_position(self, time: float):
        if not self.is_dragging:
            self.current_time = time
            self.update()

    def mousePressEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            self.is_dragging = True
            self._handle_mouse_input(event.position().x())

    def mouseMoveEvent(self, event):
        if self.is_dragging:
            self._handle_mouse_input(event.position().x())

    def mouseReleaseEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            self.is_dragging = False
            self.seek_requested.emit(self.current_time)

    def _handle_mouse_input(self, x):
        ratio = max(0.0, min(1.0, x / self.width()))
        self.current_time = ratio * self.total_duration
        self.scrub_position_changed.emit(self.current_time)
        self.update()

    def resizeEvent(self, event):
        self.cached_background = None
        super().resizeEvent(event)

    def paintEvent(self, event):
        w = self.width()
        h = self.height()

        if self.cached_background is None or self.cached_background.size() != self.size():
            self.cached_background = QPixmap(self.size())
            self.cached_background.fill(self.bg_color)

            cache_painter = QPainter(self.cached_background)
            cache_painter.setRenderHint(QPainter.RenderHint.Antialiasing)

            if self.tempo_map:
                cache_painter.setPen(QPen(self.measure_line_color, 1))
                try:
                    boundaries = self.tempo_map.get_measure_boundaries(self.total_duration)
                    for start_t, end_t in boundaries:
                        x = (start_t / self.total_duration) * w
                        cache_painter.drawLine(QPointF(x, 0), QPointF(x, h))
                except Exception: pass

            if self.notes:
                min_p = 21
                max_p = 108
                range_p = max_p - min_p

                # Reserve bottom 8px for pedal strip; top 5px margin
                note_area_h = h - 13   # 5px top margin + 8px pedal strip

                cache_painter.setPen(Qt.PenStyle.NoPen)

                for note in self.notes:
                    nx = (note.start_time / self.total_duration) * w
                    nw = (note.duration / self.total_duration) * w
                    nw = max(1.0, nw)

                    ny_ratio = 1.0 - ((note.pitch - min_p) / range_p)
                    ny = ny_ratio * note_area_h + 5
                    nh = 8

                    if note.hand == 'left':
                        cache_painter.setBrush(QBrush(self.left_hand_color))
                    elif note.hand == 'right':
                        cache_painter.setBrush(QBrush(self.right_hand_color))
                    else:
                        cache_painter.setBrush(QBrush(self.unknown_color))

                    cache_painter.drawRect(QRectF(nx, ny, nw, nh))

            # Pedal strip at the bottom
            if self.show_pedal and self.pedal_intervals:
                strip_h = 8
                y = h - strip_h
                cache_painter.setPen(Qt.PenStyle.NoPen)
                cache_painter.setBrush(QBrush(self.pedal_color))
                for start, end in self.pedal_intervals:
                    px = (start / self.total_duration) * w
                    pw = max(1.0, (end - start) / self.total_duration * w)
                    cache_painter.drawRect(QRectF(px, y, pw, strip_h))

            cache_painter.end()

        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        painter.drawPixmap(0, 0, self.cached_background)

        cx = (self.current_time / self.total_duration) * w
        painter.setPen(QPen(self.cursor_color, 2))
        painter.drawLine(QPointF(cx, 0), QPointF(cx, h))
