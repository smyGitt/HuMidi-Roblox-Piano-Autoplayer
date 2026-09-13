import { Icon } from "../../components/Icon";

interface FileStripProps {
  name: string;
  meta: string;
  onReplace: () => void;
  onReveal: () => void;
}

export function FileStrip({ name, meta, onReplace, onReveal }: FileStripProps) {
  return (
    <div className="file-strip">
      <div className="file-strip__tile">
        <Icon name="music-note" size={20} />
      </div>
      <div className="file-strip__info">
        <span className="file-strip__name">{name || "No file loaded"}</span>
        {meta && <span className="file-strip__meta">{meta}</span>}
      </div>
      <button className="file-strip__btn" onClick={onReplace}>
        Replace
      </button>
      <button className="file-strip__btn" onClick={onReveal}>
        Reveal
      </button>
    </div>
  );
}
