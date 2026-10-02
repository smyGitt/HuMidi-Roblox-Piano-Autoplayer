import { useEffect, useRef, useState } from "react";
import { ThemeProvider } from "./theme/ThemeProvider";
import { AppSettingsProvider, useAppSettings } from "./state/AppSettingsContext";
import { PlaybackConfigProvider, usePlaybackConfig } from "./state/PlaybackConfigContext";
import { LogProvider, useLog } from "./state/LogContext";
import { PlaybackEngineProvider, usePlaybackEngine } from "./state/PlaybackEngineContext";
import { Sidebar } from "./components/Sidebar";
import { TransportBar } from "./components/TransportBar";
import { FileHeader } from "./components/FileHeader";
import { TrackSelectionDialog, type HandRole } from "./dialogs/TrackSelectionDialog";
import { PlaceholderPage } from "./pages/PlaceholderPage";
import { PlaybackTab } from "./pages/playback/PlaybackTab";
import { VisualizerTab } from "./pages/visualizer/VisualizerTab";
import { SettingsTab } from "./pages/settings/SettingsTab";
import { TranslatorTab } from "./pages/translator/TranslatorTab";
import { DebugTab } from "./pages/debug/DebugTab";
import { LicenseTab } from "./pages/license/LicenseTab";
import { UpdateCheckPrompt } from "./dialogs/UpdateCheckPrompt";
import { UpdateAvailableModal } from "./dialogs/UpdateAvailableModal";
import { useCollapsedWindowSize } from "./lib/useCollapsedWindowSize";
import { PAGE_IDS, type PageId } from "./pages/pageIds";
import { usePersistedState } from "./state/usePersistedState";
import { parseBoolean, parseOneOf } from "./state/persistedParsers";
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
  const [activePage, setActivePage] = usePersistedState<PageId>("ui_active_page", "playback", {
    parse: parseOneOf(PAGE_IDS),
  });
  const [isCollapsed, setIsCollapsed] = usePersistedState("ui_collapsed", false, { parse: parseBoolean });
  const [trackSelectionOpen, setTrackSelectionOpen] = useState(false);
  const windowRef = useRef<HTMLDivElement>(null);
  const { appendLog } = useLog();
  useCollapsedWindowSize(isCollapsed, windowRef, appendLog);
  const {
    opacity,
    showStatusText,
    showTimeline,
    showPiano,
    switchToVisualizerOnPlay,
    showUpdatePrompt,
    resolveUpdatePrompt,
    updateAvailable,
    dismissUpdateAvailable,
  } = useAppSettings();
  const engine = usePlaybackEngine();
  const { config } = usePlaybackConfig();

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

  const playbackRunning = engine.isPlaying && !engine.isPaused;
  const visualizerAvailableRef = useRef(showTimeline || showPiano);
  visualizerAvailableRef.current = showTimeline || showPiano;
  const switchOnPlayRef = useRef(switchToVisualizerOnPlay);
  switchOnPlayRef.current = switchToVisualizerOnPlay;
  const wasRunningRef = useRef(false);

  useEffect(() => {
    if (playbackRunning && !wasRunningRef.current && switchOnPlayRef.current && visualizerAvailableRef.current) {
      setActivePage("visualizer");
    }
    wasRunningRef.current = playbackRunning;
  }, [playbackRunning, setActivePage]);

  useEffect(() => {
    if (engine.tracks.length > 0) setTrackSelectionOpen(true);
  }, [engine.tracks]);

  async function confirmTrackSelection(selection: { index: number; role: HandRole }[]) {
    await engine.confirmTrackSelection(selection);
    setTrackSelectionOpen(false);
  }

  const status = !engine.fileName
    ? "unloaded"
    : config.pedal_style === "none"
      ? "ready"
      : engine.isGeneratingPedal
        ? "loading"
        : engine.hasCompiledPedal
          ? "ready"
          : "loaded";
  const statusLabel = { unloaded: "NO MIDI", loaded: "NO PEDAL", loading: "GEN PEDAL", ready: "READY" }[status];

  return (
    <div ref={windowRef} className={`app-window${isCollapsed ? " app-window--collapsed" : ""}`} style={{ opacity: opacity / 100 }}>
      {!isCollapsed && (
        <div className="app-body">
          <Sidebar
            activePage={activePage}
            onNavigate={setActivePage}
            visualizerDisabled={!showTimeline && !showPiano}
          />
          <div className="app-page-area">
            <FileHeader />
            <div className="app-page-content">
              {activePage === "playback" ? (
                <PlaybackTab onEditTrackSelection={() => setTrackSelectionOpen(true)} />
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
        </div>
      )}

      {isCollapsed && <FileHeader />}

      <TransportBar
        isPlaying={engine.isPlaying && !engine.isPaused}
        clock={engine.clock}
        totalTime={engine.totalDuration}
        isCollapsed={isCollapsed}
        status={status}
        statusLabel={statusLabel}
        showStatusText={showStatusText}
        playEnabled={engine.hasCompiledNotes}
        onScrub={(time) => void engine.seek(time)}
        onPlayPause={() => {
          if (engine.isPlaying) void engine.togglePause();
          else void engine.play();
        }}
        onStop={() => void engine.stop()}
        onSeekStart={() => void engine.seek(0)}
        onSeekEnd={() => void engine.seek(engine.totalDuration)}
        onToggleCollapsed={() => setIsCollapsed((c) => !c)}
      />

      {trackSelectionOpen && (
        <TrackSelectionDialog
          tracks={engine.tracks.map((t) => ({
            index: t.index,
            name: t.name,
            instrument: t.instrument_name,
            noteCount: t.note_count,
            channel: t.is_drum ? 9 : 0,
          }))}
          onCancel={() => setTrackSelectionOpen(false)}
          onConfirm={confirmTrackSelection}
        />
      )}

      {showUpdatePrompt && <UpdateCheckPrompt onChoice={resolveUpdatePrompt} />}
      {updateAvailable && (
        <UpdateAvailableModal
          currentVersion={updateAvailable.currentVersion}
          latestVersion={updateAvailable.tag}
          releasesUrl={updateAvailable.url}
          onDismiss={dismissUpdateAvailable}
        />
      )}
    </div>
  );
}

function App() {
  return (
    <ThemeProvider>
      <LogProvider>
        <AppSettingsProvider>
          <PlaybackConfigProvider>
            <PlaybackEngineProvider>
              <AppShell />
            </PlaybackEngineProvider>
          </PlaybackConfigProvider>
        </AppSettingsProvider>
      </LogProvider>
    </ThemeProvider>
  );
}

export default App;
