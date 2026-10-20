import { useEffect, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { clampPane } from "../lib/layout";

const hoverDelayMs = 300;

export function PaneSplit({
  axis,
  label,
  size,
  onSize,
  min,
  max,
}: {
  axis: "x" | "y";
  label: string;
  size: () => number;
  onSize: (next: number) => void;
  min: number;
  max: () => number;
}) {
  const [hot, setHot] = useState(false);
  const hoverTimer = useRef<number | null>(null);
  const dragging = useRef(false);
  const clearHoverTimer = () => {
    if (hoverTimer.current != null) {
      window.clearTimeout(hoverTimer.current);
      hoverTimer.current = null;
    }
  };
  useEffect(() => clearHoverTimer, []);
  const down = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    clearHoverTimer();
    dragging.current = true;
    setHot(true);
    const origin = size();
    const start = axis === "x" ? event.clientX : event.clientY;
    const target = event.currentTarget;
    target.setPointerCapture(event.pointerId);
    const move = (pointer: PointerEvent) => {
      const point = axis === "x" ? pointer.clientX : pointer.clientY;
      onSize(clampPane(origin + point - start, min, max()));
    };
    const end = () => {
      dragging.current = false;
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", end);
      target.removeEventListener("pointercancel", end);
      if (!target.matches(":hover")) setHot(false);
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", end);
    target.addEventListener("pointercancel", end);
  };
  return (
    <div
      className={hot ? (axis === "x" ? "split x on" : "split y on") : axis === "x" ? "split x" : "split y"}
      role="separator"
      aria-orientation={axis === "x" ? "vertical" : "horizontal"}
      aria-label={label}
      onPointerEnter={() => {
        clearHoverTimer();
        hoverTimer.current = window.setTimeout(() => setHot(true), hoverDelayMs);
      }}
      onPointerLeave={() => {
        clearHoverTimer();
        if (!dragging.current) setHot(false);
      }}
      onPointerDown={down}
    />
  );
}
