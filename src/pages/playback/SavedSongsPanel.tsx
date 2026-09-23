import { Card } from "../../components/Card";
import { ArrowsClockwiseIcon, ClockIcon, ListMagnifyingGlassIcon } from "@phosphor-icons/react";

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
      titleButtons={
        <div className="saved-songs__title-btns">
          <button className="icon-btn" onClick={onRefresh} title="Refresh">
            <ArrowsClockwiseIcon size={16} weight="duotone" />
          </button>
          <button className="icon-btn" onClick={onOpenAll} title="All saves">
            <ListMagnifyingGlassIcon size={16} weight="duotone" />
          </button>
        </div>
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
                  <ClockIcon size={10} weight="duotone" />
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
