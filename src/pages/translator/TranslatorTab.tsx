import { useState } from "react";
import { Card } from "../../components/Card";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { usePlaybackConfig } from "../../state/PlaybackConfigContext";
import { usePlaybackEngine } from "../../state/PlaybackEngineContext";
import { useLog } from "../../state/LogContext";
import { isTauri, translateSheetToNotes, notesToSheet } from "../../lib/tauri";
import { Button } from "../../components/Button";
import { TabPage } from "../../components/TabPage";
import { NumberInput, Select, TextArea } from "../../components/Field";
import { usePersistedState } from "../../state/usePersistedState";
import { parseBoolean, parseNumberInRange, parseOneOf, parseString } from "../../state/persistedParsers";

const FORMATS: readonly string[] = ["Virtual Piano"];
const DEFAULT_BPM = 120;
const MIN_BPM = 20;
const MAX_BPM = 400;
const MODES = ["Import", "Export"] as const;

export function TranslatorTab() {
  const [mode, setMode] = usePersistedState<"import" | "export">("ui_translator_mode", "import", {
    parse: parseOneOf(["import", "export"] as const),
  });
  const [format, setFormat] = usePersistedState("ui_translator_format", FORMATS[0], { parse: parseOneOf(FORMATS) });
  const [importText, setImportText] = usePersistedState("ui_translator_import_text", "", { parse: parseString });
  const [bpm, setBpm] = usePersistedState("ui_translator_bpm", DEFAULT_BPM, {
    parse: parseNumberInRange(MIN_BPM, MAX_BPM),
  });
  const [humanize, setHumanize] = usePersistedState("ui_translator_humanize", false, { parse: parseBoolean });
  const [previewText, setPreviewText] = useState("");
  const [exportText, setExportText] = useState("");
  const [exportStatus, setExportStatus] = useState<string | null>(null);
  const { config } = usePlaybackConfig();
  const engine = usePlaybackEngine();
  const { appendLog } = useLog();

  async function playSheet() {
    if (!isTauri()) {
      setPreviewText("Not available outside the desktop app.");
      return;
    }
    try {
      const notes = await translateSheetToNotes(importText, bpm, config.use_88_key_layout);
      setPreviewText(`Parsed ${notes.length} note(s) at ${bpm} BPM.`);
      appendLog(`Translated sheet: ${notes.length} notes successful`);
      await engine.playTranslatedSheet(importText, bpm);
    } catch (e) {
      appendLog(`Failed to translate sheet: ${String(e)}`);
    }
  }

  async function generateSheet() {
    if (!isTauri()) {
      setExportStatus("Not available outside the desktop app.");
      return;
    }
    if (engine.finalNotes.length === 0) {
      setExportStatus("Load and compile a MIDI file on the Playback tab first.");
      return;
    }
    try {
      const sheet = await notesToSheet(
        engine.finalNotes,
        config.use_88_key_layout,
        engine.tempoEvents,
        engine.timeSignatures,
      );
      setExportText(sheet);
      const lineCount = sheet.trim() ? sheet.trim().split("\n").length : 0;
      setExportStatus(`Generated ${lineCount} line(s).`);
    } catch (e) {
      setExportStatus(`Failed: ${String(e)}`);
    }
  }

  return (
    <div className="translator-tab">
      <TabPage
        tabs={MODES}
        active={mode === "import" ? 0 : 1}
        onChange={(i) => setMode(i === 0 ? "import" : "export")}
        lead={
          <div className="translator-tab__toolbar">
            <Select value={format} onChange={(e) => setFormat(e.target.value)}>
              {FORMATS.map((f) => (
                <option key={f}>{f}</option>
              ))}
            </Select>
          </div>
        }
      >
      {mode === "import" ? (
        <div className="translator-tab__workspace">
          <div className="playback-tab__two-col">
            <Card title="Source" className="translator-tab__card">
              <TextArea
                className="translator-tab__textarea"
                placeholder="Paste a sheet here..."
                value={importText}
                onChange={(e) => setImportText(e.target.value)}
              />
            </Card>
            <Card title="Preview" className="translator-tab__card">
              {previewText ? <span>{previewText}</span> : <span className="page-placeholder__hint">Not built yet.</span>}
            </Card>
          </div>
          <div className="translator-tab__action-bar">
            <label className="translator-tab__bpm">
              BPM
              <NumberInput
                min={MIN_BPM}
                max={MAX_BPM}
                value={bpm}
                onChange={(e) => setBpm(Number(e.target.value))}
              />
            </label>
            <ToggleSwitch checked={humanize} onChange={setHumanize} label="Humanize" />
            <Button variant="accent" disabled={!importText.trim()} onClick={playSheet}>
              Play Sheet
            </Button>
          </div>
        </div>
      ) : (
        <div className="translator-tab__workspace">
          <div className="playback-tab__two-col">
            <Card title="Source" className="translator-tab__card">
              {engine.fileName ? (
                <span>{engine.fileName} — {engine.finalNotes.length} note(s) compiled</span>
              ) : (
                <span className="page-placeholder__hint">Not built yet.</span>
              )}
            </Card>
            <Card title="Output" className="translator-tab__card">
              <TextArea className="translator-tab__textarea" readOnly value={exportText} />
              {exportStatus && <span className="translator-tab__status">{exportStatus}</span>}
            </Card>
          </div>
          <div className="translator-tab__action-bar">
            <Button variant="accent" onClick={generateSheet}>
              Generate Sheet
            </Button>
            <Button
              variant="accent"
              disabled={!exportText}
              onClick={() => navigator.clipboard.writeText(exportText)}
            >
              Copy
            </Button>
          </div>
        </div>
      )}
      </TabPage>
    </div>
  );
}
