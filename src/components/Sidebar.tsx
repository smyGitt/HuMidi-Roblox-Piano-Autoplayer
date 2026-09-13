import { useRef, useState } from "react";
import { Icon } from "./Icon";
import { StatusIndicator, type PlaybackStatus } from "./StatusIndicator";
import logo from "../assets/humidi_logo.ico";
import type { PageId } from "../pages/pageIds";

const NAV_ITEMS: { id: PageId; label: string; icon: Parameters<typeof Icon>[0]["name"] }[] = [
  { id: "playback", label: "Playback", icon: "music-note" },
  { id: "visualizer", label: "Visualizer", icon: "waveform" },
  { id: "translator", label: "Translator", icon: "translate" },
  { id: "settings", label: "Settings", icon: "gear-six" },
  { id: "debug", label: "Debug", icon: "bug" },
  { id: "license", label: "About", icon: "certificate" },
];

interface SidebarProps {
  activePage: PageId;
  onNavigate: (page: PageId) => void;
  status: PlaybackStatus;
  statusLabel?: string;
  visualizerDisabled?: boolean;
}

export function Sidebar({ activePage, onNavigate, status, statusLabel, visualizerDisabled }: SidebarProps) {
  const [expanded, setExpanded] = useState(false);
  const collapseTimer = useRef<number | null>(null);

  function handleEnter() {
    if (collapseTimer.current) {
      window.clearTimeout(collapseTimer.current);
      collapseTimer.current = null;
    }
    setExpanded(true);
  }

  function handleLeave() {
    collapseTimer.current = window.setTimeout(() => setExpanded(false), 120);
  }

  return (
    <aside
      className={`sidebar${expanded ? " sidebar--expanded" : ""}`}
      onMouseEnter={handleEnter}
      onMouseLeave={handleLeave}
    >
      <div className="sidebar__logo">
        <img src={logo} alt="" width={22} height={22} />
        <span className="sidebar__wordmark">
          Hu<i>Midi</i>
        </span>
      </div>

      <nav className="sidebar__nav">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.id}
            className={`sidebar__nav-btn${activePage === item.id ? " sidebar__nav-btn--active" : ""}`}
            disabled={item.id === "visualizer" && visualizerDisabled}
            onClick={() => onNavigate(item.id)}
          >
            <Icon name={item.icon} size={20} />
            <span className="sidebar__nav-label">{item.label}</span>
          </button>
        ))}
      </nav>

      <div className="sidebar__spacer" />

      <StatusIndicator status={status} label={statusLabel} />

      <a className="sidebar__link-btn" href="https://discord.gg/bRaXP9gYZN" target="_blank" rel="noreferrer" title="Discord">
        <Icon name="discord-logo" size={20} />
        <span className="sidebar__nav-label">Discord</span>
      </a>
      <a
        className="sidebar__link-btn"
        href="https://github.com/smyGitt/HuMidi-Roblox-Piano-Autoplayer/tree/main"
        target="_blank"
        rel="noreferrer"
        title="GitHub"
      >
        <Icon name="github-logo" size={20} />
        <span className="sidebar__nav-label">GitHub</span>
      </a>
    </aside>
  );
}
