import { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { Card } from "../../components/Card";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { SliderSpinbox } from "../../components/SliderSpinbox";
import { ThemeDialog } from "../../dialogs/ThemeDialog";
import { useTheme } from "../../theme/ThemeProvider";
import { useAppSettings } from "../../state/AppSettingsContext";
import { usePlaybackConfig } from "../../state/PlaybackConfigContext";
import { useLog } from "../../state/LogContext";
import { DEFAULT_CONFIG } from "../playback/types";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  isTauri,
  onEvent,
  startBinding,
  startSaveBinding,
  setSaveDir,
  getSaveDir,
  setMidiDir,
  getMidiDir,
  getThemesFile,
  setThemesDir,
  checkForUpdatesNow,
  getAppVersion,
  type UpdateCheckOutcome,
} from "../../lib/tauri";
import { Button } from "../../components/Button";
import { TabPage } from "../../components/TabPage";
import { Select, TextInput } from "../../components/Field";
import { Label } from "../../components/Label";

const NAV_ITEMS = ["Display", "Files", "Hotkey", "System", "Privacy"] as const;
type SettingsNav = (typeof NAV_ITEMS)[number];

function DisplayPage() {
  const settings = useAppSettings();
  const { themeName, setThemeName, themeNames } = useTheme();
  const [themeDialogOpen, setThemeDialogOpen] = useState(false);

  return (
    <div className="settings-tab__display">
      <Card title="Window">
        <div className="control-row">
          <ToggleSwitch checked={settings.alwaysOnTop} onChange={settings.setAlwaysOnTop} label="Always on Top" />
        </div>
        <div className="control-row">
          <div className="control-row__label">
            <Label>Opacity</Label>
          </div>
          <SliderSpinbox min={20} max={100} value={settings.opacity} suffix="%" onChange={settings.setOpacity} />
        </div>
        <div className="control-row">
          <ToggleSwitch checked={settings.showStatusText} onChange={settings.setShowStatusText} label="Show Status Text" />
        </div>
      </Card>

      <Card title="Visualizer">
        <div className="control-row">
          <ToggleSwitch checked={settings.showTimeline} onChange={settings.setShowTimeline} label="Show Timeline" />
        </div>
        <div className="control-row">
          <ToggleSwitch checked={settings.showPiano} onChange={settings.setShowPiano} label="Show Piano" />
        </div>
        <div className="control-row">
          <ToggleSwitch
            checked={settings.showPianoPedal}
            onChange={settings.setShowPianoPedal}
            label="Show Piano Pedal"
          />
        </div>
      </Card>

      <Card
        title="Appearance"
        footer={
          <Button onClick={() => setThemeDialogOpen(true)}>
            Customize...
          </Button>
        }
      >
        <div className="control-row">
          <div className="control-row__label">
            <Label>Theme</Label>
          </div>
          <Select value={themeName} onChange={(e) => setThemeName(e.target.value)}>
            {themeNames.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </Select>
        </div>
      </Card>

      {themeDialogOpen && <ThemeDialog onClose={() => setThemeDialogOpen(false)} />}
    </div>
  );
}

function DirectoryRow({
  title,
  hint,
  path,
  onBrowse,
}: {
  title: string;
  hint: string;
  path: string;
  onBrowse: () => void;
}) {
  return (
    <Card title={title}>
      <div className="control-row">
        <TextInput className="settings-tab__path-input" readOnly value={path || "Not set"} />
        <Button
          onClick={() => void openPath(path)}
          disabled={!isTauri() || !path}
          title="Open in file explorer"
        >
          Open
        </Button>
        <Button onClick={onBrowse} disabled={!isTauri()}>
          Browse...
        </Button>
      </div>
      <span className="page-placeholder__hint">{hint}</span>
    </Card>
  );
}

function FilesPage() {
  const [saveDir, setSaveDirState] = useState("");
  const [midiDir, setMidiDirState] = useState("");
  const [themesFile, setThemesFile] = useState("");

  useEffect(() => {
    if (!isTauri()) return;
    getSaveDir().then(setSaveDirState).catch(() => {});
    getMidiDir().then(setMidiDirState).catch(() => {});
    getThemesFile()
      .then(setThemesFile)
      .catch(() => {});
  }, []);

  async function browseSaveDir() {
    if (!isTauri()) return;
    const selected = await openDialog({ directory: true, multiple: false, defaultPath: saveDir || undefined });
    if (typeof selected === "string") {
      await setSaveDir(selected);
      setSaveDirState(selected);
    }
  }

  async function browseMidiDir() {
    if (!isTauri()) return;
    const selected = await openDialog({ directory: true, multiple: false, defaultPath: midiDir || undefined });
    if (typeof selected === "string") {
      await setMidiDir(selected);
      setMidiDirState(selected);
    }
  }

  async function browseThemesDir() {
    if (!isTauri()) return;
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string") {
      const newPath = await setThemesDir(selected);
      setThemesFile(newPath);
    }
  }

  return (
    <div className="settings-tab__display">
      <DirectoryRow
        title="Save Directory"
        hint='Where "Save Playback" writes practice-run files.'
        path={saveDir}
        onBrowse={browseSaveDir}
      />
      <DirectoryRow
        title="MIDI Directory"
        hint="Default directory the MIDI file picker opens to."
        path={midiDir}
        onBrowse={browseMidiDir}
      />
      <DirectoryRow
        title="Themes File"
        hint="JSON file where custom themes are stored."
        path={themesFile}
        onBrowse={browseThemesDir}
      />
    </div>
  );
}

