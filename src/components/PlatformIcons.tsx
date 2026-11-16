import { LinuxLogoIcon, TriangleIcon, WindowsLogoIcon, XIcon } from "@phosphor-icons/react";
import { useState } from "react";
import { useKeepOnScreen } from "./useKeepOnScreen";

export type LinuxSupport = boolean | "partial";

interface PlatformIconsProps {
  windows: boolean;
  linux: LinuxSupport;
  linuxWorks?: readonly string[];
  linuxFails?: readonly string[];
}

function SystemList({ heading, systems }: { heading: string; systems: readonly string[] }) {
  if (systems.length === 0) return null;
  return (
    <>
      <span className="platform-icons__heading">{heading}</span>
      <ul className="platform-icons__list">
        {systems.map((system) => (
          <li key={system}>{system}</li>
        ))}
      </ul>
    </>
  );
}

export function PlatformIcons({ windows, linux, linuxWorks = [], linuxFails = [] }: PlatformIconsProps) {
  const [hovered, setHovered] = useState(false);
  const hasPanel = linuxWorks.length > 0 || linuxFails.length > 0;
  const { ref: panelRef, style: panelStyle, aboveClass } = useKeepOnScreen<HTMLDivElement>(hovered && hasPanel);
  return (
    <span className="platform-icons">
      <span className="platform-icons__windows">
        {windows ? (
          <WindowsLogoIcon aria-label="Works on Windows" weight="fill" />
        ) : (
          <XIcon aria-label="Not supported on Windows" weight="bold" />
        )}
      </span>
      {linux === false ? (
        <span className="platform-icons__linux">
          <XIcon aria-label="Not supported on Linux" weight="bold" />
        </span>
      ) : (
        <span
          className="platform-icons__linux info-tip"
          onMouseEnter={() => setHovered(true)}
          onMouseLeave={() => setHovered(false)}
        >
          {linux === true ? (
            <LinuxLogoIcon aria-label="Works on Linux" weight="fill" />
          ) : (
            <TriangleIcon aria-label="Partly works on Linux" weight="fill" />
          )}
          {hovered && hasPanel && (
            <div role="tooltip" ref={panelRef} style={panelStyle} className={`info-tip__text platform-icons__panel${aboveClass}`}>
              <SystemList heading="Tested on:" systems={linuxWorks} />
              <SystemList heading="Does not work on:" systems={linuxFails} />
            </div>
          )}
        </span>
      )}
    </span>
  );
}
