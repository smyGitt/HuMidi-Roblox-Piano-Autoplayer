import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CustomThemeColors } from "../lib/tauri";
import { ThemeProvider, useTheme } from "../theme/ThemeProvider";
import { ThemeDialog } from "./ThemeDialog";

const CUSTOM_NAME = "Picture";
const IMAGE = "data:image/png;base64,AAAA";
const NEW_IMAGE = "data:image/png;base64,BBBB";
const IMAGE_PATH = "C:/pics/new.png";

const tauriMocks = vi.hoisted(() => ({
  isTauri: vi.fn(() => true),
  getActiveThemeName: vi.fn<() => Promise<string | null>>(),
  setActiveThemeName: vi.fn(() => Promise.resolve()),
  getCustomThemes: vi.fn<() => Promise<CustomThemeColors[]>>(),
  saveCustomTheme: vi.fn<(theme: CustomThemeColors) => Promise<void>>(() => Promise.resolve()),
  deleteCustomTheme: vi.fn(() => Promise.resolve()),
  exportThemeFile: vi.fn<(path: string, theme: CustomThemeColors) => Promise<void>>(() => Promise.resolve()),
  importThemeFile: vi.fn<(path: string) => Promise<CustomThemeColors>>(),
  readImageDataUrl: vi.fn<(path: string) => Promise<string>>(),
}));

const dialogMocks = vi.hoisted(() => ({
  open: vi.fn<(options: unknown) => Promise<string | null>>(),
  save: vi.fn<(options: unknown) => Promise<string | null>>(),
}));

vi.mock("../lib/tauri", async () => {
  const actual = await vi.importActual<typeof import("../lib/tauri")>("../lib/tauri");
  return { ...actual, ...tauriMocks };
});
vi.mock("@tauri-apps/plugin-dialog", () => dialogMocks);

function customTheme(image: string, imageScale = 1, imageX = 0, imageY = 0): CustomThemeColors {
  return {
    name: CUSTOM_NAME,
    bg_primary: "#000000",
    bg_surface: "#111111",
    bg_input: "#222222",
    text_primary: "#ffffff",
    text_muted: "#aaaaaa",
    border: "#333333",
    accent: "#ff8800",
    accent_play: "#00ff00",
    accent_stop: "#ff0000",
    pedal_color: "#ffaa00",
    accent_loaded: "#ffff00",
    knob_color: "#ffffff",
    viz_note_left: "#010101",
    viz_note_right: "#020202",
    viz_note_other: "#030303",
    viz_progress_line: "#040404",
    viz_measure_line: "#050505",
    viz_key_white: "#060606",
    viz_key_black: "#070707",
    viz_key_pressed: "#080808",
    image,
    image_scale: imageScale,
    image_x: imageX,
    image_y: imageY,
    builtin: false,
  };
}

function AfterThemeLoads({ name }: { name: string }) {
  const { themeName } = useTheme();
  return themeName === name ? <ThemeDialog onClose={vi.fn()} /> : null;
}

async function openEditor(activeName: string, customs: CustomThemeColors[]) {
  tauriMocks.getActiveThemeName.mockResolvedValue(activeName);
  tauriMocks.getCustomThemes.mockResolvedValue(customs);
  render(
    <ThemeProvider>
      <AfterThemeLoads name={activeName} />
    </ThemeProvider>,
  );
  await waitFor(() => expect(screen.getByRole("combobox")).toBeTruthy());
  await waitFor(() => expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe(activeName));
}

beforeEach(() => {
  tauriMocks.isTauri.mockReturnValue(true);
  tauriMocks.saveCustomTheme.mockClear();
  tauriMocks.exportThemeFile.mockClear();
  tauriMocks.readImageDataUrl.mockReset().mockResolvedValue(NEW_IMAGE);
  dialogMocks.open.mockReset().mockResolvedValue(IMAGE_PATH);
  dialogMocks.save.mockReset().mockResolvedValue("C:/out/theme.json");
  vi.spyOn(window, "alert").mockImplementation(() => {});
});

afterEach(() => {
  vi.restoreAllMocks();
});

function lastSaved(): CustomThemeColors {
  const calls = tauriMocks.saveCustomTheme.mock.calls;
  return calls[calls.length - 1][0];
}

