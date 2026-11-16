import { NotePencilIcon, SmileyMehIcon } from "@phosphor-icons/react";
import { PartCard } from "./PartCard";
import { Button } from "../../components/Button";
import { Label } from "../../components/Label";
import { SidePanel } from "../../components/SidePanel";

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
    <SidePanel>
      <div className="side-panel__head">
        <Label className="card__title">LOADED PARTS</Label>
        <Button variant="icon" subtle disabled={!editEnabled} onClick={onEditSelection} title="Edit Selection">
          <NotePencilIcon weight="fill" />
        </Button>
      </div>
      <div className="loaded-parts">
        <div className="loaded-parts__pedal">
          <PartCard title="Pedal" meta={`${pedalCount} events`} />
        </div>
        <div className="loaded-parts__cards">
          {parts.length === 0 ? (
            <div className="loaded-parts__empty">
              <SmileyMehIcon weight="fill" />
              <Label className="loaded-parts__placeholder">No file loaded.</Label>
            </div>
          ) : (
            parts.map((p, i) => <PartCard key={i} title={p.name} meta={p.meta} />)
          )}
        </div>
      </div>
    </SidePanel>
  );
}
