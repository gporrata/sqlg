import { useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { clampPane, sashSize } from "../lib/layout";

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
  const [drag, setDrag] = useState(false);
  const dragging = useRef(false);
  const down = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    dragging.current = true;
    setDrag(true);
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
      setDrag(false);
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", end);
      target.removeEventListener("pointercancel", end);
      if (!target.matches(":hover")) setHot(false);
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", end);
    target.addEventListener("pointercancel", end);
  };
  const axisClass = axis === "x" ? "split x" : "split y";
  return (
    <div
      className={[axisClass, hot ? "on" : "", drag ? "dragging" : ""].filter(Boolean).join(" ")}
      style={axis === "x" ? { width: sashSize } : { height: sashSize }}
      role="separator"
      aria-orientation={axis === "x" ? "vertical" : "horizontal"}
      aria-label={label}
      onPointerEnter={() => setHot(true)}
      onPointerLeave={() => {
        if (!dragging.current) setHot(false);
      }}
      onPointerDown={down}
    />
  );
}
