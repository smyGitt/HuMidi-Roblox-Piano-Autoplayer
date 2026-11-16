import { LinuxLogoIcon, TriangleIcon, WindowsLogoIcon } from "@phosphor-icons/react";

export type LinuxSupport = boolean | "partial";

interface PlatformIconsProps {
  windows: boolean;
  linux: LinuxSupport;
}

export function PlatformIcons({ windows, linux }: PlatformIconsProps) {
  return (
    <span className="platform-icons">
      {windows && <WindowsLogoIcon aria-label="Works on Windows" weight="fill" />}
      {linux === true && <LinuxLogoIcon aria-label="Works on Linux" weight="fill" />}
      {linux === "partial" && <TriangleIcon aria-label="Partly works on Linux" weight="fill" />}
    </span>
  );
}
