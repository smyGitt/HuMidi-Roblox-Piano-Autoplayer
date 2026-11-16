import { useEffect, useMemo, useRef, useState } from "react";
import { save as saveDialog, open as openDialog } from "@tauri-apps/plugin-dialog";
import { Modal } from "../components/Modal";
import { FileArrowDownIcon, FileArrowUpIcon, NotePencilIcon, PlusSquareIcon, TrashIcon } from "@phosphor-icons/react";
import { useTheme } from "../theme/ThemeProvider";
import {
  THEMES,
  COLOR_FIELD_GROUPS,
  DEFAULT_IMAGE_OFFSET,
  DEFAULT_IMAGE_ZOOM,
  IMAGE_ZOOM_STEP,
  MIN_IMAGE_ZOOM,
  clampImageZoom,
  imageFields,
  applyThemeToRoot,
  completeTokens,
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
  readImageDataUrl,
  type CustomThemeColors,
} from "../lib/tauri";
import { Button } from "../components/Button";
import { ColorInput, Select, TextInput } from "../components/Field";
import { DraftNumberInput } from "../components/DraftNumberInput";

const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "svg"];

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
    const payload: CustomThemeColors = { name: theme.name, ...theme.tokens, ...imageFields(theme), builtin };
    await saveCustomTheme(payload);
    await refreshCustomThemes();
  }

  async function changeCurrent(patch: Partial<Omit<Theme, "name">>) {
    const changed = { tokens: current.tokens, image: current.image, imageZoom: current.imageZoom, imageX: current.imageX, imageY: current.imageY, ...patch };
    if (isBuiltin) {
      const candidate = uniqueCopyName(current.name, allNames);
      const forked: Theme = { name: candidate, ...changed };
      await persist(forked);
      setCurrent(forked);
      setDirty(false);
      return;
    }
    setCurrent({ name: current.name, ...changed });
    setDirty(true);
  }

  async function handleColorChange(field: keyof ThemeTokens, value: string) {
    await changeCurrent({ tokens: { ...current.tokens, [field]: value } });
  }

  async function handlePickImage() {
    const path = await openDialog({ multiple: false, filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }] });
    if (!path || typeof path !== "string") return;
    try {
      const image = await readImageDataUrl(path);
      await changeCurrent({ image });
    } catch (e) {
      window.alert(`Could not use that image: ${String(e)}`);
    }
  }

  async function handleClearImage() {
    await changeCurrent({ image: undefined });
  }

  async function handleImageNumber(value: number, apply: (value: number) => Partial<Omit<Theme, "name">>) {
    await changeCurrent(apply(value));
  }

  async function handleNew() {
    const candidate = uniqueCopyName(current.name, allNames);
    const forked: Theme = { ...current, name: candidate };
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
    const renamed: Theme = { ...current, name: newName };
    if (isTauri()) {
      await deleteCustomTheme(current.name);
      await saveCustomTheme({ name: newName, ...renamed.tokens, ...imageFields(renamed), builtin: false });
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
      await exportThemeFile(path, { name: current.name, ...current.tokens, ...imageFields(current), builtin: false });
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
      const { name: _n, builtin: _b, image, image_scale, image_x, image_y, ...tokens } = imported;
      const theme: Theme = {
        name: candidate,
        tokens: completeTokens(tokens),
        image: image || undefined,
        imageZoom: image_scale,
        imageX: image_x,
        imageY: image_y,
      };
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
          <Button variant="icon" subtle title="New (duplicate current)" onClick={() => void handleNew()}>
            <PlusSquareIcon weight="fill" />
          </Button>
          <Button
            variant="icon" subtle
            title="Rename"
            disabled={isBuiltin}
            onClick={() => void handleRename()}
          >
            <NotePencilIcon weight="fill" />
          </Button>
          <Button
            variant="icon" subtle
            title="Delete"
            disabled={isBuiltin}
            onClick={() => void handleDelete()}
          >
            <TrashIcon weight="fill" />
          </Button>
          <Button variant="icon" subtle title="Export" onClick={() => void handleExport()}>
            <FileArrowDownIcon weight="fill" />
          </Button>
          <Button variant="icon" subtle title="Import" onClick={() => void handleImport()}>
            <FileArrowUpIcon weight="fill" />
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

        <div className="theme-dialog__group">
          <span className="theme-dialog__group-title">Image</span>
          <div className="theme-dialog__image-row">
            <Button onClick={() => void handlePickImage()}>Choose Image</Button>
            <Button disabled={!current.image} onClick={() => void handleClearImage()}>
              Clear
            </Button>
          </div>
          <div className="theme-dialog__image-fields">
            <label className="theme-dialog__image-field">
              <span>Zoom</span>
              <DraftNumberInput
                aria-label="Zoom"
                min={MIN_IMAGE_ZOOM}
                step={IMAGE_ZOOM_STEP}
                value={current.imageZoom ?? DEFAULT_IMAGE_ZOOM}
                disabled={!current.image}
                onCommit={(v) => void handleImageNumber(v, (n) => ({ imageZoom: clampImageZoom(n) }))}
              />
              <span>x</span>
            </label>
            <label className="theme-dialog__image-field">
              <span>X</span>
              <DraftNumberInput
                aria-label="X offset"
                step={1}
                value={current.imageX ?? DEFAULT_IMAGE_OFFSET}
                disabled={!current.image}
                onCommit={(v) => void handleImageNumber(v, (n) => ({ imageX: Math.round(n) }))}
              />
            </label>
            <label className="theme-dialog__image-field">
              <span>Y</span>
              <DraftNumberInput
                aria-label="Y offset"
                step={1}
                value={current.imageY ?? DEFAULT_IMAGE_OFFSET}
                disabled={!current.image}
                onCommit={(v) => void handleImageNumber(v, (n) => ({ imageY: Math.round(n) }))}
              />
            </label>
          </div>
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
