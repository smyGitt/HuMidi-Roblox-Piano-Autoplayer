import { FolderSimpleIcon, MusicNoteIcon, XSquareIcon } from "@phosphor-icons/react";
import { Button } from "../../components/Button";

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
      <Button variant="icon" size="lg" outlined onClick={onReplace} aria-label="Replace">
        <FolderSimpleIcon weight="duotone" />
      </Button>
      <Button variant="icon" size="lg" outlined onClick={onClear} aria-label="Clear">
        <XSquareIcon weight="duotone" />
      </Button>
    </div>
  );
}
