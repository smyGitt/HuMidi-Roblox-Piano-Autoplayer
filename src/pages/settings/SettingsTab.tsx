import { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { Card } from "../../components/Card";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { SliderSpinbox } from "../../components/SliderSpinbox";
import { ThemeDialog } from "../../dialogs/ThemeDialog";
import { useTheme } from "../../theme/ThemeProvider";
import { MAX_VISIBLE_SAVES, MIN_VISIBLE_SAVES, useAppSettings } from "../../state/AppSettingsContext";
import { usePlaybackConfig } from "../../state/PlaybackConfigContext";
import { useLog } from "../../state/LogContext";
import { DEFAULT_CONFIG } from "../playback/types";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  isTauri,
  onEvent,
  startBinding,
  startSaveBinding,
  getHotkeys,
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
import { NumberInput, Select, TextInput } from "../../components/Field";
import { Label } from "../../components/Label";
import { InfoTip } from "../../components/InfoTip";
import { PlatformIcons } from "../../components/PlatformIcons";
import { SETTING_PLATFORMS, type SettingId } from "./settingsPlatforms";
import { isLinux } from "../../lib/platform";
import { usePersistedState } from "../../state/usePersistedState";
import { parseOneOf } from "../../state/persistedParsers";

const KEYBOARD_ACCESS_DESCRIPTION =
  "HuMidi reads your keyboard directly, so hotkeys work in every window, even when another app has focus. This also means HuMidi can see every key you press.";
const PEDAL_PROMPT_DESCRIPTION =
  "Minimum number of embedded MIDI sustain-pedal events before prompting whether to use them directly instead of generating new pedal events.";
const RESET_DESCRIPTION =
  "Restore all playback and humanization settings to their defaults. Pedal AI thresholds have their own Reset button on the Playback tab and are not affected here.";
const REDACT_PATHS_DESCRIPTION = "Collapses absolute file paths in the Debug log to just their filename.";

const NAV_ITEMS = ["Display", "Files", "Hotkey", "System", "Privacy"] as const;
type SettingsNav = (typeof NAV_ITEMS)[number];

function Platforms({ id }: { id: SettingId }) {
  return <PlatformIcons {...SETTING_PLATFORMS[id]} />;
}

function DisplayPage() {
  const settings = useAppSettings();
  const { themeName, setThemeName, themeNames } = useTheme();
  const [themeDialogOpen, setThemeDialogOpen] = useState(false);

  return (
    <div className="settings-tab__display">
      <Card title="Window">
        <div className="control-row">
          <ToggleSwitch checked={settings.alwaysOnTop} onChange={settings.setAlwaysOnTop} label="Always on Top" />
          <Platforms id="alwaysOnTop" />
        </div>
        <div className="control-row">
          <div className="control-row__label">
            <Label>Opacity</Label>
          </div>
          <SliderSpinbox min={20} max={100} value={settings.opacity} suffix="%" onChange={settings.setOpacity} />
          <Platforms id="opacity" />
        </div>
        <div className="control-row">
          <ToggleSwitch checked={settings.showStatusText} onChange={settings.setShowStatusText} label="Show Status Text" />
          <Platforms id="showStatusText" />
        </div>
      </Card>

      <Card title="Visualizer">
        <div className="control-row">
          <ToggleSwitch checked={settings.showTimeline} onChange={settings.setShowTimeline} label="Show Timeline" />
          <Platforms id="showTimeline" />
        </div>
        <div className="control-row">
          <ToggleSwitch
            checked={settings.visualizerHorizontal}
            onChange={settings.setVisualizerHorizontal}
            label="Horizontal Scrolling"
          />
          <Platforms id="visualizerHorizontal" />
        </div>
        <div className="control-row">
          <ToggleSwitch checked={settings.showPiano} onChange={settings.setShowPiano} label="Show Piano" />
          <Platforms id="showPiano" />
        </div>
        <div className="control-row">
          <ToggleSwitch
            checked={settings.showPianoPedal}
            onChange={settings.setShowPianoPedal}
            label="Show Piano Pedal"
          />
          <Platforms id="showPianoPedal" />
        </div>
        <div className="control-row">
          <ToggleSwitch
            checked={settings.switchToVisualizerOnPlay}
            onChange={settings.setSwitchToVisualizerOnPlay}
            label="Switch to Visualizer on Play"
          />
          <Platforms id="switchToVisualizerOnPlay" />
        </div>
      </Card>

      <Card title="Saved Songs">
        <div className="control-row">
          <div className="control-row__label">
            <Label>Max visible saves</Label>
          </div>
          <NumberInput
            min={MIN_VISIBLE_SAVES}
            max={MAX_VISIBLE_SAVES}
            value={settings.maxVisibleSaves}
            onChange={(e) => {
              const v = Number(e.target.value);
              if (!Number.isNaN(v)) settings.setMaxVisibleSaves(v);
            }}
          />
          <Platforms id="maxVisibleSaves" />
        </div>
      </Card>

      <Card title="Appearance">
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
          <Button onClick={() => setThemeDialogOpen(true)}>Customize...</Button>
          <Platforms id="theme" />
        </div>
      </Card>

      {themeDialogOpen && <ThemeDialog onClose={() => setThemeDialogOpen(false)} />}
    </div>
  );
}

function DirectoryRow({
  id,
  title,
  hint,
  path,
  onBrowse,
}: {
  id: SettingId;
  title: string;
  hint: string;
  path: string;
  onBrowse: () => void;
}) {
  return (
    <Card title={title} titleButtons={<InfoTip text={hint} />}>
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
        <Platforms id={id} />
      </div>
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
        id="saveDirectory"
        title="Save Directory"
        hint='Where "Save Playback" writes practice-run files.'
        path={saveDir}
        onBrowse={browseSaveDir}
      />
      <DirectoryRow
        id="midiDirectory"
        title="MIDI Directory"
        hint="Default directory the MIDI file picker opens to."
        path={midiDir}
        onBrowse={browseMidiDir}
      />
      <DirectoryRow
        id="themesFile"
        title="Themes File"
        hint="JSON file where custom themes are stored."
        path={themesFile}
        onBrowse={browseThemesDir}
      />
    </div>
  );
}

function HotkeyPage() {
  const settings = useAppSettings();
  const [playbackLabel, setPlaybackLabel] = useState("F6");
  const [saveLabel, setSaveLabel] = useState("Ctrl+S");
  const [listeningPlayback, setListeningPlayback] = useState(false);
  const [listeningSave, setListeningSave] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;
    getHotkeys()
      .then(([playback, save]) => {
        setPlaybackLabel(playback);
        setSaveLabel(save);
      })
      .catch(() => {});
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
          <Platforms id="playbackHotkey" />
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
          <Platforms id="saveHotkey" />
        </div>
      </Card>
      {isLinux() && (
        <Card title="Keyboard access (Linux)">
          <div className="control-row">
            <ToggleSwitch
              checked={settings.linuxKeyboardHotkeys}
              onChange={settings.setLinuxKeyboardHotkeys}
              label="Always listen to keys (Linux)"
            />
            <InfoTip text={KEYBOARD_ACCESS_DESCRIPTION} />
            <Platforms id="linuxKeyboardHotkeys" />
          </div>
        </Card>
      )}
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
          <Platforms id="autoCheckUpdates" />
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
          <Platforms id="checkNow" />
        </div>
      </Card>
      <Card title="MIDI Import">
        <div className="control-row">
          <div className="control-row__label">
            <Label>Pedal Prompt Threshold</Label>
            <InfoTip text={PEDAL_PROMPT_DESCRIPTION} />
          </div>
          <SliderSpinbox
            min={1}
            max={200}
            value={settings.pedalPromptThreshold}
            onChange={settings.setPedalPromptThreshold}
          />
          <Platforms id="pedalPromptThreshold" />
        </div>
      </Card>
      <Card title="Reset">
        <div className="control-row">
          <Button
            onClick={() =>
              setConfig((c) => ({
                ...DEFAULT_CONFIG,
                pedal_curve_on: c.pedal_curve_on,
                pedal_curve_off: c.pedal_curve_off,
              }))
            }
          >
            Reset All Settings
          </Button>
          <InfoTip text={RESET_DESCRIPTION} />
          <Platforms id="resetAll" />
        </div>
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
          <ToggleSwitch
            checked={redactPaths}
            onChange={setRedactPaths}
            label="Redact file paths"
          />
          <InfoTip text={REDACT_PATHS_DESCRIPTION} />
          <Platforms id="redactPaths" />
        </div>
      </Card>
    </div>
  );
}

export function SettingsTab() {
  const [nav, setNav] = usePersistedState<SettingsNav>("ui_settings_page", "Display", {
    parse: parseOneOf(NAV_ITEMS),
  });

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
