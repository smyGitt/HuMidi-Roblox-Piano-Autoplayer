import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SidePanel } from "./SidePanel";

describe("SidePanel", () => {
  it("is an aside with the handle icon and the content", () => {
    const { container } = render(
      <SidePanel>
        <span>inside</span>
      </SidePanel>,
    );
    const panel = container.firstElementChild as HTMLElement;
    expect(panel.tagName).toBe("ASIDE");
    expect(panel.className).toBe("side-panel");
    expect(panel.querySelector(".side-panel__handle svg")).not.toBeNull();
    expect(panel.querySelector(".side-panel__content")?.textContent).toBe("inside");
  });
});
