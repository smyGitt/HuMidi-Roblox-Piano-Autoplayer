import { useCallback, useEffect, useMemo, useState } from "react";
import { TabPage } from "../../components/TabPage";
import { LoadedRow } from "./LoadedRow";
import { SavedSongsPanel } from "./SavedSongsPanel";
import { PerformanceCard } from "./PerformanceCard";
import { OptionsCard } from "./OptionsCard";
import { TempoCard } from "./TempoCard";
import { PedalAiCard } from "./PedalAiCard";
import { HumanizeMasterRow } from "./HumanizeMasterRow";
import { HumRow } from "./HumRow";
import { ApplyToast } from "./ApplyToast";
import { Card } from "../../components/Card";
import { LoadSaveDialog, type SaveEntry } from "../../dialogs/LoadSaveDialog";
import { usePlaybackConfig } from "../../state/PlaybackConfigContext";
import { usePlaybackEngine } from "../../state/PlaybackEngineContext";
import { useLog } from "../../state/LogContext";
import { isTauri, getSaveDir, listSaves, renameSave, deleteSave, type SaveSummary } from "../../lib/tauri";
import { Checkbox } from "../../components/Field";

const TABS = ["File", "Playback", "Humanize"] as const;

function toSaveEntry(s: SaveSummary): SaveEntry {
  return {
    filepath: s.path,
    sourceMidiName: s.song_name,
    saveName: s.filename.replace(/\.json$/i, ""),
    createdAt: s.created,
    tempo: s.tempo,
    pedalStyle: s.pedal_style,
    use88Key: s.use_88_key_layout,
    humanization: s.humanization,
  };
}

interface PlaybackTabProps {
  onEditTrackSelection: () => void;
}

