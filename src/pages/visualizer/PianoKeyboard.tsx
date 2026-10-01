import type { KeyLayout } from "./keyLayout";

const PERCENT = 100;

interface PianoKeyboardProps {
  layout: KeyLayout;
  pressedPitches: ReadonlySet<number>;
  pedalDown: boolean;
  showPedal: boolean;
}

function percent(fraction: number): string {
  return `${fraction * PERCENT}%`;
}

export function PianoKeyboard({ layout, pressedPitches, pedalDown, showPedal }: PianoKeyboardProps) {
  return (
    <div className="piano-keyboard">
      <div className="piano-keyboard__keys">
        {layout.keys.map((key) => (
          <div
            key={key.pitch}
            className={[
              "piano-keyboard__key",
              key.black ? "piano-keyboard__key--black" : "piano-keyboard__key--white",
              pressedPitches.has(key.pitch) && "piano-keyboard__key--pressed",
            ]
              .filter(Boolean)
              .join(" ")}
            style={{ left: percent(key.left), width: percent(key.width) }}
          />
        ))}
      </div>
      {showPedal && (
        <div className="piano-keyboard__pedal">
          <div className={`piano-keyboard__pedal-fill${pedalDown ? " piano-keyboard__pedal-fill--down" : ""}`} />
        </div>
      )}
    </div>
  );
}
