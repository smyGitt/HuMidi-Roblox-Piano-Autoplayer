"""Temporary session cache: stores the most-recently compiled note and pedal
events together with the config snapshots used to produce them.

The cache lives in the OS temp directory so it is automatically cleaned up on
system restart. It is also explicitly deleted on application close.

The cache enables two things:
  1. Freshness checks -- compare the current UI config against the stored
     snapshot to know whether the compiled events are still current.
  2. Discard -- restore the UI to the config that was used for the last
     successful compilation (so the compiled events and the displayed settings
     are back in sync without regenerating).

Write is asynchronous (daemon thread + atomic rename) so it never blocks the
compilation worker or the GUI thread.
"""

import json
import os
import tempfile
import threading
from typing import List, Optional

from core.models import Note, KeyEvent

_CACHE_PATH = os.path.join(tempfile.gettempdir(), 'humidi_session.json')

# Config keys that drive note humanization. Changing any of these makes the
# note events (and therefore the pedal events) stale.
NOTES_CONFIG_KEYS: frozenset = frozenset([
    'simulate_hands',
    'vary_timing', 'timing_variance',
    'vary_articulation', 'articulation',
    'enable_drift_correction', 'drift_decay_factor',
    'enable_chord_roll',
    'enable_tempo_sway', 'tempo_sway_intensity', 'invert_tempo_sway',
    'enable_mistakes', 'mistake_chance',
    'tempo',
    'use_88_key_layout',
])

# Config keys that drive pedal generation only. Changing these makes only the
# pedal events stale; the note events can be reused.
PEDAL_CONFIG_KEYS: frozenset = frozenset([
    'pedal_style',
    'use_ai_pedal',
    'pedal_threshold_on',
    'pedal_threshold_off',
    'use_midi_pedal',
])


def extract_notes_config(config: dict) -> dict:
    """Return the subset of config that affects note compilation."""
    return {k: config[k] for k in NOTES_CONFIG_KEYS if k in config}


def extract_pedal_config(config: dict) -> dict:
    """Return the subset of config that affects pedal generation."""
    return {k: config[k] for k in PEDAL_CONFIG_KEYS if k in config}


def notes_config_matches(snapshot: Optional[dict], config: dict) -> bool:
    """True when snapshot is non-None and all notes-relevant keys match config."""
    if snapshot is None:
        return False
    current = extract_notes_config(config)
    return snapshot == current


def pedal_config_matches(snapshot: Optional[dict], config: dict) -> bool:
    """True when snapshot is non-None and all pedal-relevant keys match config."""
    if snapshot is None:
        return False
    current = extract_pedal_config(config)
    return snapshot == current


def write_cache(
    notes_config: dict,
    pedal_config: dict,
    note_events: List[KeyEvent],
    pedal_events: List[KeyEvent],
    humanized_notes: List[Note],
    final_notes: List[Note],
    tempo_map_data: dict,
    total_dur: float,
) -> None:
    """Persist the compiled session state to the temp cache file.

    Runs on a daemon thread so it never blocks the caller. The write is atomic
    (temp-file + os.replace) so a partial write cannot corrupt the cache.
    """
    data = {
        'notes_config': notes_config,
        'pedal_config': pedal_config,
        'note_events': [_ser_event(e) for e in note_events],
        'pedal_events': [_ser_event(e) for e in pedal_events],
        'humanized_notes': [_ser_note(n) for n in humanized_notes],
        'final_notes': [_ser_note(n) for n in final_notes],
        'tempo_map': tempo_map_data,
        'total_dur': total_dur,
    }
    threading.Thread(target=_write_atomic, args=(_CACHE_PATH, data), daemon=True).start()


def read_cache() -> Optional[dict]:
    """Read and return the cache dict, or None on any error."""
    try:
        with open(_CACHE_PATH, 'r') as f:
            return json.load(f)
    except (OSError, json.JSONDecodeError, KeyError, ValueError):
        return None


def clear_cache() -> None:
    """Delete the cache file. No-op if it does not exist."""
    try:
        os.remove(_CACHE_PATH)
    except OSError:
        pass


def tempo_map_to_dict(tempo_map) -> dict:
    """Serialize a TempoMap to a plain dict for JSON storage."""
    return {
        'events': [[t, us] for t, us in tempo_map.events],
        'time_signatures': [[t, n, d] for t, n, d in tempo_map.time_signatures],
    }


# -- Private helpers ----------------------------------------------------------

def _write_atomic(path: str, data: dict) -> None:
    target_dir = os.path.dirname(path) or '.'
    fd, tmp = tempfile.mkstemp(dir=target_dir, suffix='.tmp')
    try:
        with os.fdopen(fd, 'w') as f:
            json.dump(data, f)
        os.replace(tmp, path)
    except Exception:
        try:
            os.remove(tmp)
        except OSError:
            pass


def _ser_event(ev: KeyEvent) -> dict:
    return {
        'time': ev.time,
        'priority': ev.priority,
        'action': ev.action,
        'key_char': ev.key_char,
        'pitch': ev.pitch,
    }


def _ser_note(n: Note) -> dict:
    return {
        'id': n.id,
        'pitch': n.pitch,
        'velocity': n.velocity,
        'start_time': n.start_time,
        'duration': n.duration,
        'hand': n.hand,
    }
