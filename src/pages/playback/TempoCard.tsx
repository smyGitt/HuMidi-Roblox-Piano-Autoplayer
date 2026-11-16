import { Card } from "../../components/Card";
import { SliderSpinbox } from "../../components/SliderSpinbox";
import { NumberInput } from "../../components/Field";

const MIN_TEMPO_PERCENT = 10;
const MAX_TEMPO_PERCENT = 500;
const PERCENT = 100;
const BPM_DECIMALS = 1;
const NO_BPM = "--";

function bpmText(bpm: number): string {
  return bpm > 0 ? String(Number(bpm.toFixed(BPM_DECIMALS))) : NO_BPM;
}

interface TempoCardProps {
  tempo: number;
  originalBpm: number;
  onTempoChange: (tempo: number) => void;
}

export function TempoCard({ tempo, originalBpm, onTempoChange }: TempoCardProps) {
  const resultBpm = originalBpm > 0 ? (originalBpm * tempo) / PERCENT : 0;

  return (
    <Card
      title="TEMPO"
      className="tempo-card"
      side={
        <>
          <div className="tempo-card__stat">
            <span className="tempo-card__label">Previous BPM</span>
            <span className="field field--number tempo-card__value">{bpmText(originalBpm)}</span>
          </div>
          <label className="tempo-card__stat">
            <span className="tempo-card__label">Resulting BPM</span>
            <NumberInput
              className="tempo-card__value"
              disabled={originalBpm <= 0}
              value={Number(resultBpm.toFixed(BPM_DECIMALS))}
              onChange={(e) => {
                const target = Number(e.target.value);
                if (originalBpm > 0 && target > 0) {
                  const multiplier = Math.min(
                    MAX_TEMPO_PERCENT,
                    Math.max(MIN_TEMPO_PERCENT, (target / originalBpm) * PERCENT),
                  );
                  onTempoChange(multiplier);
                }
              }}
            />
          </label>
        </>
      }
    >
      <SliderSpinbox min={MIN_TEMPO_PERCENT} max={MAX_TEMPO_PERCENT} value={tempo} suffix="%" onChange={onTempoChange} />
    </Card>
  );
}
