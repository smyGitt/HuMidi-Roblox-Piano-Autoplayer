import { useMemo } from "react";
import { isBlackKey } from "../../lib/keyRange";

const HEIGHT = 92;
const PEDAL_STRIP_H = 12;
const PEDAL_GAP = 4;

interface PianoKeyboardProps {
  minPitch: number;
  maxPitch: number;
  activePitches: Set<number>;
  pedalActive: boolean;
  showPedal: boolean;
}

interface WhiteKey {
  pitch: number;
  index: number;
}

export function PianoKeyboard({ minPitch, maxPitch, activePitches, pedalActive, showPedal }: PianoKeyboardProps) {
  const { whiteKeys, blackKeys } = useMemo(() => {
    const white: WhiteKey[] = [];
    const black: { pitch: number; afterWhiteIndex: number }[] = [];
    let whiteIndex = 0;
    for (let p = minPitch; p <= maxPitch; p++) {
      if (isBlackKey(p)) {
        black.push({ pitch: p, afterWhiteIndex: whiteIndex - 1 });
      } else {
        white.push({ pitch: p, index: whiteIndex });
        whiteIndex++;
      }
    }
    return { whiteKeys: white, blackKeys: black };
  }, [minPitch, maxPitch]);

  const keyAreaHeight = showPedal ? HEIGHT - PEDAL_STRIP_H - PEDAL_GAP : HEIGHT;
  const whiteKeyWidthPct = 100 / whiteKeys.length;
  const blackKeyWidthPct = whiteKeyWidthPct * 0.62;

  return (
    <div className="piano-keyboard" style={{ height: HEIGHT }}>
      <div className="piano-keyboard__keys" style={{ height: keyAreaHeight }}>
        {whiteKeys.map((k) => (
          <div
            key={k.pitch}
            className={`piano-keyboard__white${activePitches.has(k.pitch) ? " piano-keyboard__white--active" : ""}`}
            style={{ width: `${whiteKeyWidthPct}%` }}
          />
        ))}
        {blackKeys.map((k) => (
          <div
            key={k.pitch}
            className={`piano-keyboard__black${activePitches.has(k.pitch) ? " piano-keyboard__black--active" : ""}`}
            style={{
              width: `${blackKeyWidthPct}%`,
              left: `${(k.afterWhiteIndex + 1) * whiteKeyWidthPct - blackKeyWidthPct / 2}%`,
            }}
          />
        ))}
      </div>
      {showPedal && (
        <div className="piano-keyboard__pedal-strip" style={{ height: PEDAL_STRIP_H, marginTop: PEDAL_GAP }}>
          <div className={`piano-keyboard__pedal-fill${pedalActive ? " piano-keyboard__pedal-fill--active" : ""}`} />
        </div>
      )}
    </div>
  );
}
