export interface LiveKeysSnapshot {
  pitches: ReadonlySet<number>;
  pedal: boolean;
}

const EMPTY_SNAPSHOT: LiveKeysSnapshot = { pitches: new Set(), pedal: false };

function sameMembers(a: ReadonlySet<number>, b: ReadonlySet<number>): boolean {
  if (a.size !== b.size) return false;
  for (const value of a) if (!b.has(value)) return false;
  return true;
}

export class LiveKeys {
  private current: LiveKeysSnapshot = EMPTY_SNAPSHOT;
  private readonly listeners = new Set<() => void>();

  snapshot = (): LiveKeysSnapshot => this.current;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };

  setPitches(pitches: Iterable<number>): void {
    const next = new Set(pitches);
    if (sameMembers(next, this.current.pitches)) return;
    this.update({ pitches: next, pedal: this.current.pedal });
  }

  setPedal(pedal: boolean): void {
    if (pedal === this.current.pedal) return;
    this.update({ pitches: this.current.pitches, pedal });
  }

  clear(): void {
    if (this.current.pitches.size === 0 && !this.current.pedal) return;
    this.update(EMPTY_SNAPSHOT);
  }

  private update(next: LiveKeysSnapshot): void {
    this.current = next;
    for (const listener of this.listeners) listener();
  }
}
