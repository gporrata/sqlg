export const MIN_WINDOW_WIDTH = 960;
export const MIN_WINDOW_HEIGHT = 640;

export type Rect = { x: number; y: number; width: number; height: number };
export type Edge = "n" | "e" | "s" | "w" | "ne" | "nw" | "se" | "sw";

export function moveRect(origin: Pick<Rect, "x" | "y">, dx: number, dy: number): Pick<Rect, "x" | "y"> {
  return { x: origin.x + dx, y: origin.y + dy };
}

export function resizeRect(origin: Rect, edge: Edge, dx: number, dy: number): Rect {
  let { x, y, width, height } = origin;
  if (edge.includes("e")) width = Math.max(MIN_WINDOW_WIDTH, origin.width + dx);
  if (edge.includes("w")) {
    width = Math.max(MIN_WINDOW_WIDTH, origin.width - dx);
    x = origin.x + origin.width - width;
  }
  if (edge.includes("s")) height = Math.max(MIN_WINDOW_HEIGHT, origin.height + dy);
  if (edge.includes("n")) {
    height = Math.max(MIN_WINDOW_HEIGHT, origin.height - dy);
    y = origin.y + origin.height - height;
  }
  return { x, y, width, height };
}
