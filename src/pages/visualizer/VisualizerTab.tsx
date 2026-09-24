import { useEffect, useMemo, useState } from "react";
import { isTauri, type MeasureBoundary } from "../../lib/tauri";
import { useAppSettings } from "../../state/AppSettingsContext";
import { usePlaybackEngine } from "../../state/PlaybackEngineContext";
import { PianoRoll } from "./PianoRoll";
import { PianoKeyboard } from "./PianoKeyboard";
import { buildDemoNotes, type VizNote } from "./demoNotes";
import { Button } from "../../components/Button";

const MIN_PITCH = 21;
const MAX_PITCH = 108;

function demoMeasureBoundaries(totalDuration: number): MeasureBoundary[] {
  const boundaries: MeasureBoundary[] = [];
  for (let start = 0; start < totalDuration; start += 2) {
    boundaries.push([start, Math.min(start + 2, totalDuration)]);
  }
  return boundaries;
}

export function VisualizerTab() {
  const { showTimeline, showPiano, showPianoPedal } = useAppSettings();
  const engine = usePlaybackEngine();

  const usingRealData = isTauri() && engine.hasCompiledNotes;
  const [dragTime, setDragTime] = useState<number | null>(null);

  const demo = useMemo(() => buildDemoNotes(), []);
  const demoMeasures = useMemo(() => demoMeasureBoundaries(demo.totalDuration), [demo.totalDuration]);
  const [demoTime, setDemoTime] = useState(0);
  const [demoPlaying, setDemoPlaying] = useState(true);

  useEffect(() => {
    if (usingRealData || !demoPlaying) return;
    const id = setInterval(() => {
      setDemoTime((t) => (t + 0.05 >= demo.totalDuration ? 0 : t + 0.05));
    }, 50);
    return () => clearInterval(id);
  }, [usingRealData, demoPlaying, demo.totalDuration]);

  const demoActivePitches = useMemo(() => {
    const active = new Set<number>();
    for (const n of demo.notes) {
      if (n.start <= demoTime && demoTime < n.start + n.duration) active.add(n.pitch);
    }
    return active;
  }, [demo.notes, demoTime]);

  const demoPedalActive = useMemo(
    () => demo.pedalIntervals.some(([start, end]) => start <= demoTime && demoTime < end),
    [demo.pedalIntervals, demoTime],
  );

  const notes: VizNote[] = usingRealData
    ? engine.finalNotes.map((n) => ({
        pitch: n.pitch,
        start: n.start_time,
        duration: n.duration,
        hand: n.hand === "left" || n.hand === "right" ? n.hand : "unknown",
      }))
    : demo.notes;
  const totalDuration = usingRealData ? Math.max(engine.totalDuration, 0.1) : demo.totalDuration;
  const pedalIntervals = usingRealData ? engine.pedalIntervals : demo.pedalIntervals;
  const measureBoundaries = usingRealData ? engine.measureBoundaries : demoMeasures;
  const liveCurrentTime = usingRealData ? engine.currentTime : demoTime;
  const liveActivePitches = usingRealData ? engine.activePitches : demoActivePitches;
  const livePedalActive = usingRealData ? engine.pedalActive : demoPedalActive;

  const scrubPreview = useMemo(() => {
    if (dragTime === null) return null;
    const active = new Set<number>();
    for (const n of notes) {
      if (n.start <= dragTime && dragTime < n.start + n.duration) active.add(n.pitch);
    }
    let pedalDown = false;
    for (const [start, end] of pedalIntervals) {
      if (start <= dragTime) pedalDown = dragTime < end;
      else break;
    }
    return { activePitches: active, pedalActive: pedalDown };
  }, [dragTime, notes, pedalIntervals]);

  const currentTime = dragTime ?? liveCurrentTime;
  const activePitches = scrubPreview ? scrubPreview.activePitches : liveActivePitches;
  const pedalActive = scrubPreview ? scrubPreview.pedalActive : livePedalActive;

  function handleScrub(time: number) {
    setDragTime(time);
  }

  function handleSeek(time: number) {
    setDragTime(null);
    if (usingRealData) void engine.seek(time);
    else setDemoTime(time);
  }

  return (
    <div className="visualizer-tab">
      <div className="visualizer-tab__toolbar">
        <span className="visualizer-tab__hint">
          {usingRealData ? engine.fileName : "Demo data (no MIDI compiled yet)"} · 88-key layout and Timeline/Piano
          visibility: Settings &gt; Display / Playback &gt; Options
        </span>
        {!usingRealData && (
          <Button className="visualizer-tab__play-btn" onClick={() => setDemoPlaying((p) => !p)}>
            {demoPlaying ? "Pause" : "Play"} demo
          </Button>
        )}
      </div>

      {!showTimeline && !showPiano && (
        <div className="visualizer-tab__empty">
          Both the timeline and the piano are hidden. Re-enable them in Settings &gt; Display.
        </div>
      )}

      {showTimeline && (
        <PianoRoll
          notes={notes}
          totalDuration={totalDuration}
          currentTime={currentTime}
          measureBoundaries={measureBoundaries}
          pedalIntervals={pedalIntervals}
          minPitch={MIN_PITCH}
          maxPitch={MAX_PITCH}
          showPedal
          onScrub={handleScrub}
          onSeek={handleSeek}
        />
      )}

      {showPiano && (
        <PianoKeyboard
          minPitch={MIN_PITCH}
          maxPitch={MAX_PITCH}
          activePitches={activePitches}
          pedalActive={pedalActive}
          showPedal={showPianoPedal}
        />
      )}
    </div>
  );
}