export function PlaybackTab({ onEditTrackSelection }: PlaybackTabProps) {
  const [subTab, setSubTab] = useState(0);
  const { config, updateConfig, setConfig } = usePlaybackConfig();
  const engine = usePlaybackEngine();
  const { appendLog } = useLog();

  const [toastVisible, setToastVisible] = useState(false);
  const [savedInitial, setSavedInitial] = useState(config);
  const [saves, setSaves] = useState<SaveSummary[]>([]);
  const [loadDialogOpen, setLoadDialogOpen] = useState(false);

  const refreshSaves = useCallback(async () => {
    if (!isTauri()) return;
    try {
      const dir = await getSaveDir();
      setSaves(dir ? await listSaves(dir) : []);
    } catch {
      setSaves([]);
    }
  }, []);

  useEffect(() => {
    void refreshSaves();
  }, [refreshSaves]);

  useEffect(() => {
    if (loadDialogOpen) void refreshSaves();
  }, [loadDialogOpen, refreshSaves]);

  async function handleRenameSave(filepath: string, newName: string) {
    try {
      await renameSave(filepath, newName);
      await refreshSaves();
    } catch (e) {
      appendLog(`Failed to rename save: ${String(e)}`);
    }
  }

  async function handleDeleteSave(filepath: string) {
    try {
      await deleteSave(filepath);
      await refreshSaves();
    } catch (e) {
      appendLog(`Failed to delete save: ${String(e)}`);
    }
  }

  function trackedUpdateConfig(patch: Parameters<typeof updateConfig>[0]) {
    updateConfig(patch);
    if (engine.fileName) setToastVisible(true);
  }

  const humanizeAll = useMemo(
    () =>
      config.vary_timing &&
      config.vary_articulation &&
      config.simulate_hands &&
      config.enable_chord_roll &&
      config.enable_tempo_sway &&
      config.enable_drift_correction &&
      config.enable_mistakes,
    [config],
  );

  function toggleAll(v: boolean) {
    trackedUpdateConfig({
      simulate_hands: v,
      enable_chord_roll: v,
      vary_timing: v,
      vary_articulation: v,
      enable_tempo_sway: v,
      enable_drift_correction: v,
      enable_mistakes: v,
    });
  }

  return (
    <div className="playback-tab">
      <TabPage
        tabs={TABS}
        active={subTab}
        onChange={setSubTab}
        overlay={
          subTab === 0 && (
            <SavedSongsPanel
              saves={saves.map((s) => ({
                filepath: s.path,
                saveName: s.filename.replace(/\.json$/i, ""),
                songName: s.song_name,
                timeStr: s.last_accessed,
              }))}
              onRefresh={() => void refreshSaves()}
              onOpenAll={() => setLoadDialogOpen(true)}
              onSaveClick={() => setLoadDialogOpen(true)}
            />
          )
        }
      >          {subTab === 0 && (            <div className="playback-tab__file-page">              <LoadedRow                parts={engine.parts}                pedalCount={engine.pedalIntervals.length}                onEditSelection={onEditTrackSelection}                editEnabled={engine.tracks.length > 0}              />            </div>          )}          {subTab === 1 && (            <div className="playback-tab__playback-page">              <div className="playback-tab__two-col">                <PerformanceCard                  config={config}                  onChange={trackedUpdateConfig}                  onReset={() => trackedUpdateConfig({ transpose: 0, pedal_style: "ai", use_velocity_accent: false })}                  midiPedalAvailable={false}                />                <OptionsCard                  config={config}                  onChange={trackedUpdateConfig}                  onReset={() => trackedUpdateConfig({ use_88_key_layout: false, countdown: true, debug_mode: false })}                />              </div>              <TempoCard                tempo={config.tempo}                originalBpm={engine.originalBpm}                onTempoChange={(v) => trackedUpdateConfig({ tempo: v })}              />              <PedalAiCard                hasThresholds={engine.hasCompiledPedal}                generateEnabled={engine.hasCompiledNotes}                thresholdOn={config.pedal_threshold_on < 0 ? 0.5 : config.pedal_threshold_on}                thresholdOff={config.pedal_threshold_off < 0 ? 0.5 : config.pedal_threshold_off}                stats={engine.aiStats}                onGenerate={engine.generatePedal}                onThresholdChange={(on, off) => trackedUpdateConfig({ pedal_threshold_on: on, pedal_threshold_off: off })}                onReset={() => {                  const [on, off] = engine.defaultAiThresholds ?? [0.5, 0.5];                  trackedUpdateConfig({ pedal_threshold_on: on, pedal_threshold_off: off });                }}              />            </div>          )}          {subTab === 2 && (            <div className="playback-tab__humanize-page">              <HumanizeMasterRow                humanizeAll={humanizeAll}                simulateHands={config.simulate_hands}                chordRoll={config.enable_chord_roll}                onHumanizeAllChange={toggleAll}                onSimulateHandsChange={(v) => trackedUpdateConfig({ simulate_hands: v })}                onChordRollChange={(v) => trackedUpdateConfig({ enable_chord_roll: v })}                onReset={() => trackedUpdateConfig({ simulate_hands: false, enable_chord_roll: false })}              />              <Card title="TIMING & FEEL">                <HumRow                  name="Vary Timing"                  desc="randomize note-start jitter"                  checked={config.vary_timing}                  onCheckedChange={(v) => trackedUpdateConfig({ vary_timing: v })}                  min={0}                  max={0.1}                  value={config.timing_variance}                  suffix=" s"                  decimals={3}                  onValueChange={(v) => trackedUpdateConfig({ timing_variance: v })}                />                <HumRow                  name="Vary Articulation"                  desc="randomize note-hold length"                  checked={config.vary_articulation}                  onCheckedChange={(v) => trackedUpdateConfig({ vary_articulation: v })}                  min={50}                  max={100}                  value={config.articulation}                  suffix="%"                  decimals={1}                  onValueChange={(v) => trackedUpdateConfig({ articulation: v })}                />                <HumRow                  name="Tempo Sway"                  desc="gentle tempo rubato over time"                  checked={config.enable_tempo_sway}                  onCheckedChange={(v) => trackedUpdateConfig({ enable_tempo_sway: v })}                  min={0}                  max={0.1}                  value={config.tempo_sway_intensity}                  suffix=" s"                  decimals={3}                  onValueChange={(v) => trackedUpdateConfig({ tempo_sway_intensity: v })}                />                <div className="check-pair">                  <label className="check-pair__inline">                    <Checkbox                      disabled={!config.enable_tempo_sway}                      checked={config.invert_tempo_sway}                      onChange={(e) => trackedUpdateConfig({ invert_tempo_sway: e.target.checked })}                    />                    Invert Sway                  </label>                  <span className="check-pair__desc">flip the sway curve phase</span>                </div>              </Card>              <Card title="HANDS & IMPERFECTION">                <HumRow                  name="Hand Drift"                  desc="gradual left/right timing drift"                  checked={config.enable_drift_correction}                  onCheckedChange={(v) => trackedUpdateConfig({ enable_drift_correction: v })}                  min={0}                  max={100}                  value={config.drift_decay_factor}                  suffix="%"                  decimals={1}                  onValueChange={(v) => trackedUpdateConfig({ drift_decay_factor: v })}                />                <HumRow                  name="Mistake Chance"                  desc="occasional dropped or mistimed notes"                  checked={config.enable_mistakes}                  onCheckedChange={(v) => trackedUpdateConfig({ enable_mistakes: v })}                  min={0}                  max={10}                  value={config.mistake_chance}                  suffix="%"                  decimals={1}                  onValueChange={(v) => trackedUpdateConfig({ mistake_chance: v })}                />              </Card>            </div>          )}
      </TabPage>

      <ApplyToast
        visible={toastVisible}
        onApply={() => {
          setSavedInitial(config);
          setToastVisible(false);
        }}
        onDiscard={() => {
          setConfig(savedInitial);
          setToastVisible(false);
        }}
      />

      {loadDialogOpen && (
        <LoadSaveDialog
          saves={saves.map(toSaveEntry)}
          onCancel={() => setLoadDialogOpen(false)}
          onLoad={(save) => {
            trackedUpdateConfig({ tempo: save.tempo, use_88_key_layout: save.use88Key });
            void engine.resumeSave(save.filepath, save.saveName);
            setLoadDialogOpen(false);
          }}
          onRename={(filepath, newName) => void handleRenameSave(filepath, newName)}
          onDelete={(filepath) => void handleDeleteSave(filepath)}
        />
      )}
    </div>
  );
}
