import { LinuxLogoIcon, TriangleIcon, WindowsLogoIcon } from "@phosphor-icons/react";
import { render, screen } from "@testing-library/react";
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
  });

  it("shows only the Linux logo when it works only there", () => {
    render(<PlatformIcons windows={false} linux />);
    expect(screen.queryByLabelText("Works on Windows")).toBeNull();
    expect(screen.getByLabelText("Works on Linux")).toBeTruthy();
  });

  it("shows a triangle instead of the Linux logo when it only partly works on Linux", () => {
    render(<PlatformIcons windows linux="partial" />);
    expect(screen.getByLabelText("Works on Windows")).toBeTruthy();
    expect(screen.getByLabelText("Partly works on Linux")).toBeTruthy();
    expect(screen.queryByLabelText("Works on Linux")).toBeNull();
  });

  it("keeps its empty space when it works on neither", () => {
    const { container } = render(<PlatformIcons windows={false} linux={false} />);
    const slot = container.firstChild as HTMLElement;
    expect(slot.className).toBe("platform-icons");
    expect(slot.querySelector("svg")).toBeNull();
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
        <PlatformIcons windows={false} linux="partial" />
      </>,
    ).container;
    const paths = (html: string) => [...html.matchAll(/ d="([^"]+)"/g)].map((match) => match[1]);
    const expectedPaths = paths(expected);
    const actualPaths = paths(actual.innerHTML);
    expect(actualPaths).toEqual([expectedPaths[0], expectedPaths[1], expectedPaths[2]]);
  });
});
