import type { PointerEvent as ReactPointerEvent } from "react";
import { clampPane } from "../lib/layout";

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
  const down = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    const origin = size();
    const start = axis === "x" ? event.clientX : event.clientY;
    const target = event.currentTarget;
    target.setPointerCapture(event.pointerId);
    const move = (pointer: PointerEvent) => {
      const point = axis === "x" ? pointer.clientX : pointer.clientY;
      onSize(clampPane(origin + point - start, min, max()));
    };
    const end = () => {
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", end);
      target.removeEventListener("pointercancel", end);
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", end);
    target.addEventListener("pointercancel", end);
  };
  return (
    <div
      className={axis === "x" ? "split x" : "split y"}
      role="separator"
      aria-orientation={axis === "x" ? "vertical" : "horizontal"}
      aria-label={label}
      onPointerDown={down}
    />
  );
}
