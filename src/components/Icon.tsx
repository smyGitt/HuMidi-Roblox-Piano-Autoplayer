const modules = import.meta.glob("../assets/icons/duotone/*.svg", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const iconSource = new Map<string, string>();
for (const path in modules) {
  const stem = path.split("/").pop()!.replace("-duotone.svg", "");
  iconSource.set(stem, modules[path]);
}

export type IconName =
  | "arrow-counter-clockwise"
  | "arrows-clockwise"
  | "bug"
  | "certificate"
  | "clock"
  | "delete-theme"
  | "discord-logo"
  | "export-theme"
  | "floppy-disk"
  | "folder-closed"
  | "folder-open"
  | "gear-six"
  | "github-logo"
  | "import-theme"
  | "inspect-mode"
  | "list-magnifying-glass"
  | "music-note"
  | "new-theme"
  | "palette"
  | "pause"
  | "play"
  | "rename-theme"
  | "resize-collapse"
  | "resize-expand"
  | "stop"
  | "toggle-left"
  | "toggle-right"
  | "translate"
  | "waveform";

interface IconProps {
  name: IconName;
  size?: number;
  className?: string;
  color?: string;
}

export function Icon({ name, size = 20, className, color }: IconProps) {
  const svg = iconSource.get(name);
  if (!svg) return null;
  return (
    <span
      className={className}
      style={{
        display: "inline-flex",
        width: size,
        height: size,
        color: color ?? "currentColor",
      }}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}