describe("ThemeDialog image picker", () => {
  it("has an Image group with a Choose Image button and a Clear button", async () => {
    await openEditor(CUSTOM_NAME, [customTheme("")]);
    const title = screen.getByText("Image", { selector: ".theme-dialog__group-title" });
    const group = title.closest(".theme-dialog__group") as HTMLElement;
    expect(group.querySelector("button")?.textContent).toBe("Choose Image");
    expect([...group.querySelectorAll("button")].map((button) => button.textContent)).toEqual(["Choose Image", "Clear"]);
  });

  it("asks for an image file, reads it, and keeps it on the custom theme until Save", async () => {
    await openEditor(CUSTOM_NAME, [customTheme("")]);
    fireEvent.click(screen.getByText("Choose Image"));
    await waitFor(() => expect(tauriMocks.readImageDataUrl).toHaveBeenCalledWith(IMAGE_PATH));
    expect(dialogMocks.open).toHaveBeenCalledWith({
      multiple: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg"] }],
    });
    expect(tauriMocks.saveCustomTheme).not.toHaveBeenCalled();
    const save = screen.getByText("Save") as HTMLButtonElement;
    await waitFor(() => expect(save.disabled).toBe(false));
    fireEvent.click(save);
    await waitFor(() => expect(lastSaved().image).toBe(NEW_IMAGE));
    expect(lastSaved().name).toBe(CUSTOM_NAME);
  });

  it("does nothing when the file dialog is cancelled", async () => {
    dialogMocks.open.mockResolvedValue(null);
    await openEditor(CUSTOM_NAME, [customTheme("")]);
    fireEvent.click(screen.getByText("Choose Image"));
    await waitFor(() => expect(dialogMocks.open).toHaveBeenCalled());
    expect(tauriMocks.readImageDataUrl).not.toHaveBeenCalled();
    expect((screen.getByText("Save") as HTMLButtonElement).disabled).toBe(true);
  });

  it("tells the user when the image cannot be used and keeps the theme as it was", async () => {
    tauriMocks.readImageDataUrl.mockRejectedValue("That image is too large.");
    await openEditor(CUSTOM_NAME, [customTheme("")]);
    fireEvent.click(screen.getByText("Choose Image"));
    await waitFor(() => expect(window.alert).toHaveBeenCalledWith("Could not use that image: That image is too large."));
    expect((screen.getByText("Save") as HTMLButtonElement).disabled).toBe(true);
  });

  it("Clear removes the image, and is disabled while there is none", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    const clear = screen.getByText("Clear") as HTMLButtonElement;
    expect(clear.disabled).toBe(false);
    fireEvent.click(clear);
    await waitFor(() => expect((screen.getByText("Clear") as HTMLButtonElement).disabled).toBe(true));
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() => expect(lastSaved().image).toBe(""));
  });

  it("Revert brings the saved image back", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    fireEvent.click(screen.getByText("Clear"));
    await waitFor(() => expect((screen.getByText("Clear") as HTMLButtonElement).disabled).toBe(true));
    fireEvent.click(screen.getByText("Revert"));
    await waitFor(() => expect((screen.getByText("Clear") as HTMLButtonElement).disabled).toBe(false));
  });

  it("on a built-in theme, choosing an image makes a custom copy that carries it", async () => {
    await openEditor("Midnight", []);
    fireEvent.click(screen.getByText("Choose Image"));
    await waitFor(() => expect(tauriMocks.saveCustomTheme).toHaveBeenCalled());
    expect(lastSaved().name).toBe("Midnight Copy");
    expect(lastSaved().image).toBe(NEW_IMAGE);
    expect(lastSaved().accent).not.toBe("");
  });

  it("keeps the image when a colour is changed on the custom theme", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    const hex = screen.getAllByRole("textbox")[0] as HTMLInputElement;
    fireEvent.change(hex, { target: { value: "#123456" } });
    fireEvent.keyDown(hex, { key: "Enter" });
    await waitFor(() => expect((screen.getByText("Save") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() => expect(lastSaved().image).toBe(IMAGE));
    expect(lastSaved().bg_primary).toBe("#123456");
  });

  it("New (duplicate) copies the image", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    fireEvent.click(screen.getByTitle("New (duplicate current)"));
    await waitFor(() => expect(tauriMocks.saveCustomTheme).toHaveBeenCalled());
    expect(lastSaved().name).toBe("Picture Copy");
    expect(lastSaved().image).toBe(IMAGE);
  });

  it("Rename keeps the image", async () => {
    vi.spyOn(window, "prompt").mockReturnValue("Renamed");
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    fireEvent.click(screen.getByTitle("Rename"));
    await waitFor(() => expect(tauriMocks.saveCustomTheme).toHaveBeenCalled());
    expect(lastSaved().name).toBe("Renamed");
    expect(lastSaved().image).toBe(IMAGE);
  });

  it("Export writes the image into the theme file", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    fireEvent.click(screen.getByTitle("Export"));
    await waitFor(() => expect(tauriMocks.exportThemeFile).toHaveBeenCalled());
    expect(tauriMocks.exportThemeFile.mock.calls[0][1].image).toBe(IMAGE);
  });

  it("Import keeps the image from the file", async () => {
    dialogMocks.open.mockResolvedValue("C:/in/theme.json");
    tauriMocks.importThemeFile.mockResolvedValue({ ...customTheme(IMAGE), name: "Fresh" });
    await openEditor(CUSTOM_NAME, [customTheme("")]);
    fireEvent.click(screen.getByTitle("Import"));
    await waitFor(() => expect(tauriMocks.saveCustomTheme).toHaveBeenCalled());
    expect(lastSaved().name).toBe("Fresh");
    expect(lastSaved().image).toBe(IMAGE);
  });
});

