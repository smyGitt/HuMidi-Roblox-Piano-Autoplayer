import { useRef, useState } from "react";
import {
  BugIcon,
  CertificateIcon,
  DiscordLogoIcon,
  GearSixIcon,
  GithubLogoIcon,
  MusicNoteIcon,
  TranslateIcon,
  WaveformIcon,
  type Icon as PhosphorIcon,
} from "@phosphor-icons/react";
import { StatusIndicator, type PlaybackStatus } from "./StatusIndicator";
import logo from "../assets/humidi_logo.ico";
import type { PageId } from "../pages/pageIds";

const NAV_ITEMS: { id: PageId; label: string; icon: PhosphorIcon }[] = [
  { id: "playback", label: "Playback", icon: MusicNoteIcon },
  { id: "visualizer", label: "Visualizer", icon: WaveformIcon },
  { id: "translator", label: "Translator", icon: TranslateIcon },
  { id: "settings", label: "Settings", icon: GearSixIcon },
  { id: "debug", label: "Debug", icon: BugIcon },
  { id: "license", label: "About", icon: CertificateIcon },
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
        {NAV_ITEMS.map(({ id, label, icon: NavIcon }) => (
          <button
            key={id}
            className={`sidebar__nav-btn${activePage === id ? " sidebar__nav-btn--active" : ""}`}
            disabled={id === "visualizer" && visualizerDisabled}
            onClick={() => onNavigate(id)}
          >
            <NavIcon size={20} weight="duotone" />
            <span className="sidebar__nav-label">{label}</span>
          </button>
        ))}
      </nav>

      <div className="sidebar__spacer" />

      <StatusIndicator status={status} label={statusLabel} />

      <a className="sidebar__link-btn" href="https://discord.gg/bRaXP9gYZN" target="_blank" rel="noreferrer" title="Discord">
        <DiscordLogoIcon size={20} weight="duotone" />
        <span className="sidebar__nav-label">Discord</span>
      </a>
      <a
        className="sidebar__link-btn"
        href="https://github.com/smyGitt/HuMidi-Roblox-Piano-Autoplayer/tree/main"
        target="_blank"
        rel="noreferrer"
        title="GitHub"
      >
        <GithubLogoIcon size={20} weight="duotone" />
        <span className="sidebar__nav-label">GitHub</span>
      </a>
    </aside>
  );
}
