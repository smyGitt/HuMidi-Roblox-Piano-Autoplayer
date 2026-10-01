import { useEffect, useRef, useState } from "react";
import type { PlaybackClock } from "./playbackClock";

export function useClockValue<T>(clock: PlaybackClock, select: (time: number) => T): T {
  const selectRef = useRef(select);
  selectRef.current = select;
  const [value, setValue] = useState(() => select(clock.time()));

  useEffect(() => {
    const update = () => setValue(selectRef.current(clock.time()));
    update();
    return clock.subscribe(update);
  }, [clock]);

  return value;
}
