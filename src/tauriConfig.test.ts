import { describe, expect, it } from "vitest";

const capabilities = JSON.parse(
  Object.values(
    import.meta.glob<string>("/src-tauri/capabilities/default.json", { query: "?raw", import: "default", eager: true }),
  )[0],
) as { permissions: (string | { identifier: string; allow?: { path?: string }[] })[] };

const config = JSON.parse(
  Object.values(import.meta.glob<string>("/src-tauri/tauri.conf.json", { query: "?raw", import: "default", eager: true }))[0],
) as { plugins?: { opener?: { requireLiteralLeadingDot?: boolean } } };

describe("main window capability", () => {
  it("lets the Open buttons in Settings open any folder the user has chosen", () => {
    const grant = capabilities.permissions.find(
      (permission) => typeof permission === "object" && permission.identifier === "opener:allow-open-path",
    ) as { allow?: { path?: string }[] } | undefined;
    expect(grant).toBeDefined();
    expect(grant!.allow).toEqual([{ path: "**" }]);
  });

  it("keeps the opener default, so web links and revealing a file still work", () => {
    expect(capabilities.permissions).toContain("opener:default");
  });

  it("keeps the permissions the app already used", () => {
    for (const permission of ["core:default", "core:window:allow-set-always-on-top", "core:window:allow-set-size", "dialog:default", "global-shortcut:default"]) {
      expect(capabilities.permissions).toContain(permission);
    }
  });
});

describe("opener plugin config", () => {
  it("matches folders whose names start with a dot, like ~/.humidi on Linux and macOS", () => {
    expect(config.plugins?.opener?.requireLiteralLeadingDot).toBe(false);
  });
});
