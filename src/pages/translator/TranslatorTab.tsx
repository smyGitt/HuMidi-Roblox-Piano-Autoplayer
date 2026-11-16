import { CopySimpleIcon, FloppyDiskIcon } from "@phosphor-icons/react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { ToggleSwitch } from "../../components/ToggleSwitch";
import { usePlaybackConfig } from "../../state/PlaybackConfigContext";
import { usePlaybackEngine } from "../../state/PlaybackEngineContext";
import { useLog } from "../../state/LogContext";
import { isTauri, translateSheetToNotes, notesToSheet, writeTextFile } from "../../lib/tauri";
import { Button } from "../../components/Button";
import { TabPage } from "../../components/TabPage";
import { NumberInput, Select, TextArea } from "../../components/Field";
import { usePersistedState } from "../../state/usePersistedState";
import { parseBoolean, parseNumberInRange, parseOneOf, parseString } from "../../state/persistedParsers";

const FORMATS: readonly string[] = ["Virtual Piano"];
const DEFAULT_BPM = 120;
const MIN_BPM = 20;
const MAX_BPM = 400;
const DEFAULT_SHEET_FILE_NAME = "sheet";
const SHEET_FILE_EXTENSION = "txt";
const MIDI_EXTENSION = /\.midi?$/i;
const MODES = ["Import", "Export"] as const;

function FormatSelect({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return (
    <div className="translator-tab__format">
      <Select value={value} onChange={(e) => onChange(e.target.value)}>
        {FORMATS.map((f) => (
          <option key={f}>{f}</option>
        ))}
      </Select>
    </div>
  );
}

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
  const [exportText, setExportText] = useState("");
  const [exportStatus, setExportStatus] = useState<string | null>(null);
  const { config } = usePlaybackConfig();
  const engine = usePlaybackEngine();
  const { appendLog } = useLog();

  async function playSheet() {
    if (!isTauri()) return;
    try {
      const notes = await translateSheetToNotes(importText, bpm, config.use_88_key_layout);
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
      setExportStatus("Please load a MIDI file first");
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

  async function saveSheet() {
    const baseName = engine.fileName.replace(MIDI_EXTENSION, "") || DEFAULT_SHEET_FILE_NAME;
    try {
      const path = await saveDialog({
        defaultPath: `${baseName}.${SHEET_FILE_EXTENSION}`,
        filters: [{ name: "Text", extensions: [SHEET_FILE_EXTENSION] }],
      });
      if (typeof path !== "string") return;
      await writeTextFile(path, exportText);
      setExportStatus(`Saved to ${path}`);
    } catch (e) {
      setExportStatus(`Failed to save: ${String(e)}`);
    }
  }

  return (
    <div className="translator-tab">
      <TabPage
        tabs={MODES}
        active={mode === "import" ? 0 : 1}
        onChange={(i) => setMode(i === 0 ? "import" : "export")}
      >
      {mode === "import" ? (
        <div className="translator-tab__workspace">
          <TextArea
            className="translator-tab__textarea"
            placeholder="Paste a sheet here..."
            value={importText}
            onChange={(e) => setImportText(e.target.value)}
          />
          <div className="translator-tab__action-bar">
            <FormatSelect value={format} onChange={setFormat} />
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
            <Button
              variant="accent"
              className="translator-tab__end"
              disabled={!importText.trim()}
              onClick={playSheet}
            >
              Play Sheet
            </Button>
          </div>
        </div>
      ) : (
        <div className="translator-tab__workspace">
          <TextArea className="translator-tab__textarea" readOnly value={exportText} />
          <div className="translator-tab__action-bar">
            <FormatSelect value={format} onChange={setFormat} />
            <Button variant="accent" onClick={generateSheet}>
              Generate Sheet
            </Button>
            <Button
              variant="accent"
              size="sm"
              disabled={!exportText}
              onClick={() => navigator.clipboard.writeText(exportText)}
              aria-label="Copy"
              title="Copy"
            >
              <CopySimpleIcon weight="fill" />
            </Button>
            <Button
              variant="accent"
              size="sm"
              disabled={!exportText}
              onClick={saveSheet}
              aria-label="Save sheet"
              title="Save sheet"
            >
              <FloppyDiskIcon weight="fill" />
            </Button>
            {exportStatus && <span className="translator-tab__status">{exportStatus}</span>}
          </div>
        </div>
      )}
      </TabPage>
    </div>
  );
}
