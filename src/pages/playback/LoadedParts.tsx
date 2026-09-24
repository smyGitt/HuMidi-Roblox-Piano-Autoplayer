import { Card } from "../../components/Card";
import { PartCard } from "./PartCard";
import { Button } from "../../components/Button";

export interface TrackPart {
  name: string;
  meta: string;
}

interface LoadedPartsProps {
  parts: TrackPart[];
  pedalCount: number;
  onEditSelection: () => void;
  editEnabled: boolean;
}

export function LoadedParts({ parts, pedalCount, onEditSelection, editEnabled }: LoadedPartsProps) {
  return (
    <Card title="LOADED PARTS" className="loaded-parts-card">
      <div className="loaded-parts">
        <div className="loaded-parts__cards">
          {parts.length === 0 ? (
            <span className="loaded-parts__placeholder">No file loaded.</span>
          ) : (
            parts.map((p, i) => <PartCard key={i} title={p.name} meta={p.meta} />)
          )}
        </div>
        <div className="loaded-parts__pedal">
          <PartCard title="Pedal" meta={`${pedalCount} events`} />
        </div>
        <Button disabled={!editEnabled} onClick={onEditSelection}>
          Edit Selection
        </Button>
      </div>
    </Card>
  );
}
