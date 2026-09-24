import { ArrowsClockwiseIcon, ClockIcon, ListIcon, ListMagnifyingGlassIcon } from "@phosphor-icons/react";
import { Button } from "../../components/Button";

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
    <aside className="saved-songs-sidebar">
      <div className="saved-songs-sidebar__handle">
        <ListIcon weight="duotone" />
      </div>
      <div className="saved-songs-sidebar__content">
        <div className="saved-songs-sidebar__head">
          <span className="card__title">SAVED SONGS</span>
          <div className="saved-songs__title-btns">
            <Button variant="icon" subtle onClick={onRefresh} title="Refresh">
              <ArrowsClockwiseIcon weight="duotone" />
            </Button>
            <Button variant="icon" subtle onClick={onOpenAll} title="All saves">
              <ListMagnifyingGlassIcon weight="duotone" />
            </Button>
          </div>
        </div>
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
      </div>
    </aside>
  );
}
