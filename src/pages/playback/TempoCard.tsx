import { Card } from "../../components/Card";
import { SliderSpinbox } from "../../components/SliderSpinbox";
import { NumberInput } from "../../components/Field";

interface TempoCardProps {
  tempo: number;
  originalBpm: number;
  onTempoChange: (tempo: number) => void;
}

export function TempoCard({ tempo, originalBpm, onTempoChange }: TempoCardProps) {
  const resultBpm = originalBpm > 0 ? (originalBpm * tempo) / 100 : 0;

  return (
    <Card title="TEMPO" className="tempo-card">
      <SliderSpinbox min={10} max={1000} value={tempo} suffix="%" onChange={onTempoChange} />
      <div className="tempo-card__bpm-row">
        <label>
          Resulting BPM
          <NumberInput
            disabled={originalBpm <= 0}
            value={Number(resultBpm.toFixed(1))}
            onChange={(e) => {
              const target = Number(e.target.value);
              if (originalBpm > 0 && target > 0) {
                const multiplier = Math.min(1000, Math.max(10, (target / originalBpm) * 100));
                onTempoChange(multiplier);
              }
            }}
          />
        </label>
        <span className="tempo-card__original">{originalBpm > 0 ? `${originalBpm.toFixed(1)} BPM` : "-- BPM"}</span>
      </div>
    </Card>
  );
}
