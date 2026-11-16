import { render, screen } from "@testing-library/react";
import {
  ArrowsInSimpleIcon,
  ArrowsOutSimpleIcon,
  PauseIcon,
  PlayIcon,
  SkipBackIcon,
  SkipForwardIcon,
  StopIcon,
} from "@phosphor-icons/react";
import { describe, expect, it, vi } from "vitest";
import { PlaybackClock } from "../state/playbackClock";
import { TransportBar } from "./TransportBar";

function renderBar(isPlaying: boolean) {
  return render(
    <TransportBar
      isPlaying={isPlaying}
      clock={new PlaybackClock({ selfTimed: false })}
      totalTime={10}
      isCollapsed={false}
      status="ready"
      statusLabel="READY"
      showStatusText={false}
      playEnabled
      onScrub={vi.fn()}
      onPlayPause={vi.fn()}
      onStop={vi.fn()}
      onSeekStart={vi.fn()}
      onSeekEnd={vi.fn()}
      onToggleCollapsed={vi.fn()}
    />,
  );
}

function pathsOf(html: string): string[] {
  return [...html.matchAll(/ d="([^"]+)"/g)].map((match) => match[1]);
}

function expectedPaths(element: React.ReactElement): string[] {
  const { container, unmount } = render(element);
  const paths = pathsOf(container.innerHTML);
  unmount();
  return paths;
}

function buttonPaths(title: string): string[] {
  return pathsOf((screen.getByTitle(title) as HTMLElement).innerHTML);
}

describe("TransportBar icons", () => {
  it("uses the filled icons without a circle", () => {
    renderBar(false);
    expect(buttonPaths("To start")).toEqual(expectedPaths(<SkipBackIcon weight="fill" />));
    expect(buttonPaths("Play")).toEqual(expectedPaths(<PlayIcon weight="fill" />));
    expect(buttonPaths("Stop")).toEqual(expectedPaths(<StopIcon weight="fill" />));
    expect(buttonPaths("To end")).toEqual(expectedPaths(<SkipForwardIcon weight="fill" />));
  });

  it("shows the filled pause icon while playing", () => {
    renderBar(true);
    expect(buttonPaths("Pause")).toEqual(expectedPaths(<PauseIcon weight="fill" />));
  });
});

describe("TransportBar skip buttons", () => {
  it("mark only the skip buttons so they can be drawn smaller", () => {
    renderBar(false);
    expect(screen.getByTitle("To start").className).toContain("transport-bar__skip");
    expect(screen.getByTitle("To end").className).toContain("transport-bar__skip");
    expect(screen.getByTitle("Play").className).not.toContain("transport-bar__skip");
    expect(screen.getByTitle("Stop").className).not.toContain("transport-bar__skip");
  });
});

describe("TransportBar collapse toggle", () => {
  function renderCollapsible(isCollapsed: boolean) {
    return render(
      <TransportBar
        isPlaying={false}
        clock={new PlaybackClock({ selfTimed: false })}
        totalTime={10}
        isCollapsed={isCollapsed}
        status="ready"
        statusLabel="READY"
        showStatusText={false}
        playEnabled
        onScrub={vi.fn()}
        onPlayPause={vi.fn()}
        onStop={vi.fn()}
        onSeekStart={vi.fn()}
        onSeekEnd={vi.fn()}
        onToggleCollapsed={vi.fn()}
      />,
    );
  }

  it("shows the filled arrows-in icon when the bar is expanded and can collapse", () => {
    renderCollapsible(false);
    expect(buttonPaths("Collapse")).toEqual(expectedPaths(<ArrowsInSimpleIcon weight="fill" />));
  });

  it("shows the filled arrows-out icon when the bar is collapsed and can expand, without rotating it", () => {
    renderCollapsible(true);
    expect(buttonPaths("Expand")).toEqual(expectedPaths(<ArrowsOutSimpleIcon weight="fill" />));
    expect(screen.getByTitle("Expand").querySelector("svg")?.getAttribute("style")).toBeNull();
  });

  it("keeps the toggle at its current size", () => {
    renderCollapsible(false);
    expect(screen.getByTitle("Collapse").className).toContain("btn--md");
  });
});

describe("TransportBar collapse toggle color", () => {
  it("is colored like the sidebar icons, muted with the primary color on hover", () => {
    render(
      <TransportBar
        isPlaying={false}
        clock={new PlaybackClock({ selfTimed: false })}
        totalTime={10}
        isCollapsed={false}
        status="ready"
        statusLabel="READY"
        showStatusText={false}
        playEnabled
        onScrub={vi.fn()}
        onPlayPause={vi.fn()}
        onStop={vi.fn()}
        onSeekStart={vi.fn()}
        onSeekEnd={vi.fn()}
        onToggleCollapsed={vi.fn()}
      />,
    );
    expect(screen.getByTitle("Collapse").className).toContain("btn--subtle");
  });
});

describe("TransportBar image", () => {
  function renderWithImage(image: string | undefined, imageZoom?: number, imageX?: number, imageY?: number) {
    return render(
      <TransportBar
        isPlaying={false}
        clock={new PlaybackClock({ selfTimed: false })}
        totalTime={10}
        isCollapsed={false}
        status="ready"
        statusLabel="READY"
        showStatusText={false}
        playEnabled
        image={image}
        imageZoom={imageZoom}
        imageX={imageX}
        imageY={imageY}
        onScrub={vi.fn()}
        onPlayPause={vi.fn()}
        onStop={vi.fn()}
        onSeekStart={vi.fn()}
        onSeekEnd={vi.fn()}
        onToggleCollapsed={vi.fn()}
      />,
    );
  }

  it("shows no image when none is given", () => {
    renderWithImage(undefined);
    expect(document.querySelector(".transport-bar__image")).toBeNull();
  });

  it("puts the given image straight in the bar, as a background layer outside every row", () => {
    renderWithImage("data:image/png;base64,AAAA");
    const image = document.querySelector(".transport-bar__image") as HTMLImageElement;
    expect(image.getAttribute("src")).toBe("data:image/png;base64,AAAA");
    expect(image.getAttribute("alt")).toBe("");
    const bar = document.querySelector(".transport-bar") as HTMLElement;
    expect(image.parentElement).toBe(bar);
    expect(bar.firstElementChild).toBe(image);
    expect(document.querySelector(".transport-bar__side--end")?.querySelector("img")).toBeNull();
  });

  function imageVar(name: string): string {
    return (document.querySelector(".transport-bar__image") as HTMLElement).style.getPropertyValue(name);
  }

  it("scales the image by its zoom multiplier as given, so 2.5 is 2.5 and no zoom is 1", () => {
    const first = renderWithImage("data:image/png;base64,AAAA", 2.5);
    expect(imageVar("--image-zoom")).toBe("2.5");
    first.unmount();
    const second = renderWithImage("data:image/png;base64,AAAA", 6.7);
    expect(imageVar("--image-zoom")).toBe("6.7");
    second.unmount();
    renderWithImage("data:image/png;base64,AAAA");
    expect(imageVar("--image-zoom")).toBe("1");
  });

  it("moves the image by its x and y offsets in pixels, and not at all when there are none", () => {
    const first = renderWithImage("data:image/png;base64,AAAA", 1, 12, -5);
    expect(imageVar("--image-x")).toBe("12px");
    expect(imageVar("--image-y")).toBe("-5px");
    first.unmount();
    renderWithImage("data:image/png;base64,AAAA");
    expect(imageVar("--image-x")).toBe("0px");
    expect(imageVar("--image-y")).toBe("0px");
  });
});
