const MS_PER_SECOND = 1000;

export const MAX_EXTRAPOLATION_SECONDS = 0.25;

export interface FrameScheduler {
  request: (callback: () => void) => number;
  cancel: (handle: number) => void;
}

const browserFrames: FrameScheduler = {
  request: (callback) => window.requestAnimationFrame(() => callback()),
  cancel: (handle) => window.cancelAnimationFrame(handle),
};

export interface PlaybackClockOptions {
  selfTimed: boolean;
  nowMs?: () => number;
  frames?: FrameScheduler;
}

export class PlaybackClock {
  private anchorTime = 0;
  private anchorMs = 0;
  private running = false;
  private live = false;
  private durationSeconds = 0;
  private frameHandle: number | null = null;
  private readonly listeners = new Set<() => void>();
  private readonly selfTimed: boolean;
  private readonly nowMs: () => number;
  private readonly frames: FrameScheduler;

  constructor({ selfTimed, nowMs = () => performance.now(), frames = browserFrames }: PlaybackClockOptions) {
    this.selfTimed = selfTimed;
    this.nowMs = nowMs;
    this.frames = frames;
    this.anchorMs = nowMs();
  }

  time(): number {
    return this.clamp(this.isAdvancing() ? this.extrapolate() : this.anchorTime);
  }

  duration(): number {
    return this.durationSeconds;
  }

  isAdvancing(): boolean {
    return this.running && (this.selfTimed || this.live);
  }

  report(time: number): void {
    const wasAdvancing = this.isAdvancing();
    this.anchorTime = time;
    this.anchorMs = this.nowMs();
    this.live = true;
    if (!wasAdvancing) this.changed();
  }

  jump(time: number): void {
    this.anchorTime = time;
    this.anchorMs = this.nowMs();
    this.live = false;
    this.changed();
  }

  setRunning(running: boolean): void {
    if (running === this.running) return;
    this.anchorTime = this.time();
    this.anchorMs = this.nowMs();
    this.running = running;
    this.live = false;
    this.changed();
  }

  setDuration(seconds: number): void {
    if (seconds === this.durationSeconds) return;
    this.durationSeconds = seconds;
    this.changed();
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    this.schedule();
    return () => {
      this.listeners.delete(listener);
      if (this.listeners.size === 0) this.stopFrames();
    };
  };

  private extrapolate(): number {
    const elapsed = (this.nowMs() - this.anchorMs) / MS_PER_SECOND;
    return this.anchorTime + (this.selfTimed ? elapsed : Math.min(elapsed, MAX_EXTRAPOLATION_SECONDS));
  }

  private clamp(time: number): number {
    return Math.min(Math.max(time, 0), this.durationSeconds);
  }

  private changed(): void {
    for (const listener of this.listeners) listener();
    this.schedule();
  }

  private schedule(): void {
    if (this.frameHandle !== null || this.listeners.size === 0 || !this.isAdvancing()) return;
    this.frameHandle = this.frames.request(this.tick);
  }

  private tick = (): void => {
    this.frameHandle = null;
    for (const listener of this.listeners) listener();
    this.schedule();
  };

  private stopFrames(): void {
    if (this.frameHandle === null) return;
    this.frames.cancel(this.frameHandle);
    this.frameHandle = null;
  }
}
