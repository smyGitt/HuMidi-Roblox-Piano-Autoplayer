"""Pure compilation pipeline: humanize notes then build a sorted KeyEvent list.

This module has no Qt dependency and drives no hardware. It can be called from
any worker thread or test without a running QApplication.

Two-phase API
-------------
compile_note_events  -- humanization only; returns (sorted note events, humanized notes)
compile_pedal_events -- pedal generation from humanized notes
merge_compiled       -- heap-merges note and pedal events into the final sorted list

compile_events       -- backwards-compatible one-shot wrapper that calls all three
"""

import copy
import heapq
import random
from typing import Callable, Dict, List, Optional, Tuple

from core.config import PlaybackConfig
from core.models import Note, KeyEvent, MusicalSection
from core.core import KeyMapper
from core.humanizer import Humanizer
import core.pedal_generator as pedal_generator


def compile_note_events(
    config: PlaybackConfig,
    notes: List[Note],
    sections: List[MusicalSection],
    log: Optional[Callable[[str], None]] = None,
) -> Tuple[List[KeyEvent], List[Note]]:
    """Run humanization and build note press/release KeyEvents.

    Returns (note_events, humanized_notes).

    note_events     -- sorted KeyEvent list containing only press/release events
                       (no pedal). Safe to store and re-use across pedal
                       regenerations because it is returned as a plain list, not
                       a live heap.
    humanized_notes -- post-Humanizer note list; must be passed to
                       compile_pedal_events so pedal generation operates on the
                       same timing that will actually be played.
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

    press_count = sum(1 for e in temp_heap if e.action == 'press')
    _log(
        f"[COMPILE] Note events: {len(temp_heap)} | "
        f"Mistakes: {mistakes_injected} | Unmapped: {notes_unmapped}"
    )

    note_events: List[KeyEvent] = []
    while temp_heap:
        note_events.append(heapq.heappop(temp_heap))

    return note_events, all_notes


def compile_midi_pedal_events(
    midi_pedal_events: List[Tuple[float, bool]],
) -> List[KeyEvent]:
    """Convert raw (time_sec, is_on) CC 64 pairs from the MIDI file into pedal KeyEvents.

    Priority matches the values used by all generated strategies: down=1, up=0.
    Events are returned in time order (input is assumed sorted; no extra sort needed).
    """
    result: List[KeyEvent] = []
    for time_sec, is_on in midi_pedal_events:
        key_char = 'down' if is_on else 'up'
        priority = 1 if is_on else 0
        result.append(KeyEvent(time_sec, priority, 'pedal', key_char))
    return result


def compile_pedal_events(
    config: PlaybackConfig,
    humanized_notes: List[Note],
    sections: List[MusicalSection],
    log: Optional[Callable[[str], None]] = None,
    out_meta: Optional[dict] = None,
    midi_pedal_events: Optional[List[Tuple[float, bool]]] = None,
) -> List[KeyEvent]:
    """Generate pedal KeyEvents from humanized notes.

    humanized_notes must be the list returned by compile_note_events (same
    timing as what the player will execute). Passing the raw pre-humanizer
    notes will cause the pedal to be misaligned with playback.

    When midi_pedal_events is non-empty and config['use_midi_pedal'] is True,
    the raw CC 64 events from the MIDI file are converted directly rather than
    running the generation algorithm.
    """
    if midi_pedal_events and config.get('use_midi_pedal', False):
        if log:
            log(f"[COMPILE] Pedal source: MIDI file CC 64 ({len(midi_pedal_events)} events)")
        return compile_midi_pedal_events(midi_pedal_events)
    if log:
        log(f"[COMPILE] Pedal style: {config.get('pedal_style', 'none')}")
    return pedal_generator.generate_events(config, humanized_notes, sections, log, out_meta)


def merge_compiled(
    note_events: List[KeyEvent],
    pedal_events: List[KeyEvent],
    log: Optional[Callable[[str], None]] = None,
) -> List[KeyEvent]:
    """Merge sorted note events and pedal events into a single sorted KeyEvent list.

    Creates a fresh heap each call so note_events can be reused across multiple
    pedal regeneration runs without mutation.
    """
    heap: List[KeyEvent] = list(note_events)
    heapq.heapify(heap)
    for ev in pedal_events:
        heapq.heappush(heap, ev)

    compiled: List[KeyEvent] = []
    while heap:
        compiled.append(heapq.heappop(heap))

    if log and compiled:
        total_duration = compiled[-1].time
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

        log(
            f"[COMPILE] Result: {len(compiled)} events | "
            f"press={press_count} release={release_count} pedal_down={pedal_down} pedal_up={pedal_up}"
        )
        log(
            f"[COMPILE] Duration: {total_duration:.2f}s | Pitch range: {pitch_range} | "
            f"Unique keys: {unique_keys}"
        )
        if total_duration > 0:
            log(
                f"[COMPILE] Density: {press_count / total_duration:.1f} presses/sec | "
                f"{pedal_down / total_duration:.2f} pedal-downs/sec"
            )

    return compiled


def compile_events(
    config: PlaybackConfig,
    notes: List[Note],
    sections: List[MusicalSection],
    log: Optional[Callable[[str], None]] = None,
    out_meta: Optional[dict] = None,
    midi_pedal_events: Optional[List[Tuple[float, bool]]] = None,
) -> List[KeyEvent]:
    """One-shot wrapper: humanize + pedal generation + merge.

    Backwards-compatible entry point used by _SaveWorker and the translator
    path's _PrepareWorker. For the two-phase MIDI play path use
    compile_note_events / compile_pedal_events / merge_compiled directly.

    out_meta is forwarded to compile_pedal_events; AI strategies populate it
    with 'threshold_on' / 'threshold_off' on success.

    midi_pedal_events -- when provided and config['use_midi_pedal'] is True,
    bypasses generation and uses the raw CC 64 events from the MIDI file.
    """
    note_events, humanized_notes = compile_note_events(config, notes, sections, log)
    pedal_events = compile_pedal_events(
        config, humanized_notes, sections, log, out_meta, midi_pedal_events
    )
    return merge_compiled(note_events, pedal_events, log)


def _get_mistake_pitch(original_pitch: int) -> Optional[int]:
    candidates = [original_pitch + d for d in (-2, -1, 1, 2)]
    if KeyMapper.is_black_key(original_pitch):
        black_pool = [p for p in candidates if KeyMapper.is_black_key(p)]
        white_pool = [p for p in candidates if not KeyMapper.is_black_key(p)]
        pool = (black_pool if random.random() < 0.5 else white_pool) or black_pool or white_pool
        return random.choice(pool) if pool else None
    valid = [p for p in candidates if not KeyMapper.is_black_key(p)]
    return random.choice(valid) if valid else None
