import math
from typing import List, Dict, Tuple

from pynput.keyboard import Key

from core.models import Note
from core.core import KeyMapper, TempoMap

_SHIFT_NUM_TO_SYMBOL = {
    '1': '!', '2': '@', '3': '#', '4': '$', '5': '%',
    '6': '^', '7': '&', '8': '*', '9': '(', '0': ')'
}
_SYMBOL_TO_BASE = {v: k for k, v in _SHIFT_NUM_TO_SYMBOL.items()}


def _build_pitch_to_char(key_mapper: KeyMapper) -> Dict[int, str]:
    """pitch -> display character for sheet notation (excludes Ctrl-modified keys)."""
    result = {}
    for pitch, data in key_mapper.key_map.items():
        key = data['key']
        mods = data['modifiers']
        if Key.ctrl in mods:
            continue
        if not mods:
            result[pitch] = key
        elif Key.shift in mods:
            if key.isalpha():
                result[pitch] = key.upper()
            elif key in _SHIFT_NUM_TO_SYMBOL:
                result[pitch] = _SHIFT_NUM_TO_SYMBOL[key]
    return result


def _build_char_to_pitch(key_mapper: KeyMapper) -> Dict[str, int]:
    """display character -> pitch (reverse of _build_pitch_to_char)."""
    result = {}
    for pitch, char in _build_pitch_to_char(key_mapper).items():
        if char not in result:
            result[char] = pitch
    return result


class VirtualPianoFormat:
    NAME = "Virtual Piano"


    @staticmethod
    def _parse_chunk(chunk: str) -> List[Tuple[List[str], int]]:
        """
        Parse one space-separated chunk into a list of (chars, dash_count) tuples.

        Rules:
        - '[abc]' groups chars into a chord.
        - A '-' immediately following a note/chord extends its duration.
        - A new char immediately following dashes starts the next note without
          requiring a space separator.  e.g. 'y-t' → [(['y'], 1), (['t'], 0)]
        """
        tokens = []
        i = 0
        while i < len(chunk):
            if chunk[i] == '[':
                j = chunk.find(']', i)
                if j == -1:
                    i += 1
                    continue
                chars = list(chunk[i + 1:j])
                i = j + 1
            elif chunk[i] == '-':
                i += 1
                continue
            else:
                chars = [chunk[i]]
                i += 1

            dashes = 0
            while i < len(chunk) and chunk[i] == '-':
                dashes += 1
                i += 1

            if chars:
                tokens.append((chars, dashes))

        return tokens


    @classmethod
    def parse(cls, text: str, bpm: float, key_mapper: KeyMapper) -> List[Note]:
        """Convert a Virtual Piano text sheet to Note objects.

        Each token is assumed to be a 16th note (0 dashes), 8th (1 dash),
        quarter (2 dashes), half (3 dashes), etc.
        Duration formula: base_16th * 2^dashes
        """
        char_to_pitch = _build_char_to_pitch(key_mapper)
        base_16th = 60.0 / (bpm * 4)

        notes: List[Note] = []
        note_id = 0
        current_time = 0.0

        for line in text.strip().split('\n'):
            line = line.strip()
            if not line:
                continue
            for chunk in line.split():
                for chars, dashes in cls._parse_chunk(chunk):
                    duration = base_16th * (2 ** dashes)
                    for char in chars:
                        pitch = char_to_pitch.get(char)
                        if pitch is not None:
                            notes.append(Note(
                                id=note_id,
                                pitch=pitch,
                                velocity=64,
                                start_time=current_time,
                                duration=duration,
                                hand='unknown',
                                original_track_index=0,
                                channel=0,
                            ))
                            note_id += 1
                    current_time += duration

        return notes


    @classmethod
    def serialize(cls, notes: List[Note], key_mapper: KeyMapper, tempo_map: TempoMap) -> str:
        """Convert Note objects to Virtual Piano sheet text.

        Notes:
        - Black keys that share a display char with a white key are skipped.
        - Ctrl-modified keys (88-key extended range) are skipped.
        - Rests (gaps between notes) are not represented; the exported sheet
          plays all notes packed sequentially at the chosen BPM.
        """
        if not notes:
            return ""

        pitch_to_char = _build_pitch_to_char(key_mapper)
        avg_bpm = cls._estimate_bpm(tempo_map)
        base_16th = 60.0 / (avg_bpm * 4)

        def quantize(t: float) -> float:
            return round(t / base_16th) * base_16th

        def duration_to_dashes(dur_secs: float) -> int:
            n16 = max(1, round(dur_secs / base_16th))
            exp = round(math.log2(n16))
            return max(0, min(exp, 3))

        groups: Dict[float, Dict] = {}
        for note in sorted(notes, key=lambda n: n.start_time):
            char = pitch_to_char.get(note.pitch)
            if char is None:
                continue
            q = quantize(note.start_time)
            if q not in groups:
                groups[q] = {'chars': [], 'duration': note.duration}
            else:
                groups[q]['duration'] = max(groups[q]['duration'], note.duration)
            if char not in groups[q]['chars']:
                groups[q]['chars'].append(char)

        if not groups:
            return "# No mappable notes found (all notes may be out of range or on Ctrl keys)."

        tokens = []
        for q_start in sorted(groups):
            group = groups[q_start]
            chars = group['chars']
            dashes = '-' * duration_to_dashes(group['duration'])
            if len(chars) == 1:
                tokens.append(f"{chars[0]}{dashes}")
            else:
                tokens.append(f"[{''.join(chars)}]{dashes}")

        lines = []
        current_line: List[str] = []
        line_len = 0
        for token in tokens:
            if line_len + len(token) + 1 > 80 and current_line:
                lines.append(' '.join(current_line))
                current_line = []
                line_len = 0
            current_line.append(token)
            line_len += len(token) + 1

        if current_line:
            lines.append(' '.join(current_line))

        return '\n'.join(lines)

    @staticmethod
    def _estimate_bpm(tempo_map: TempoMap) -> float:
        if not tempo_map.events:
            return 120.0
        return 60_000_000.0 / tempo_map.events[0][1]



class FormatRegistry:
    _FORMATS = {
        VirtualPianoFormat.NAME: VirtualPianoFormat,
    }

    @classmethod
    def names(cls) -> List[str]:
        return list(cls._FORMATS.keys())

    @classmethod
    def get(cls, name: str):
        return cls._FORMATS.get(name)