function HotkeyPage() {
  const [playbackLabel, setPlaybackLabel] = useState("F6");
  const [saveLabel, setSaveLabel] = useState("Ctrl+S");
  const [listeningPlayback, setListeningPlayback] = useState(false);
  const [listeningSave, setListeningSave] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    const unlisten = [
      onEvent("hotkey_bound_updated", (label) => {
        setPlaybackLabel(label);
        setListeningPlayback(false);
      }),
      onEvent("hotkey_bound_save_updated", (label) => {
        setSaveLabel(label);
        setListeningSave(false);
      }),
    ];
    return () => {
      unlisten.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  return (
    <div className="settings-tab__display">
      <Card title="Playback Toggle">
        <div className="control-row">
          <Label className="control-row__label">{listeningPlayback ? "Listening..." : playbackLabel}</Label>
          <Button
            disabled={!isTauri() || listeningPlayback}
            onClick={() => {
              setListeningPlayback(true);
              void startBinding();
            }}
          >
            Change
          </Button>
        </div>
      </Card>
      <Card title="Save Playback">
        <div className="control-row">
          <Label className="control-row__label">{listeningSave ? "Listening..." : saveLabel}</Label>
          <Button
            disabled={!isTauri() || listeningSave}
            onClick={() => {
              setListeningSave(true);
              void startSaveBinding();
            }}
          >
            Change
          </Button>
        </div>
      </Card>
    </div>
  );
}

function SystemPage() {
  const settings = useAppSettings();
  const { setConfig } = usePlaybackConfig();
  const [checking, setChecking] = useState(false);
  const [checkResult, setCheckResult] = useState<UpdateCheckOutcome | null>(null);
  const [appVersion, setAppVersion] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    getAppVersion()
      .then(setAppVersion)
      .catch(() => {});
  }, []);

  async function handleCheckNow() {
    if (!isTauri()) return;
    setChecking(true);
    try {
      setCheckResult(await checkForUpdatesNow());
    } catch {
      setCheckResult({ status: "indeterminate" });
    } finally {
      setChecking(false);
    }
  }

  return (
    <div className="settings-tab__display">
      {appVersion && (
        <Card title="">
          <Label className="settings-tab__version">App version: {appVersion}</Label>
        </Card>
      )}
      <Card title="Updates">
        <div className="control-row">
          <ToggleSwitch
            checked={settings.autoCheckUpdates}
            onChange={settings.setAutoCheckUpdates}
            label="Automatically check for updates"
          />
        </div>
        <div className="control-row">
          <Button disabled={checking} onClick={() => void handleCheckNow()}>
            {checking ? "Checking..." : "Check Now"}
          </Button>
          {checkResult?.status === "update_available" && (
            <Button variant="accent" onClick={() => void openUrl(checkResult.url)}>
              Update available ({checkResult.tag}): Open
            </Button>
          )}
          {checkResult?.status === "no_update" && <span>Up to date.</span>}
          {checkResult?.status === "indeterminate" && <span>Couldn't check for updates.</span>}
        </div>
      </Card>
      <Card title="MIDI Import">
        <div className="control-row">
          <div className="control-row__label">
            <Label>Pedal Prompt Threshold</Label>
          </div>
          <SliderSpinbox
            min={1}
            max={200}
            value={settings.pedalPromptThreshold}
            onChange={settings.setPedalPromptThreshold}
          />
        </div>
        <span className="page-placeholder__hint">
          Minimum number of embedded MIDI sustain-pedal events before prompting whether to use them directly
          instead of generating new pedal events.
        </span>
      </Card>
      <Card title="Reset">
        <Button
          onClick={() =>
            setConfig((c) => ({
              ...DEFAULT_CONFIG,
              pedal_threshold_on: c.pedal_threshold_on,
              pedal_threshold_off: c.pedal_threshold_off,
            }))
          }
        >
          Reset All Settings
        </Button>
        <span className="page-placeholder__hint">
          Restore all playback and humanization settings to their defaults. Pedal AI thresholds have their own
          Reset button on the Playback tab and are not affected here.
        </span>
      </Card>
    </div>
  );
}

function PrivacyPage() {
  const { redactPaths, setRedactPaths } = useLog();
  return (
    <div className="settings-tab__display">
      <Card title="Debug Log">
        <div className="control-row">
          <ToggleSwitch checked={redactPaths} onChange={setRedactPaths} label="Redact file paths" />
        </div>
        <span className="page-placeholder__hint">Collapses absolute file paths in the Debug log to just their filename.</span>
      </Card>
    </div>
  );
}

export function SettingsTab() {
  const [nav, setNav] = useState<SettingsNav>("Display");

  return (
    <div className="settings-tab">
      <TabPage
        tabs={NAV_ITEMS}
        active={NAV_ITEMS.indexOf(nav)}
        onChange={(i) => setNav(NAV_ITEMS[i])}
      >
        {nav === "Display" && <DisplayPage />}
        {nav === "Files" && <FilesPage />}
        {nav === "Hotkey" && <HotkeyPage />}
        {nav === "System" && <SystemPage />}
        {nav === "Privacy" && <PrivacyPage />}
      </TabPage>
    </div>
  );
}
