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
    expect(screen.getByText("To GitHub Releases")).toBeTruthy();
    expect(screen.getByText("To SourceForge")).toBeTruthy();
    expect(screen.getByText("Cancel")).toBeTruthy();
    expect(screen.queryByText("Update")).toBeNull();
    expect(screen.queryByText(/Updating/)).toBeNull();
  });

  it("opens the releases page from the GitHub button", () => {
    renderModal();
    fireEvent.click(screen.getByText("To GitHub Releases"));
    expect(openUrl).toHaveBeenCalledWith(RELEASES_URL);
  });

  it("opens SourceForge from the SourceForge button", () => {
    renderModal();
    fireEvent.click(screen.getByText("To SourceForge"));
    expect(openUrl).toHaveBeenCalledWith(SOURCEFORGE_URL);
  });

  it("dismisses with Cancel without opening anything", () => {
    const onDismiss = renderModal();
    fireEvent.click(screen.getByText("Cancel"));
    expect(onDismiss).toHaveBeenCalledTimes(1);
    expect(openUrl).not.toHaveBeenCalled();
  });
});
