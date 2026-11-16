import { XIcon } from "@phosphor-icons/react";
import { LinuxLogoIcon, TriangleIcon, WindowsLogoIcon } from "@phosphor-icons/react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PlatformIcons } from "./PlatformIcons";

describe("PlatformIcons", () => {
  it("shows both logos when the setting works on both systems", () => {
    render(<PlatformIcons windows linux />);
    expect(screen.getByLabelText("Works on Windows")).toBeTruthy();
    expect(screen.getByLabelText("Works on Linux")).toBeTruthy();
  });

  it("shows only the Windows logo when it works only there", () => {
    render(<PlatformIcons windows linux={false} />);
    expect(screen.getByLabelText("Works on Windows")).toBeTruthy();
    expect(screen.queryByLabelText("Works on Linux")).toBeNull();
    expect(screen.getByLabelText("Not supported on Linux")).toBeTruthy();
    expect(screen.queryByLabelText("Not supported on Windows")).toBeNull();
  });

  it("shows only the Linux logo when it works only there", () => {
    render(<PlatformIcons windows={false} linux />);
    expect(screen.queryByLabelText("Works on Windows")).toBeNull();
    expect(screen.getByLabelText("Works on Linux")).toBeTruthy();
    expect(screen.getByLabelText("Not supported on Windows")).toBeTruthy();
    expect(screen.queryByLabelText("Not supported on Linux")).toBeNull();
  });

  it("shows a triangle instead of the Linux logo when it only partly works on Linux", () => {
    render(<PlatformIcons windows linux="partial" />);
    expect(screen.getByLabelText("Works on Windows")).toBeTruthy();
    expect(screen.getByLabelText("Partly works on Linux")).toBeTruthy();
    expect(screen.queryByLabelText("Works on Linux")).toBeNull();
  });

  it("shows an X for both systems when it works on neither", () => {
    const { container } = render(<PlatformIcons windows={false} linux={false} />);
    const slot = container.firstChild as HTMLElement;
    expect(slot.className).toBe("platform-icons");
    expect(screen.getByLabelText("Not supported on Windows")).toBeTruthy();
    expect(screen.getByLabelText("Not supported on Linux")).toBeTruthy();
    expect(slot.querySelectorAll("svg")).toHaveLength(2);
  });

  it("is the same element whichever logos it shows, so the slot width never changes", () => {
    const { container } = render(<PlatformIcons windows={false} linux />);
    expect((container.firstChild as HTMLElement).className).toBe("platform-icons");
  });
});

describe("PlatformIcons weight", () => {
  it("draws every logo in the fill weight", () => {
    const light = render(
      <>
        <WindowsLogoIcon aria-label="Works on Windows" weight="fill" />
        <LinuxLogoIcon aria-label="Works on Linux" weight="fill" />
        <TriangleIcon aria-label="Partly works on Linux" weight="fill" />
      </>,
    );
    const expected = light.container.innerHTML;
    light.unmount();
    const actual = render(
      <>
        <PlatformIcons windows linux />
        <PlatformIcons windows linux="partial" />
      </>,
    ).container;
    const paths = (html: string) => [...html.matchAll(/ d="([^"]+)"/g)].map((match) => match[1]);
    const expectedPaths = paths(expected);
    const actualPaths = paths(actual.innerHTML);
    expect(actualPaths).toEqual([
      expectedPaths[0],
      expectedPaths[1],
      expectedPaths[0],
      expectedPaths[2],
    ]);
  });
});

function panelParts(): { heading: string; items: string[] }[] {
  const panel = screen.getByRole("tooltip");
  const headings = [...panel.querySelectorAll(".platform-icons__heading")];
  return headings.map((heading) => ({
    heading: heading.textContent ?? "",
    items: [...(heading.nextElementSibling as HTMLElement).querySelectorAll("li")].map((item) => item.textContent ?? ""),
  }));
}

