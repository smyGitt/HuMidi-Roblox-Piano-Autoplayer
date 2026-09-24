import { Card } from "../../components/Card";
import { PartCard } from "./PartCard";
import { Button } from "../../components/Button";

export interface TrackPart {
  name: string;
  meta: string;
}

interface LoadedRowProps {
  parts: TrackPart[];
  pedalCount: number;
  onEditSelection: () => void;
  editEnabled: boolean;
}

export function LoadedRow({ parts, pedalCount, onEditSelection, editEnabled }: LoadedRowProps) {
  return (
    <Card title="LOADED" className="loaded-row-card">
      <div className="loaded-row">
        <div className="loaded-row__cards">
          {parts.length === 0 ? (
            <span className="loaded-row__placeholder">No file loaded.</span>
          ) : (
            parts.map((p, i) => <PartCard key={i} title={p.name} meta={p.meta} />)
          )}
        </div>
        <div className="loaded-row__pedal">
          <PartCard title="Pedal" meta={`${pedalCount} events`} />
        </div>
        <Button disabled={!editEnabled} onClick={onEditSelection}>
          Edit Selection
        </Button>
      </div>
    </Card>
  );
}
