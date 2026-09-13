import { useEffect, useState } from "react";
import { ThemeProvider } from "./theme/ThemeProvider";
import { AppSettingsProvider, useAppSettings } from "./state/AppSettingsContext";
import { PlaybackConfigProvider } from "./state/PlaybackConfigContext";
import { LogProvider } from "./state/LogContext";
import { PlaybackEngineProvider, usePlaybackEngine } from "./state/PlaybackEngineContext";
import { Sidebar } from "./components/Sidebar";
import { TransportBar } from "./components/TransportBar";
import { PlaceholderPage } from "./pages/PlaceholderPage";
import { PlaybackTab } from "./pages/playback/PlaybackTab";
import { VisualizerTab } from "./pages/visualizer/VisualizerTab";
import { SettingsTab } from "./pages/settings/SettingsTab";
import { TranslatorTab } from "./pages/translator/TranslatorTab";
import { DebugTab } from "./pages/debug/DebugTab";
import { LicenseTab } from "./pages/license/LicenseTab";
import type { PageId } from "./pages/pageIds";
import "./App.css";

const PAGE_TITLES: Record<PageId, string> = {
  playback: "Playback",
  visualizer: "Visualizer",
  translator: "Translator",
  settings: "Settings",
  debug: "Debug",
  license: "About / License",
};

function AppShell() {
  const [activePage, setActivePage] = useState<PageId>("playback");
  const [isCollapsed, setIsCollapsed] = useState(false);
  const { opacity } = useAppSettings();
  const engine = usePlaybackEngine();

  useEffect(() => {
    function handleKeydown(e: KeyboardEvent) {
      if (e.ctrlKey && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setIsCollapsed((c) => !c);
      }
    }
    window.addEventListener("keydown", handleKeydown);
    return () => window.removeEventListener("keydown", handleKeydown);
  }, []);

  const status = !engine.fileName ? "unloaded" : engine.hasCompiledPedal ? "ready" : "loaded";
  const statusLabel = engine.fileName || "No file loaded";

  return (
    <div className={`app-window${isCollapsed ? " app-window--collapsed" : ""}`} style={{ opacity: opacity / 100 }}>
      {!isCollapsed && (
        <div className="app-body">
          <Sidebar activePage={activePage} onNavigate={setActivePage} status={status} statusLabel={statusLabel} />
          <div className="app-page-area">
            {activePage === "playback" ? (
              <PlaybackTab />
            ) : activePage === "visualizer" ? (
              <VisualizerTab />
            ) : activePage === "settings" ? (
              <SettingsTab />
            ) : activePage === "translator" ? (
              <TranslatorTab />
            ) : activePage === "debug" ? (
              <DebugTab />
            ) : activePage === "license" ? (
              <LicenseTab />
            ) : (
              <PlaceholderPage title={PAGE_TITLES[activePage]} />
            )}
          </div>
        </div>
      )}

      <TransportBar
        isPlaying={engine.isPlaying && !engine.isPaused}
        currentTime={engine.currentTime}
        totalTime={engine.totalDuration}
        isCollapsed={isCollapsed}
        saveEnabled={engine.hasCompiledNotes}
        onScrub={(v) => engine.seek(engine.totalDuration > 0 ? (v / 10000) * engine.totalDuration : 0)}
        onSeekCommit={() => {}}
        onPlayPause={() => {
          if (engine.isPlaying) void engine.togglePause();
          else void engine.play();
        }}
        onStop={() => void engine.stop()}
        onSave={() => void engine.save()}
        onToggleCollapsed={() => setIsCollapsed((c) => !c)}
      />
    </div>
  );
}

function App() {
  return (
    <ThemeProvider>
      <AppSettingsProvider>
        <PlaybackConfigProvider>
          <LogProvider>
            <PlaybackEngineProvider>
              <AppShell />
            </PlaybackEngineProvider>
          </LogProvider>
        </PlaybackConfigProvider>
      </AppSettingsProvider>
    </ThemeProvider>
  );
}

export default App;
