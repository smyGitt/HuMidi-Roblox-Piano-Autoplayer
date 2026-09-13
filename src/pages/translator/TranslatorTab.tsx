import { useState } from "react";
import { Card } from "../../components/Card";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { usePlaybackConfig } from "../../state/PlaybackConfigContext";
import { usePlaybackEngine } from "../../state/PlaybackEngineContext";
import { useLog } from "../../state/LogContext";
import { isTauri, translateSheetToNotes, notesToSheet } from "../../lib/tauri";

const FORMATS = ["Virtual Piano"];

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
      <div className="translator-tab__toolbar">
        <select className="control-row__select" value={format} onChange={(e) => setFormat(e.target.value)}>
          {FORMATS.map((f) => (
            <option key={f}>{f}</option>
          ))}
        </select>
        <div className="sub-tab-bar sub-tab-bar--compact">
          <button
            className={`sub-tab-bar__btn${mode === "import" ? " sub-tab-bar__btn--active" : ""}`}
            onClick={() => setMode("import")}
          >
            Import
          </button>
          <button
            className={`sub-tab-bar__btn${mode === "export" ? " sub-tab-bar__btn--active" : ""}`}
            onClick={() => setMode("export")}
          >
            Export
          </button>
        </div>
      </div>

      {mode === "import" ? (
        <div className="translator-tab__workspace">
          <div className="playback-tab__two-col">
            <Card title="Source" className="translator-tab__card">
              <textarea
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
              <input
                type="number"
                min={20}
                max={400}
                value={bpm}
                onChange={(e) => setBpm(Number(e.target.value))}
              />
            </label>
            <ToggleSwitch checked={humanize} onChange={setHumanize} label="Humanize" />
            <button className="translator-tab__play-btn" disabled={!importText.trim()} onClick={playSheet}>
              Play Sheet
            </button>
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
              <textarea className="translator-tab__textarea" readOnly value={exportText} />
              {exportStatus && <span className="translator-tab__status">{exportStatus}</span>}
            </Card>
          </div>
          <div className="translator-tab__action-bar">
            <button className="translator-tab__play-btn" onClick={generateSheet}>
              Generate Sheet
            </button>
            <button
              className="translator-tab__play-btn"
              disabled={!exportText}
              onClick={() => navigator.clipboard.writeText(exportText)}
            >
              Copy
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
