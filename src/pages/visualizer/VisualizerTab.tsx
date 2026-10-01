import { useMemo, useState, useSyncExternalStore } from "react";
import { foldPitch, getKeyRange } from "../../lib/keyRange";
import { activePitchesAt, buildNoteTimeline, pedalActiveAt } from "../../lib/noteTimeline";
import { useAppSettings } from "../../state/AppSettingsContext";
import { usePlaybackConfig } from "../../state/PlaybackConfigContext";
import { usePlaybackEngine } from "../../state/PlaybackEngineContext";
import { buildKeyLayout } from "./keyLayout";
import { NoteCanvas } from "./NoteCanvas";
import { PianoKeyboard } from "./PianoKeyboard";

export function VisualizerTab() {
  const { showTimeline, showPiano, showPianoPedal, visualizerHorizontal } = useAppSettings();
  const { config } = usePlaybackConfig();
  const engine = usePlaybackEngine();
  const live = useSyncExternalStore(engine.liveKeys.subscribe, engine.liveKeys.snapshot);
  const [dragTime, setDragTime] = useState<number | null>(null);

  const range = getKeyRange(config.use_88_key_layout);
  const layout = useMemo(() => buildKeyLayout(range), [range]);
  const timeline = useMemo(() => buildNoteTimeline(engine.playedNotes, range), [engine.playedNotes, range]);

  const keys = useMemo(() => {
    if (dragTime !== null) {
      return { pitches: activePitchesAt(timeline, dragTime), pedal: pedalActiveAt(engine.pedalIntervals, dragTime) };
    }
    return { pitches: new Set([...live.pitches].map((pitch) => foldPitch(pitch, range))), pedal: live.pedal };
  }, [dragTime, timeline, engine.pedalIntervals, live, range]);

  return (
    <div className="visualizer-tab">
      {showTimeline && (
        <NoteCanvas
          clock={engine.clock}
          timeline={timeline}
          layout={layout}
          measureBoundaries={engine.measureBoundaries}
          pedalIntervals={engine.pedalIntervals}
          horizontal={visualizerHorizontal}
          duration={engine.totalDuration}
          onDrag={setDragTime}
          onSeek={(time) => void engine.seek(time)}
        />
      )}
      {showPiano && (
        <PianoKeyboard layout={layout} pressedPitches={keys.pitches} pedalDown={keys.pedal} showPedal={showPianoPedal} />
      )}
    </div>
  );
}
