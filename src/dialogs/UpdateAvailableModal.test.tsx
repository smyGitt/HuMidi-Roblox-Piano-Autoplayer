import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SOURCEFORGE_URL, UpdateAvailableModal } from "./UpdateAvailableModal";

const openUrl = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl }));

const RELEASES_URL = "https://example.test/releases/latest";

function renderModal(onDismiss = vi.fn()) {
  render(
    <UpdateAvailableModal currentVersion="2.1.6" latestVersion="v2.2.0" releasesUrl={RELEASES_URL} onDismiss={onDismiss} />,
  );
  return onDismiss;
}

describe("UpdateAvailableModal", () => {
  beforeEach(() => openUrl.mockReset());

  it("states the current and latest versions", () => {
    renderModal();
    expect(screen.getByText(/current: 2\.1\.6, latest: v2\.2\.0/)).toBeTruthy();
  });

  it("offers only GitHub Releases, SourceForge and Cancel, with no in-app update", () => {
    renderModal();
    expect(screen.getByText("GitHub")).toBeTruthy();
    expect(screen.getByText("SourceForge")).toBeTruthy();
    expect(screen.getByText("Cancel")).toBeTruthy();
    expect(screen.queryByText("Update")).toBeNull();
    expect(screen.queryByText(/Updating/)).toBeNull();
  });

  it("labels the buttons GitHub and SourceForge, not the long wording", () => {
    renderModal();
    expect(screen.queryByText("To GitHub Releases")).toBeNull();
    expect(screen.queryByText("To SourceForge")).toBeNull();
  });

  it("puts the GitHub logo on the GitHub button only", () => {
    renderModal();
    const github = screen.getByText("GitHub").closest("button")!;
    const sourceforge = screen.getByText("SourceForge").closest("button")!;
    const cancel = screen.getByText("Cancel").closest("button")!;
    expect(github.querySelectorAll("svg")).toHaveLength(1);
    expect(sourceforge.querySelectorAll("svg")).toHaveLength(0);
    expect(cancel.querySelectorAll("svg")).toHaveLength(0);
  });

  it("uses the standard button size instead of the large one", () => {
    renderModal();
    for (const label of ["GitHub", "SourceForge", "Cancel"]) {
      expect(screen.getByText(label).closest("button")!.className).not.toContain("btn--lg");
    }
  });

  it("is a compact modal", () => {
    renderModal();
    const modal = document.querySelector(".modal") as HTMLElement;
    expect(modal.style.width).toBe("420px");
    expect(modal.style.height).toBe("224px");
  });

  it("opens the releases page from the GitHub button", () => {
    renderModal();
    fireEvent.click(screen.getByText("GitHub"));
    expect(openUrl).toHaveBeenCalledWith(RELEASES_URL);
  });

  it("opens SourceForge from the SourceForge button", () => {
    renderModal();
    fireEvent.click(screen.getByText("SourceForge"));
    expect(openUrl).toHaveBeenCalledWith(SOURCEFORGE_URL);
  });

  it("dismisses with Cancel without opening anything", () => {
    const onDismiss = renderModal();
    fireEvent.click(screen.getByText("Cancel"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
    expect(openUrl).not.toHaveBeenCalled();
  });
});
