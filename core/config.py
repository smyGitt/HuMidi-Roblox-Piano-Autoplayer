from typing import TypedDict


class MidiConfig(TypedDict, total=False):
    """Keys consumed by the MIDI parsing and key-mapping layer."""
    midi_file: str
    tempo: float
    transpose: int
    use_88_key_layout: bool


class HumanizationConfig(TypedDict, total=False):
    """Keys consumed by Humanizer and the hand-simulation step."""
    simulate_hands: bool
    humanization_on: bool
    vary_velocity: bool
    vary_timing: bool
    timing_variance: float
    vary_articulation: bool
    articulation: float
    enable_drift_correction: bool
    drift_decay_factor: float
    enable_chord_roll: bool
    enable_mistakes: bool
    mistake_chance: float
    enable_tempo_sway: bool
    tempo_sway_intensity: float
    invert_tempo_sway: bool


class PedalConfig(TypedDict, total=False):
    """Keys consumed by pedal_generator.generate_events."""
    pedal_style: str
    use_ai_pedal: bool
    pedal_threshold_on: float   # raw sigmoid threshold for pedal-down edge; -1.0 = auto
    pedal_threshold_off: float  # raw sigmoid threshold for pedal-up edge; -1.0 = auto


class PlaybackOptions(TypedDict, total=False):
    """Keys consumed by the Player execution loop."""
    countdown: bool
    auto_pause: bool
    debug_mode: bool


class PlaybackConfig(MidiConfig, HumanizationConfig, PedalConfig, PlaybackOptions):
    """Full playback configuration dict.

    Inherits from the four domain-specific sub-configs so that consumers can
    be typed against only the subset they actually read:

        def apply_to_hand(self, notes, config: HumanizationConfig): ...
        def generate_events(config: PedalConfig, ...): ...

    At runtime this is still a plain dict -- total=False on each base means all
    keys remain optional and no runtime enforcement is added.
    """
