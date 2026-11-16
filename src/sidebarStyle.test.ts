import { describe, expect, it } from "vitest";
import { appCss, ruleBody } from "./test/appCss";

describe("left sidebar overflow", () => {
  it("scrolls vertically when its contents do not fit, and never sideways", () => {
    const body = ruleBody(".sidebar");
    expect(body).toMatch(/overflow-y:\s*auto/);
    expect(body).toMatch(/overflow-x:\s*hidden/);
    expect(body).not.toMatch(/overflow:\s*hidden/);
  });

  it("gets the thin scrollbar every element has", () => {
    const universal = appCss.slice(appCss.indexOf("\n* {"), appCss.indexOf("}", appCss.indexOf("\n* {")));
    expect(universal).toMatch(/scrollbar-width:\s*thin/);
    expect(ruleBody(".sidebar")).not.toMatch(/scrollbar-width:\s*(auto|none)/);
  });
});
