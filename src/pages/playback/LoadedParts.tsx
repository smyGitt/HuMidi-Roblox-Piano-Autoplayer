import { ListIcon, NotePencilIcon } from "@phosphor-icons/react";
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
    <aside className="side-panel">
      <div className="side-panel__handle">
        <ListIcon weight="duotone" />
      </div>
      <div className="side-panel__content">
        <div className="side-panel__head">
          <span className="card__title">LOADED PARTS</span>
          <Button variant="icon" subtle disabled={!editEnabled} onClick={onEditSelection} title="Edit Selection">
            <NotePencilIcon weight="duotone" />
          </Button>
        </div>
        <div className="loaded-parts">
          <div className="loaded-parts__pedal">
            <PartCard title="Pedal" meta={`${pedalCount} events`} />
          </div>
          <div className="loaded-parts__cards">
            {parts.length === 0 ? (
              <span className="loaded-parts__placeholder">No file loaded.</span>
            ) : (
              parts.map((p, i) => <PartCard key={i} title={p.name} meta={p.meta} />)
            )}
          </div>
        </div>
      </div>
    </aside>
  );
}
