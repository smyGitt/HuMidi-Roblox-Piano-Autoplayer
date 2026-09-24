import { useEffect, useMemo, useRef, useState } from "react";
import { save as saveDialog, open as openDialog } from "@tauri-apps/plugin-dialog";
import { Modal } from "../components/Modal";
import { FileArrowDownIcon, FileArrowUpIcon, NotePencilIcon, PlusSquareIcon, TrashIcon } from "@phosphor-icons/react";
import { useTheme } from "../theme/ThemeProvider";
import {
  THEMES,
  COLOR_FIELD_GROUPS,
  applyThemeToRoot,
  uniqueCopyName,
  type Theme,
  type ThemeTokens,
} from "../theme/tokens";
import {
  isTauri,
  saveCustomTheme,
  deleteCustomTheme,
  exportThemeFile,
  importThemeFile,
  type CustomThemeColors,
} from "../lib/tauri";
import { Button } from "../components/Button";
import { ColorInput, Select, TextInput } from "../components/Field";

interface ThemeDialogProps {
  onClose: () => void;
}

function ColorSwatch({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (hex: string) => void;
}) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);

  function commit(hex: string) {
    if (/^#[0-9a-fA-F]{6}$/.test(hex)) onChange(hex);
    else setText(value);
  }

  return (
    <div className="theme-dialog__swatch">
      <ColorInput value={value} onChange={onChange} />
      <div className="theme-dialog__swatch-info">
        <span className="theme-dialog__swatch-label">{label}</span>
        <TextInput
          className="theme-dialog__swatch-hex"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onBlur={() => commit(text)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit(text);
          }}
        />
      </div>
    </div>
  );
}

