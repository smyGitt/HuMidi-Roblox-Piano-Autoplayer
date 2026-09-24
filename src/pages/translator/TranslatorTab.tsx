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

const FORMATS = ["Virtual Piano"];
const MODES = ["Import", "Export"] as const;

export function TranslatorTab() {
  const [mode, setMode] = useState<"import" | "export">("import");
  const [format, setFormat] = useState(FORMATS[0]);
  const [importText, setImportText] = useState("");
  const [bpm, setBpm] = useState(120);
  const [humanize, setHumanize] = useState(false);
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
        title="Translator"
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
                min={20}
                max={400}
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
