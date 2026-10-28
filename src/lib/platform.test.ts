import { afterEach, describe, expect, it, vi } from "vitest";
import { isLinux } from "./platform";

const WEBKITGTK_ON_LINUX =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15";
const WEBVIEW2_ON_WINDOWS =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0";
const WKWEBVIEW_ON_MACOS =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15";

function userAgentIs(value: string) {
  vi.spyOn(window.navigator, "userAgent", "get").mockReturnValue(value);
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("isLinux", () => {
  it("is true for the Linux web view", () => {
    userAgentIs(WEBKITGTK_ON_LINUX);
    expect(isLinux()).toBe(true);
  });

  it("is false for the Windows web view", () => {
    userAgentIs(WEBVIEW2_ON_WINDOWS);
    expect(isLinux()).toBe(false);
  });

  it("is false for the macOS web view", () => {
    userAgentIs(WKWEBVIEW_ON_MACOS);
    expect(isLinux()).toBe(false);
  });

  it("is false when there is no user agent", () => {
    userAgentIs("");
    expect(isLinux()).toBe(false);
  });
});
