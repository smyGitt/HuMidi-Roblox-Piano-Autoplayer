import { FolderSimpleIcon, MusicNoteIcon, XSquareIcon } from "@phosphor-icons/react";

interface FileStripProps {
  name: string;
  meta: string;
  onReplace: () => void;
  onClear: () => void;
}

export function FileStrip({ name, meta, onReplace, onClear }: FileStripProps) {
  return (
    <div className="file-strip">
      <div className="file-strip__tile">
        <MusicNoteIcon size={20} weight="duotone" />
      </div>
      <div className="file-strip__info">
        <span className="file-strip__name">{name || "No file loaded"}</span>
        {meta && <span className="file-strip__meta">{meta}</span>}
      </div>
      <button className="file-strip__btn" onClick={onReplace} aria-label="Replace">
        <FolderSimpleIcon weight="duotone" />
      </button>
      <button className="file-strip__btn" onClick={onClear} aria-label="Clear">
        <XSquareIcon weight="duotone" />
      </button>
    </div>
  );
}