describe("PlatformIcons hover panel", () => {
  const WORKS = ["Mint XFCE (X11)", "Fedora Wayland"];

  function hover(label: string) {
    const icon = screen.getByLabelText(label) as HTMLElement;
    fireEvent.mouseEnter(icon.closest(".platform-icons__linux") as HTMLElement);
  }

  it("shows which systems the Linux logo works on while it is hovered", () => {
    render(<PlatformIcons windows linux linuxWorks={WORKS} />);
    expect(screen.queryByRole("tooltip")).toBeNull();
    hover("Works on Linux");
    expect(panelParts()).toEqual([{ heading: "Tested on:", items: ["Mint XFCE (X11)", "Fedora Wayland"] }]);
  });

  it("hides the panel again when the mouse leaves", () => {
    render(<PlatformIcons windows linux linuxWorks={WORKS} />);
    hover("Works on Linux");
    fireEvent.mouseLeave(document.querySelector(".platform-icons__linux") as HTMLElement);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("shows both lists for the triangle: where it works and where it does not", () => {
    render(<PlatformIcons windows linux="partial" linuxWorks={["X11"]} linuxFails={["Wayland (Mint, Fedora)"]} />);
    hover("Partly works on Linux");
    expect(panelParts()).toEqual([
      { heading: "Tested on:", items: ["X11"] },
      { heading: "Does not work on:", items: ["Wayland (Mint, Fedora)"] },
    ]);
  });

  it("has no panel when no systems are given", () => {
    render(<PlatformIcons windows linux />);
    hover("Works on Linux");
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("gives the Windows logo no panel", () => {
    render(<PlatformIcons windows linux linuxWorks={WORKS} />);
    fireEvent.mouseEnter(screen.getByLabelText("Works on Windows"));
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("keeps the slot and the logos where they were, with the Linux logo in a hover wrapper", () => {
    const { container } = render(<PlatformIcons windows linux linuxWorks={WORKS} />);
    expect((container.firstElementChild as HTMLElement).className).toBe("platform-icons");
    expect([...(container.firstElementChild as HTMLElement).children].map((c) => c.className.split(" ")[0])).toEqual([
      "platform-icons__windows",
      "platform-icons__linux",
    ]);
  });
});

describe("PlatformIcons panel list", () => {
  it("is a real bulleted list, one item per system, under its heading", () => {
    render(<PlatformIcons windows linux linuxWorks={["A", "B", "C"]} />);
    fireEvent.mouseEnter(document.querySelector(".platform-icons__linux") as HTMLElement);
    const lists = document.querySelectorAll("[role='tooltip'] ul");
    expect(lists).toHaveLength(1);
    expect([...lists[0].children].map((child) => child.tagName)).toEqual(["LI", "LI", "LI"]);
    expect(lists[0].className).toBe("platform-icons__list");
    expect(document.querySelector("[role='tooltip']")?.tagName).toBe("DIV");
  });

  it("has no heading or list for a side with no systems", () => {
    render(<PlatformIcons windows linux linuxWorks={["A"]} />);
    fireEvent.mouseEnter(document.querySelector(".platform-icons__linux") as HTMLElement);
    expect(document.querySelectorAll(".platform-icons__heading")).toHaveLength(1);
    expect(document.querySelectorAll("[role='tooltip'] ul")).toHaveLength(1);
  });
});

describe("PlatformIcons panel near the edge of the window", () => {
  it("shifts back inside when the panel would run past the left edge", () => {
    const original = HTMLElement.prototype.getBoundingClientRect;
    Object.defineProperty(window, "innerWidth", { value: 1000, configurable: true });
    Object.defineProperty(window, "innerHeight", { value: 700, configurable: true });
    HTMLElement.prototype.getBoundingClientRect = function () {
      const isPanel = this.getAttribute("role") === "tooltip";
      return (isPanel ? { left: -30, top: 100, right: 170, bottom: 150, width: 200, height: 50, x: -30, y: 100, toJSON() {} } : original.call(this)) as DOMRect;
    };
    render(<PlatformIcons windows linux linuxWorks={["A"]} />);
    fireEvent.mouseEnter(document.querySelector(".platform-icons__linux") as HTMLElement);
    expect(screen.getByRole("tooltip").style.transform).toBe("translateX(38px)");
    HTMLElement.prototype.getBoundingClientRect = original;
  });
});

describe("PlatformIcons not supported marks", () => {
  function pathsOf(html: string): string[] {
    return [...html.matchAll(/ d="([^"]+)"/g)].map((match) => match[1]);
  }

  it("draws the X in the bold weight", () => {
    render(<PlatformIcons windows linux={false} />);
    const expected = render(<XIcon weight="bold" />);
    const mark = screen.getByLabelText("Not supported on Linux");
    expect(pathsOf(mark.outerHTML)).toEqual(pathsOf(expected.container.innerHTML));
  });

  it("puts each X in its own system's slot, so the logos stay in their places", () => {
    const { container } = render(<PlatformIcons windows={false} linux />);
    const slots = [...(container.firstElementChild as HTMLElement).children].map((child) => child.className.split(" ")[0]);
    expect(slots).toEqual(["platform-icons__windows", "platform-icons__linux"]);
    expect(screen.getByLabelText("Not supported on Windows").closest(".platform-icons__windows")).not.toBeNull();
  });

  it("never shows the X and a logo for the same system", () => {
    render(<PlatformIcons windows linux />);
    expect(screen.queryByLabelText("Not supported on Windows")).toBeNull();
    expect(screen.queryByLabelText("Not supported on Linux")).toBeNull();
  });

  it("gives the X no hover panel", () => {
    render(<PlatformIcons windows linux={false} linuxWorks={[]} linuxFails={[]} />);
    fireEvent.mouseEnter(screen.getByLabelText("Not supported on Linux").parentElement as HTMLElement);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });
});
