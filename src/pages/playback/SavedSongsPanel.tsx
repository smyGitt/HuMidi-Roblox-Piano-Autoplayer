import { ArrowsClockwiseIcon, ClockIcon, ListMagnifyingGlassIcon } from "@phosphor-icons/react";
import { Button } from "../../components/Button";
import { Card } from "../../components/Card";

export interface SavedSong {
  filepath: string;
  saveName: string;
  songName: string;
  timeStr: string;
}

interface SavedSongsPanelProps {
  saves: SavedSong[];
  onRefresh: () => void;
  onOpenAll: () => void;
  onSaveClick: (save: SavedSong) => void;
}

export function SavedSongsPanel({ saves, onRefresh, onOpenAll, onSaveClick }: SavedSongsPanelProps) {
  return (
    <Card
      title="SAVED SONGS"
      className="saved-songs-card"
      titleButtons={
        <>
          <Button variant="icon" subtle onClick={onRefresh} title="Refresh">
            <ArrowsClockwiseIcon weight="duotone" />
          </Button>
          <Button variant="icon" subtle onClick={onOpenAll} title="All saves">
            <ListMagnifyingGlassIcon weight="duotone" />
          </Button>
        </>
      }
    >
      <div className="saved-songs__list">
        {saves.length === 0 ? (
          <span className="saved-songs__placeholder">No saved songs.</span>
        ) : (
          saves.map((s) => (
            <div key={s.filepath} className="save-card" onClick={() => onSaveClick(s)}>
              <span className="part-card__title">{s.saveName}</span>
              <span className="part-card__meta">{s.songName}</span>
              {s.timeStr && (
                <span className="save-card__time">
                  <ClockIcon weight="duotone" />
                  {s.timeStr}
                </span>
              )}
            </div>
          ))
        )}
      </div>
    </Card>
  );
}
