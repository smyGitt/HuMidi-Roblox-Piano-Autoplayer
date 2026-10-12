import { beforeEach, describe, expect, it, vi } from "vitest";
import { forwardKey, toBackendConfig } from "./tauri";
import { DEFAULT_CONFIG, type PlaybackConfig } from "../pages/playback/types";

const coreMocks = vi.hoisted(() => ({
  invoke: vi.fn<(command: string, args?: Record<string, unknown>) => Promise<unknown>>(),
}));

vi.mock("@tauri-apps/api/core", () => coreMocks);

beforeEach(() => {
  coreMocks.invoke.mockReset().mockResolvedValue(undefined);
});

describe("forwardKey", () => {
  it("sends the key code and direction to the forward_key command", async () => {
    await forwardKey("KeyG", true);
    expect(coreMocks.invoke).toHaveBeenCalledTimes(1);
    expect(coreMocks.invoke).toHaveBeenCalledWith("forward_key", { code: "KeyG", pressed: true });
  });

  it("sends a release as pressed false", async () => {
    await forwardKey("ControlLeft", false);
    expect(coreMocks.invoke).toHaveBeenCalledWith("forward_key", { code: "ControlLeft", pressed: false });
  });

  it("passes a backend failure on to the caller", async () => {
    coreMocks.invoke.mockRejectedValue(new Error("no backend"));
    await expect(forwardKey("KeyG", true)).rejects.toThrow("no backend");
  });
});

function config(patch: Partial<PlaybackConfig> = {}): PlaybackConfig {
  return { ...DEFAULT_CONFIG, ...patch };
}

describe("toBackendConfig", () => {
  it("converts tempo from a UI percentage to the Rust multiplier", () => {
    expect(toBackendConfig(config({ tempo: 100 }), "song.mid").tempo).toBeCloseTo(1.0);
    expect(toBackendConfig(config({ tempo: 250 }), "song.mid").tempo).toBeCloseTo(2.5);
    expect(toBackendConfig(config({ tempo: 10 }), "song.mid").tempo).toBeCloseTo(0.1);
    expect(toBackendConfig(config({ tempo: 1000 }), "song.mid").tempo).toBeCloseTo(10.0);
  });

  it("converts articulation from a UI percentage to a 0-1 fraction", () => {
    expect(toBackendConfig(config({ articulation: 95 }), "song.mid").articulation).toBeCloseTo(0.95);
    expect(toBackendConfig(config({ articulation: 50 }), "song.mid").articulation).toBeCloseTo(0.5);
    expect(toBackendConfig(config({ articulation: 100 }), "song.mid").articulation).toBeCloseTo(1.0);
  });

  it("converts drift_decay_factor from a UI percentage to a 0-1 fraction", () => {
    expect(toBackendConfig(config({ drift_decay_factor: 25 }), "song.mid").drift_decay_factor).toBeCloseTo(0.25);
    expect(toBackendConfig(config({ drift_decay_factor: 0 }), "song.mid").drift_decay_factor).toBeCloseTo(0);
    expect(toBackendConfig(config({ drift_decay_factor: 100 }), "song.mid").drift_decay_factor).toBeCloseTo(1.0);
  });

  it("passes mistake_chance through unconverted (Rust divides by 100 internally)", () => {
    expect(toBackendConfig(config({ mistake_chance: 0.5 }), "song.mid").mistake_chance).toBe(0.5);
    expect(toBackendConfig(config({ mistake_chance: 10 }), "song.mid").mistake_chance).toBe(10);
  });

  it("passes timing_variance and tempo_sway_intensity through unconverted (both already in seconds)", () => {
    const c = config({ timing_variance: 0.02, tempo_sway_intensity: 0.05 });
    const backend = toBackendConfig(c, "song.mid");
    expect(backend.timing_variance).toBe(0.02);
    expect(backend.tempo_sway_intensity).toBe(0.05);
  });

  it("passes the pedal threshold curves through unconverted, including empty meaning automatic", () => {
    expect(toBackendConfig(config({ pedal_curve_on: [] }), "song.mid").pedal_curve_on).toEqual([]);
    const nodes = [{ time: 0, value: 0.62 }, { time: 5, value: 0.4 }];
    const backend = toBackendConfig(config({ pedal_curve_on: nodes, pedal_curve_off: nodes }), "song.mid");
    expect(backend.pedal_curve_on).toEqual(nodes);
    expect(backend.pedal_curve_off).toEqual(nodes);
  });

  it("passes transpose and boolean fields through unconverted", () => {
    const c = config({
      transpose: -12,
      countdown: false,
      release_pedal_on_start: true,
      simulate_hands: true,
      use_midi_pedal: true,
      invert_tempo_sway: true,
    });
    const backend = toBackendConfig(c, "song.mid");
    expect(backend.transpose).toBe(-12);
    expect(backend.countdown).toBe(false);
    expect(backend.release_pedal_on_start).toBe(true);
    expect(backend.simulate_hands).toBe(true);
    expect(backend.use_midi_pedal).toBe(true);
    expect(backend.invert_tempo_sway).toBe(true);
  });

  it("always hardcodes vary_velocity and use_ai_pedal to false regardless of frontend state", () => {
    const backend = toBackendConfig(config(), "song.mid");
    expect(backend.vary_velocity).toBe(false);
    expect(backend.use_ai_pedal).toBe(false);
  });

  it("takes midi_file from the explicit path argument, not from the config object", () => {
    expect(toBackendConfig(config({ midi_file: "wrong.mid" }), "/real/path/song.mid").midi_file).toBe(
      "/real/path/song.mid",
    );
  });

  it("round-trips every remaining field name unchanged", () => {
    const c = config({ pedal_style: "legato", pedal_before_humanize: false });
    const backend = toBackendConfig(c, "song.mid");
    expect(backend.pedal_style).toBe("legato");
    expect(backend.pedal_before_humanize).toBe(false);
  });
});
