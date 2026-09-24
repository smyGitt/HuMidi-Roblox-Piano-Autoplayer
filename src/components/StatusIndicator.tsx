import { useEffect, useRef } from "react";
import { CheckCircleIcon, CircleDashedIcon, CircleIcon } from "@phosphor-icons/react";

export type PlaybackStatus = "unloaded" | "loading" | "loaded" | "ready";

type Phase = "rest" | "cruise" | "burst" | "landing" | "bobble";

interface Rotor {
  angle: number;
  velocity: number;
  phase: Phase;
  target: number;
}

const CRUISE_SPEED = 420;
const SPIN_UP_SECONDS = 0.25;
const BURST_SPEED = 1080;
const BURST_STIFFNESS = 9;
const LANDING_SPEED = 240;
const LANDING_DECAY_SECONDS = 0.3;
const MIN_LANDING_TRAVEL = 90;
const BOBBLE_STIFFNESS = 13;
const BOBBLE_DAMPING_RATIO = 0.28;
const MAX_FRAME_SECONDS = 0.05;
const SUBSTEP_SECONDS = 1 / 240;

function nextUpright(angle: number, minTravel: number) {
  return Math.ceil((angle + minTravel) / 360) * 360;
}

function springAcceleration(offset: number, velocity: number, stiffness: number, dampingRatio: number) {
  return -stiffness * stiffness * offset - 2 * dampingRatio * stiffness * velocity;
}

function settle(rotor: Rotor) {
  rotor.angle = 0;
  rotor.velocity = 0;
  rotor.target = 0;
  rotor.phase = "rest";
}

function retarget(rotor: Rotor, status: PlaybackStatus) {
  if (status === "loading") {
    rotor.phase = "cruise";
    return;
  }
  if (status === "ready") {
    if (rotor.velocity < CRUISE_SPEED / 2) rotor.velocity = CRUISE_SPEED;
    rotor.target = nextUpright(rotor.angle, MIN_LANDING_TRAVEL);
    rotor.phase = "landing";
    return;
  }
  if (status === "loaded") {
    if (rotor.velocity < BURST_SPEED / 2) rotor.velocity = BURST_SPEED;
    rotor.target = nextUpright(rotor.angle, 360);
    rotor.phase = "burst";
    return;
  }
  if (rotor.phase !== "rest") {
    rotor.target = nextUpright(rotor.angle, 0);
    rotor.phase = "burst";
  }
}

function advance(rotor: Rotor, h: number) {
  if (rotor.phase === "cruise") {
    rotor.velocity += (CRUISE_SPEED - rotor.velocity) * (1 - Math.exp(-h / SPIN_UP_SECONDS));
    rotor.angle = (rotor.angle + rotor.velocity * h) % 360;
    return;
  }
  if (rotor.phase === "landing") {
    rotor.velocity = LANDING_SPEED + (rotor.velocity - LANDING_SPEED) * Math.exp(-h / LANDING_DECAY_SECONDS);
    rotor.angle += rotor.velocity * h;
    if (rotor.angle >= rotor.target) rotor.phase = "bobble";
    return;
  }
  const stiffness = rotor.phase === "burst" ? BURST_STIFFNESS : BOBBLE_STIFFNESS;
  const dampingRatio = rotor.phase === "burst" ? 1 : BOBBLE_DAMPING_RATIO;
  rotor.velocity += springAcceleration(rotor.angle - rotor.target, rotor.velocity, stiffness, dampingRatio) * h;
  rotor.angle += rotor.velocity * h;
  if (Math.abs(rotor.angle - rotor.target) < 0.15 && Math.abs(rotor.velocity) < 3) settle(rotor);
}

function step(rotor: Rotor, seconds: number) {
  let remaining = Math.min(seconds, MAX_FRAME_SECONDS);
  while (remaining > 0 && rotor.phase !== "rest") {
    const h = Math.min(remaining, SUBSTEP_SECONDS);
    remaining -= h;
    advance(rotor, h);
  }
}

interface StatusIndicatorProps {
  status: PlaybackStatus;
  label?: string;
}

export function StatusIndicator({ status, label }: StatusIndicatorProps) {
  const spinnerRef = useRef<HTMLSpanElement>(null);
  const rotorRef = useRef<Rotor>({
    angle: 0,
    velocity: status === "loading" ? CRUISE_SPEED : 0,
    phase: status === "loading" ? "cruise" : "rest",
    target: 0,
  });
  const previousStatusRef = useRef(status);
  const frameRef = useRef(0);

  useEffect(() => {
    const rotor = rotorRef.current;
    if (previousStatusRef.current !== status) {
      retarget(rotor, status);
      previousStatusRef.current = status;
    }
    if (rotor.phase === "rest" || frameRef.current !== 0) return;
    let last = 0;
    function tick(now: number) {
      const seconds = last === 0 ? 0 : (now - last) / 1000;
      last = now;
      step(rotor, seconds);
      if (spinnerRef.current) spinnerRef.current.style.transform = `rotate(${rotor.angle}deg)`;
      frameRef.current = rotor.phase === "rest" ? 0 : requestAnimationFrame(tick);
    }
    frameRef.current = requestAnimationFrame(tick);
  }, [status]);

  useEffect(
    () => () => {
      cancelAnimationFrame(frameRef.current);
      frameRef.current = 0;
    },
    [],
  );

  const glyph = status === "loaded" || status === "unloaded" ? "circle" : status === "ready" ? "check" : "dashed";

  return (
    <div className="status-indicator" data-status={status} title={label}>
      <span className="status-indicator__spinner" ref={spinnerRef}>
        <CircleDashedIcon className="status-indicator__glyph" data-active={glyph === "dashed"} size={40} weight="duotone" />
        <CircleIcon className="status-indicator__glyph" data-active={glyph === "circle"} size={40} weight="duotone" />
        <CheckCircleIcon className="status-indicator__glyph" data-active={glyph === "check"} size={40} weight="duotone" />
      </span>
      {label && <span className="status-indicator__label">{label}</span>}
    </div>
  );
}