describe("ThemeDialog image zoom and offsets", () => {
  function group(): HTMLElement {
    return screen.getByText("Image", { selector: ".theme-dialog__group-title" }).closest(".theme-dialog__group") as HTMLElement;
  }
  function box(label: string): HTMLInputElement {
    return screen.getByLabelText(label) as HTMLInputElement;
  }
  function change(label: string, value: string) {
    fireEvent.change(box(label), { target: { value } });
  }
  async function saveAndRead(): Promise<CustomThemeColors> {
    await waitFor(() => expect((screen.getByText("Save") as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByText("Save"));
    await waitFor(() => expect(tauriMocks.saveCustomTheme).toHaveBeenCalled());
    return lastSaved();
  }

  it("has Zoom, X and Y boxes in the Image group: zoom in steps of 0.1 from 0.1 with no maximum, offsets in whole numbers", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    const zoom = box("Zoom");
    expect(group().contains(zoom)).toBe(true);
    expect(zoom.min).toBe("0.1");
    expect(zoom.step).toBe("0.1");
    expect(zoom.hasAttribute("max")).toBe(false);
    expect(zoom.value).toBe("1");
    expect(group().textContent).toContain("x");
    for (const label of ["X offset", "Y offset"]) {
      expect(group().contains(box(label))).toBe(true);
      expect(box(label).value).toBe("0");
      expect(box(label).step).toBe("1");
      expect(box(label).hasAttribute("min")).toBe(false);
      expect(box(label).hasAttribute("max")).toBe(false);
    }
  });

  it("has no slider", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    expect(group().querySelector("input[type='range']")).toBeNull();
  });

  it("shows the theme's saved zoom and offsets", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 2.5, -12, 7)]);
    expect(box("Zoom").value).toBe("2.5");
    expect(box("X offset").value).toBe("-12");
    expect(box("Y offset").value).toBe("7");
  });

  it("disables all three while the theme has no image", async () => {
    await openEditor(CUSTOM_NAME, [customTheme("")]);
    for (const label of ["Zoom", "X offset", "Y offset"]) expect(box(label).disabled).toBe(true);
  });

  it("changing them waits for Save, then saves them with the theme and its image", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    change("Zoom", "3.5");
    change("X offset", "-40");
    change("Y offset", "25");
    expect(tauriMocks.saveCustomTheme).not.toHaveBeenCalled();
    const saved = await saveAndRead();
    expect([saved.image_scale, saved.image_x, saved.image_y, saved.image]).toEqual([3.5, -40, 25, IMAGE]);
  });

  it("has no upper limit on zoom, and keeps it to one decimal", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    change("Zoom", "1234.56");
    expect((await saveAndRead()).image_scale).toBe(1234.6);
  });

  it("rounds 6.74 to 6.7", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    change("Zoom", "6.74");
    expect((await saveAndRead()).image_scale).toBe(6.7);
  });

  it("never goes below 0.1x", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 2)]);
    change("Zoom", "0");
    expect((await saveAndRead()).image_scale).toBe(0.1);
  });

  it("keeps offsets whole", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE)]);
    change("X offset", "3.6");
    change("Y offset", "-2.4");
    const saved = await saveAndRead();
    expect([saved.image_x, saved.image_y]).toEqual([4, -2]);
  });

  it("lets each box be emptied, with nothing put in it and nothing changed or saved", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 2, 5, 6)]);
    for (const label of ["Zoom", "X offset", "Y offset"]) {
      fireEvent.focus(box(label));
      change(label, "");
      expect(box(label).value).toBe("");
    }
    expect((screen.getByText("Save") as HTMLButtonElement).disabled).toBe(true);
    expect(tauriMocks.saveCustomTheme).not.toHaveBeenCalled();
  });

  it("shows the real value again when an emptied box loses focus", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 2, 5, 6)]);
    fireEvent.focus(box("X offset"));
    change("X offset", "");
    fireEvent.blur(box("X offset"));
    expect(box("X offset").value).toBe("5");
  });

  it("types a new offset after clearing the old one with no 0 in front, and a minus sign can come first", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 1, 100, 100)]);
    fireEvent.focus(box("X offset"));
    change("X offset", "");
    change("X offset", "1");
    change("X offset", "12");
    expect(box("X offset").value).toBe("12");
    fireEvent.focus(box("Y offset"));
    change("Y offset", "");
    change("Y offset", "-30");
    expect(box("Y offset").value).toBe("-30");
    const saved = await saveAndRead();
    expect([saved.image_x, saved.image_y]).toEqual([12, -30]);
  });

  it("lets a zoom of 0.5 be typed even though 0 alone is below the minimum", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 2)]);
    fireEvent.focus(box("Zoom"));
    change("Zoom", "");
    change("Zoom", "0");
    expect(box("Zoom").value).toBe("0");
    change("Zoom", "0.5");
    expect(box("Zoom").value).toBe("0.5");
    expect((await saveAndRead()).image_scale).toBe(0.5);
  });

  it("Revert brings all three back", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 2, 5, 6)]);
    change("Zoom", "9");
    change("X offset", "50");
    change("Y offset", "60");
    await waitFor(() => expect(box("Zoom").value).toBe("9"));
    fireEvent.click(screen.getByText("Revert"));
    await waitFor(() => expect([box("Zoom").value, box("X offset").value, box("Y offset").value]).toEqual(["2", "5", "6"]));
  });

  it("keep them when a different image is chosen", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 3, 5, 6)]);
    fireEvent.click(screen.getByText("Choose Image"));
    const saved = await saveAndRead();
    expect([saved.image, saved.image_scale, saved.image_x, saved.image_y]).toEqual([NEW_IMAGE, 3, 5, 6]);
  });

  it("keep them when a colour is changed", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 3, 5, 6)]);
    const hex = screen.getAllByRole("textbox")[0] as HTMLInputElement;
    fireEvent.change(hex, { target: { value: "#123456" } });
    fireEvent.keyDown(hex, { key: "Enter" });
    const saved = await saveAndRead();
    expect([saved.bg_primary, saved.image_scale, saved.image_x, saved.image_y]).toEqual(["#123456", 3, 5, 6]);
  });

  it("on a built-in theme, choosing an image makes a custom copy that has the default zoom and offsets", async () => {
    await openEditor("Midnight", []);
    expect(box("Zoom").disabled).toBe(true);
    fireEvent.click(screen.getByText("Choose Image"));
    await waitFor(() => expect(tauriMocks.saveCustomTheme).toHaveBeenCalled());
    expect([lastSaved().name, lastSaved().image_scale, lastSaved().image_x, lastSaved().image_y]).toEqual(["Midnight Copy", 1, 0, 0]);
  });

  it("New (duplicate) carries them", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 3, 5, 6)]);
    fireEvent.click(screen.getByTitle("New (duplicate current)"));
    await waitFor(() => expect(lastSaved().name).toBe("Picture Copy"));
    expect([lastSaved().image_scale, lastSaved().image_x, lastSaved().image_y]).toEqual([3, 5, 6]);
  });

  it("Rename keeps them", async () => {
    vi.spyOn(window, "prompt").mockReturnValue("Renamed");
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 3, 5, 6)]);
    fireEvent.click(screen.getByTitle("Rename"));
    await waitFor(() => expect(lastSaved().name).toBe("Renamed"));
    expect([lastSaved().image_scale, lastSaved().image_x, lastSaved().image_y]).toEqual([3, 5, 6]);
  });

  it("Export writes them into the theme file", async () => {
    await openEditor(CUSTOM_NAME, [customTheme(IMAGE, 3, 5, 6)]);
    fireEvent.click(screen.getByTitle("Export"));
    await waitFor(() => expect(tauriMocks.exportThemeFile).toHaveBeenCalled());
    const exported = tauriMocks.exportThemeFile.mock.calls[0][1];
    expect([exported.image_scale, exported.image_x, exported.image_y]).toEqual([3, 5, 6]);
  });

  it("Import keeps them from the file", async () => {
    dialogMocks.open.mockResolvedValue("C:/in/theme.json");
    tauriMocks.importThemeFile.mockResolvedValue({ ...customTheme(IMAGE, 2.5, -8, 9), name: "Fresh" });
    await openEditor(CUSTOM_NAME, [customTheme("")]);
    fireEvent.click(screen.getByTitle("Import"));
    await waitFor(() => expect(lastSaved().name).toBe("Fresh"));
    expect([lastSaved().image_scale, lastSaved().image_x, lastSaved().image_y]).toEqual([2.5, -8, 9]);
  });
});
