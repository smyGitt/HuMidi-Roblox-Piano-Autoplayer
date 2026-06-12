"""Pure compilation pipeline: humanize notes then build a sorted KeyEvent list.

This module has no Qt dependency and drives no hardware. It can be called from
any worker thread or test without a running QApplication.
"""

import copy
import heapq
import random
from typing import Callable, Dict, List, Optional

from core.config import PlaybackConfig
from core.models import Note, KeyEvent, MusicalSection, KeyState
from core.core import KeyMapper
from core.humanizer import Humanizer
import core.pedal_generator as pedal_generator


def compile_events(
    config: PlaybackConfig,
    notes: List[Note],
    sections: List[MusicalSection],
    log: Optional[Callable[[str], None]] = None,
) -> List[KeyEvent]:
    """Run humanization + event compilation for a set of notes.

    Returns a sorted KeyEvent list ready for playback or serialization.
    Does not drive hardware, manage threads, or emit Qt signals.

    Parameters
    ----------
    config:   Full playback config dict.
    notes:    Notes with hand assignments already applied.
    sections: MusicalSection list from SectionAnalyzer.
    log:      Optional callable for debug logging (e.g. signal.emit).
    """
    def _log(msg: str) -> None:
        if log:
            log(msg)

    _log("\n=== HUMANIZATION PIPELINE ===")

    debug_list: Optional[List[str]] = [] if config.get('debug_mode') else None
    humanizer = Humanizer(config, debug_list)
    humanized_notes = copy.deepcopy(notes)
    left_hand_notes  = [n for n in humanized_notes if n.hand == 'left']
    right_hand_notes = [n for n in humanized_notes if n.hand == 'right']
    unknown_notes    = [n for n in humanized_notes if n.hand == 'unknown']

    _log(
        f"[PIPELINE] Input: {len(humanized_notes)} notes total | "
        f"L={len(left_hand_notes)} R={len(right_hand_notes)} Unknown={len(unknown_notes)}"
    )
    resync_points = (
        {round(n.start_time, 2) for n in left_hand_notes}
        .intersection({round(n.start_time, 2) for n in right_hand_notes})
    )
    _log(f"[PIPELINE] Resync points (both hands simultaneous): {len(resync_points)}")

    humanizer.apply_to_hand(left_hand_notes, 'left', resync_points)
    humanizer.apply_to_hand(right_hand_notes, 'right', resync_points)
    all_notes = sorted(left_hand_notes + right_hand_notes, key=lambda n: n.start_time)
    humanizer.apply_tempo_rubato(all_notes, sections)

    if debug_list and log:
        for msg in debug_list:
            log(msg)

    _log("\n=== COMPILATION ===")

    mapper = KeyMapper(use_88_key_layout=config.get('use_88_key_layout', False))
    use_mistakes   = config.get('enable_mistakes', False)
    mistake_chance = config.get('mistake_chance', 0) / 100.0
    temp_heap: List[KeyEvent] = []
    mistakes_injected = 0
    notes_unmapped    = 0

    _log(
        f"[COMPILE] Notes to compile: {len(all_notes)} | Mistakes: {'ON' if use_mistakes else 'OFF'}"
        + (f" ({mistake_chance * 100:.1f}%)" if use_mistakes else "")
    )

    for note in all_notes:
        scheduled = False
        if use_mistakes and random.random() < mistake_chance:
            mistake_pitch = _get_mistake_pitch(note.pitch)
            if mistake_pitch:
                key_data = mapper.get_key_data(mistake_pitch)
                if key_data:
                    mk_char = key_data['key']
                    heapq.heappush(temp_heap, KeyEvent(note.start_time, 2, 'press', mk_char, pitch=mistake_pitch))
                    heapq.heappush(temp_heap, KeyEvent(note.start_time + note.duration, 4, 'release', mk_char, pitch=mistake_pitch))
                    scheduled = True
                    mistakes_injected += 1

        if not scheduled:
            key_data = mapper.get_key_data(note.pitch)
            if key_data:
                key_char = key_data['key']
                heapq.heappush(temp_heap, KeyEvent(note.start_time, 2, 'press', key_char, pitch=note.pitch))
                heapq.heappush(temp_heap, KeyEvent(note.end_time,   4, 'release', key_char, pitch=note.pitch))
            else:
                notes_unmapped += 1

    _log(f"[COMPILE] Pedal style: {config.get('pedal_style', 'none')}")
    for event in pedal_generator.generate_events(config, all_notes, sections, log):
        heapq.heappush(temp_heap, event)

    compiled: List[KeyEvent] = []
    while temp_heap:
        compiled.append(heapq.heappop(temp_heap))

    total_duration = compiled[-1].time if compiled else 0.0

    press_count   = sum(1 for e in compiled if e.action == 'press')
    release_count = sum(1 for e in compiled if e.action == 'release')
    pedal_down    = sum(1 for e in compiled if e.action == 'pedal' and e.key_char == 'down')
    pedal_up      = sum(1 for e in compiled if e.action == 'pedal' and e.key_char == 'up')
    pitches       = [e.pitch for e in compiled if e.pitch is not None]
    pitch_range   = (
        f"{KeyMapper.pitch_to_name(min(pitches))}-{KeyMapper.pitch_to_name(max(pitches))}"
        if pitches else "none"
    )
    unique_keys = len({e.key_char for e in compiled if e.action in ('press', 'release')})

    _log(
        f"[COMPILE] Result: {len(compiled)} events | "
        f"press={press_count} release={release_count} pedal_down={pedal_down} pedal_up={pedal_up}"
    )
    _log(
        f"[COMPILE] Duration: {total_duration:.2f}s | Pitch range: {pitch_range} | "
        f"Unique keys: {unique_keys} | Mistakes: {mistakes_injected} | Unmapped: {notes_unmapped}"
    )
    if total_duration > 0:
        _log(
            f"[COMPILE] Density: {press_count / total_duration:.1f} presses/sec | "
            f"{pedal_down / total_duration:.2f} pedal-downs/sec"
        )

    return compiled


def _get_mistake_pitch(original_pitch: int) -> Optional[int]:
    candidates = [original_pitch + d for d in (-2, -1, 1, 2)]
    if KeyMapper.is_black_key(original_pitch):
        black_pool = [p for p in candidates if KeyMapper.is_black_key(p)]
        white_pool = [p for p in candidates if not KeyMapper.is_black_key(p)]
        pool = (black_pool if random.random() < 0.5 else white_pool) or black_pool or white_pool
        return random.choice(pool) if pool else None
    valid = [p for p in candidates if not KeyMapper.is_black_key(p)]
    return random.choice(valid) if valid else None
