import { useCallback, useEffect, useMemo, useState } from "react";
import { usePersistedState } from "../../state/usePersistedState";
import { parseOneOf } from "../../state/persistedParsers";
import { TabPage } from "../../components/TabPage";
import { LoadedParts } from "./LoadedParts";
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
import { useAppSettings } from "../../state/AppSettingsContext";
import { usePlaybackEngine } from "../../state/PlaybackEngineContext";
import { useLog } from "../../state/LogContext";
import { isTauri, getSaveDir, listSaves, renameSave, deleteSave, type SaveSummary } from "../../lib/tauri";
import { Checkbox } from "../../components/Field";
import { formatSaveTime } from "../../lib/formatSaveTime";

const TABS = ["Info", "Playback", "Humanize"] as const;

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
  const [subTab, setSubTab] = usePersistedState("ui_playback_subtab", 0, {
    parse: parseOneOf(TABS.map((_, index) => index)),
  });
  const { config, updateConfig } = usePlaybackConfig();
  const engine = usePlaybackEngine();
  const { appendLog } = useLog();

  const [saves, setSaves] = useState<SaveSummary[]>([]);
  const [loadDialogOpen, setLoadDialogOpen] = useState(false);
  const [loadDialogSelected, setLoadDialogSelected] = useState<string | null>(null);
  const { maxVisibleSaves } = useAppSettings();

  function openLoadDialog(selectedFilepath: string | null) {
    setLoadDialogSelected(selectedFilepath);
    setLoadDialogOpen(true);
  }

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

  useEffect(() => {
    if (engine.saveCount > 0) void refreshSaves();
  }, [engine.saveCount, refreshSaves]);

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
    updateConfig({
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
            <LoadedParts
              parts={engine.parts}
              pedalCount={engine.pedalIntervals.length}
              onEditSelection={onEditTrackSelection}
              editEnabled={engine.tracks.length > 0}
            />
          )
        }
      >          {subTab === 0 && (            <div className="playback-tab__file-page"> <SavedSongsPanel saves={saves.slice(0, maxVisibleSaves).map((s) => ({ filepath: s.path, saveName: s.filename.replace(/\.json$/i, ""), songName: s.song_name, timeStr: formatSaveTime(s.last_accessed), }))} onRefresh={() => void refreshSaves()} onOpenAll={() => openLoadDialog(null)} onSaveClick={(save) => openLoadDialog(save.filepath)} /> </div>          )}          {subTab === 1 && (            <div className="playback-tab__playback-page">              <div className="playback-tab__two-col">                <PerformanceCard                  config={config}                  onChange={updateConfig}                  onReset={() => updateConfig({ transpose: 0, pedal_style: "ai", pedal_before_humanize: true })}                  midiPedalAvailable={false}                />                <OptionsCard                  config={config}                  onChange={updateConfig}                  onReset={() => updateConfig({ use_88_key_layout: false, countdown: true, debug_mode: false })}                />              </div>              <TempoCard                tempo={config.tempo}                originalBpm={engine.originalBpm}                onTempoChange={(v) => updateConfig({ tempo: v })}              />              <PedalAiCard
                hasThresholds={engine.hasCompiledPedal && engine.aiCurves !== null}
                confidence={engine.pedalConfidence}
                generateEnabled={engine.hasCompiledNotes}
                curves={{ on: config.pedal_curve_on, off: config.pedal_curve_off }}
                stats={engine.aiStats}
                onGenerate={engine.generatePedal}
                onCurvesChange={(curves) => updateConfig({ pedal_curve_on: curves.on, pedal_curve_off: curves.off })}
                onReset={() => {
                  const curves = engine.defaultAiCurves;
                  if (curves) updateConfig({ pedal_curve_on: curves.on, pedal_curve_off: curves.off });
                }}
              />            </div>          )}          {subTab === 2 && (            <div className="playback-tab__humanize-page">              <HumanizeMasterRow                humanizeAll={humanizeAll}                simulateHands={config.simulate_hands}                chordRoll={config.enable_chord_roll}                onHumanizeAllChange={toggleAll}                onSimulateHandsChange={(v) => updateConfig({ simulate_hands: v })}                onChordRollChange={(v) => updateConfig({ enable_chord_roll: v })}                onReset={() => updateConfig({ simulate_hands: false, enable_chord_roll: false })}              />              <Card title="TIMING & FEEL">                <HumRow                  name="Vary Timing"                  desc="notes start a little early or late, like a real player"                  checked={config.vary_timing}                  onCheckedChange={(v) => updateConfig({ vary_timing: v })}                  min={0}                  max={0.1}                  value={config.timing_variance}                  suffix=" s"                  decimals={3}                  onValueChange={(v) => updateConfig({ timing_variance: v })}                />                <HumRow                  name="Vary Articulation"                  desc="some notes get held a bit longer or shorter"                  checked={config.vary_articulation}                  onCheckedChange={(v) => updateConfig({ vary_articulation: v })}                  min={50}                  max={100}                  value={config.articulation}                  suffix="%"                  decimals={1}                  onValueChange={(v) => updateConfig({ articulation: v })}                />                <HumRow                  name="Tempo Sway"                  desc="the speed drifts a little faster and slower as the song goes"                  checked={config.enable_tempo_sway}                  onCheckedChange={(v) => updateConfig({ enable_tempo_sway: v })}                  min={0}                  max={0.1}                  value={config.tempo_sway_intensity}                  suffix=" s"                  decimals={3}                  onValueChange={(v) => updateConfig({ tempo_sway_intensity: v })}                />                <div className="check-pair">                  <label className="check-pair__inline">                    <Checkbox                      disabled={!config.enable_tempo_sway}                      checked={config.invert_tempo_sway}                      onChange={(e) => updateConfig({ invert_tempo_sway: e.target.checked })}                    />                    Invert Sway                  </label>                  <span className="check-pair__desc">makes the speed changes go the opposite way</span>                </div>              </Card>              <Card title="HANDS & IMPERFECTION">                <HumRow                  name="Hand Drift"                  desc="your hands slowly slip out of sync with each other"                  checked={config.enable_drift_correction}                  onCheckedChange={(v) => updateConfig({ enable_drift_correction: v })}                  min={0}                  max={100}                  value={config.drift_decay_factor}                  suffix="%"                  decimals={1}                  onValueChange={(v) => updateConfig({ drift_decay_factor: v })}                />                <HumRow                  name="Mistake Chance"                  desc="once in a while a note gets missed or played wrong"                  checked={config.enable_mistakes}                  onCheckedChange={(v) => updateConfig({ enable_mistakes: v })}                  min={0}                  max={10}                  value={config.mistake_chance}                  suffix="%"                  decimals={1}                  onValueChange={(v) => updateConfig({ mistake_chance: v })}                />              </Card>            </div>          )}
      </TabPage>

      <ApplyToast
        visible={engine.configDirty}
        onApply={() => void engine.applyConfigChanges()}
        onDiscard={engine.discardConfigChanges}
      />

      {loadDialogOpen && (
        <LoadSaveDialog
          saves={saves.map(toSaveEntry)}
          initialSelected={loadDialogSelected}
          onCancel={() => setLoadDialogOpen(false)}
          onLoad={(save) => {
            updateConfig({ tempo: save.tempo, use_88_key_layout: save.use88Key });
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
