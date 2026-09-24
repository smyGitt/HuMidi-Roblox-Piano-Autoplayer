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
import logo from "../assets/humidi_logo.ico";
import type { PageId } from "../pages/pageIds";
import { Button, Link } from "./Button";

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
  visualizerDisabled?: boolean;
}

export function Sidebar({ activePage, onNavigate, visualizerDisabled }: SidebarProps) {
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
        <img src={logo} alt="" width={24} height={24} />
        <span className="sidebar__wordmark">
          Hu<i>Midi</i>
        </span>
      </div>

      <nav className="sidebar__nav">
        {NAV_ITEMS.map(({ id, label, icon: NavIcon }) => (
          <Button
            key={id}
            variant="nav" active={activePage === id}
            disabled={id === "visualizer" && visualizerDisabled}
            onClick={() => onNavigate(id)}
          >
            <NavIcon weight="duotone" />
            <span className="sidebar__nav-label">{label}</span>
          </Button>
        ))}
      </nav>

      <div className="sidebar__spacer" />

      <Link variant="nav" href="https://discord.gg/bRaXP9gYZN" target="_blank" rel="noreferrer" title="Discord">
        <DiscordLogoIcon weight="duotone" />
        <span className="sidebar__nav-label">Discord</span>
      </Link>
      <Link
        variant="nav"
        href="https://github.com/smyGitt/HuMidi-Roblox-Piano-Autoplayer/tree/main"
        target="_blank"
        rel="noreferrer"
        title="GitHub"
      >
        <GithubLogoIcon weight="duotone" />
        <span className="sidebar__nav-label">GitHub</span>
      </Link>
    </aside>
  );
}