export function ThemeDialog({ onClose }: ThemeDialogProps) {
  const { theme: activeTheme, customThemes, setThemeName, refreshCustomThemes } = useTheme();

  const originalTheme = useRef(activeTheme);
  const [current, setCurrent] = useState<Theme>(activeTheme);
  const [dirty, setDirty] = useState(false);

  const builtinNames = useMemo(() => new Set(THEMES.map((t) => t.name)), []);
  const isBuiltin = builtinNames.has(current.name);
  const allNames = useMemo(
    () => new Set([...builtinNames, ...customThemes.map((t) => t.name)]),
    [builtinNames, customThemes],
  );
  const allSelectable = useMemo(() => [...THEMES, ...customThemes], [customThemes]);

  useEffect(() => {
    applyThemeToRoot(current);
    return () => applyThemeToRoot(originalTheme.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [current]);

  function selectByName(name: string) {
    const found = allSelectable.find((t) => t.name === name);
    if (found) {
      setCurrent(found);
      setDirty(false);
    }
  }

  async function persist(theme: Theme, builtin = false): Promise<void> {
    if (!isTauri()) return;
    const payload: CustomThemeColors = { name: theme.name, ...theme.tokens, builtin };
    await saveCustomTheme(payload);
    await refreshCustomThemes();
  }

  async function handleColorChange(field: keyof ThemeTokens, value: string) {
    if (isBuiltin) {
      const candidate = uniqueCopyName(current.name, allNames);
      const forked: Theme = { name: candidate, tokens: { ...current.tokens, [field]: value } };
      await persist(forked);
      setCurrent(forked);
      setDirty(false);
      return;
    }
    const updated: Theme = { name: current.name, tokens: { ...current.tokens, [field]: value } };
    setCurrent(updated);
    setDirty(true);
  }

  async function handleNew() {
    const candidate = uniqueCopyName(current.name, allNames);
    const forked: Theme = { name: candidate, tokens: current.tokens };
    await persist(forked);
    setCurrent(forked);
    setDirty(false);
  }

  async function handleDelete() {
    if (isBuiltin) return;
    if (!window.confirm(`Delete custom theme "${current.name}"?`)) return;
    if (isTauri()) await deleteCustomTheme(current.name);
    await refreshCustomThemes();
    const fallback = THEMES.find((t) => t.name === "Midnight")!;
    setCurrent(fallback);
    setDirty(false);
  }

  async function handleRename() {
    if (isBuiltin) return;
    const input = window.prompt("New theme name:", current.name);
    if (input === null) return;
    const newName = input.trim();
    if (!newName || newName === current.name) return;
    if (allNames.has(newName)) {
      window.alert(`A theme named "${newName}" already exists.`);
      return;
    }
    const renamed: Theme = { name: newName, tokens: current.tokens };
    if (isTauri()) {
      await deleteCustomTheme(current.name);
      await saveCustomTheme({ name: newName, ...renamed.tokens, builtin: false });
    }
    await refreshCustomThemes();
    setCurrent(renamed);
    setDirty(false);
  }

  async function handleSave() {
    if (isBuiltin) return;
    await persist(current);
    setDirty(false);
  }

  function handleRevert() {
    const original = customThemes.find((t) => t.name === current.name);
    if (original) setCurrent(original);
    setDirty(false);
  }

  async function handleExport() {
    if (!isTauri()) return;
    const safeName = current.name.replace(/[/\\]/g, "-");
    const path = await saveDialog({ defaultPath: `${safeName}.json`, filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path) return;
    try {
      await exportThemeFile(path, { name: current.name, ...current.tokens, builtin: false });
    } catch (e) {
      window.alert(`Export failed: ${String(e)}`);
    }
  }

  async function handleImport() {
    if (!isTauri()) return;
    const path = await openDialog({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
    if (!path || typeof path !== "string") return;
    try {
      const imported = await importThemeFile(path);
      let candidate = imported.name || "Imported Theme";
      let n = 2;
      while (allNames.has(candidate)) {
        candidate = `${imported.name || "Imported Theme"} ${n}`;
        n += 1;
      }
      const { name: _n, builtin: _b, ...tokens } = imported;
      const theme: Theme = { name: candidate, tokens: tokens as ThemeTokens };
      await persist(theme);
      setCurrent(theme);
      setDirty(false);
    } catch (e) {
      window.alert(`Import failed: ${String(e)}`);
    }
  }

  function handleCancel() {
    applyThemeToRoot(originalTheme.current);
    onClose();
  }

  function handleAccept() {
    if (dirty && !isBuiltin) void persist(current);
    setThemeName(current.name);
    onClose();
  }

  return (
    <Modal
      title="Theme Editor"
      onClose={handleCancel}
      width={640}
      height={620}
      footer={
        <>
          <Button onClick={handleCancel}>
            Cancel
          </Button>
          <Button variant="accent" onClick={handleAccept}>
            OK
          </Button>
        </>
      }
    >
      <div className="theme-dialog">
        <div className="theme-dialog__toolbar">
          <Select
            className="theme-dialog__select"
            value={current.name}
            onChange={(e) => selectByName(e.target.value)}
          >
            {allSelectable.map((t) => (
              <option key={t.name} value={t.name}>
                {t.name}
              </option>
            ))}
          </Select>
          <Button variant="icon" subtle outlined title="New (duplicate current)" onClick={() => void handleNew()}>
            <PlusSquareIcon weight="duotone" />
          </Button>
          <Button
            variant="icon" subtle outlined
            title="Rename"
            disabled={isBuiltin}
            onClick={() => void handleRename()}
          >
            <NotePencilIcon weight="duotone" />
          </Button>
          <Button
            variant="icon" subtle outlined
            title="Delete"
            disabled={isBuiltin}
            onClick={() => void handleDelete()}
          >
            <TrashIcon weight="duotone" />
          </Button>
          <Button variant="icon" subtle outlined title="Export" onClick={() => void handleExport()}>
            <FileArrowDownIcon weight="duotone" />
          </Button>
          <Button variant="icon" subtle outlined title="Import" onClick={() => void handleImport()}>
            <FileArrowUpIcon weight="duotone" />
          </Button>
        </div>

        <div className="theme-dialog__groups">
          {COLOR_FIELD_GROUPS.map((group) => (
            <div key={group.label} className="theme-dialog__group">
              <span className="theme-dialog__group-title">{group.label}</span>
              <div className="theme-dialog__group-fields">
                {group.fields.map((f) => (
                  <ColorSwatch
                    key={f.key}
                    label={f.label}
                    value={current.tokens[f.key]}
                    onChange={(hex) => void handleColorChange(f.key, hex)}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>

        <div className="theme-dialog__save-row">
          <Button disabled={!dirty || isBuiltin} onClick={() => void handleSave()}>
            Save
          </Button>
          <Button disabled={!dirty} onClick={handleRevert}>
            Revert
          </Button>
        </div>
      </div>
    </Modal>
  );
}
