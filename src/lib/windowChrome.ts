import { getCurrentWindow, LogicalPosition, LogicalSize } from "@tauri-apps/api/window";
import { moveRect, resizeRect, type Edge, type Rect } from "./windowFrame";

export function inShell(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function windowRect(): Promise<Rect | null> {
  if (!inShell()) return null;
  const win = getCurrentWindow();
  const factor = await win.scaleFactor();
  const pos = (await win.outerPosition()).toLogical(factor);
  const size = (await win.outerSize()).toLogical(factor);
  return { x: pos.x, y: pos.y, width: size.width, height: size.height };
}

let suppressTitleClick = false;

export function consumeTitleClick(): boolean {
  const suppress = suppressTitleClick;
  suppressTitleClick = false;
  return suppress;
}

async function applyRect(origin: Rect, next: Rect): Promise<void> {
  const win = getCurrentWindow();
  const ops: Promise<void>[] = [];
  if (next.width !== origin.width || next.height !== origin.height) {
    ops.push(win.setSize(new LogicalSize(next.width, next.height)));
  }
  if (next.x !== origin.x || next.y !== origin.y) {
    ops.push(win.setPosition(new LogicalPosition(next.x, next.y)));
  }
  await Promise.all(ops);
}

function enqueue(queue: Promise<void>, id: number, ticket: () => number, run: () => Promise<void>): Promise<void> {
  return queue
    .then(() => {
      if (id !== ticket()) return;
      return run();
    })
    .then(
      () => undefined,
      () => undefined,
    );
}

export function trackWindowDrag(startX: number, startY: number): void {
  const origin = windowRect();
  let ticket = 0;
  let queue = Promise.resolve();
  const onMove = (event: PointerEvent) => {
    const dx = event.screenX - startX;
    const dy = event.screenY - startY;
    if (Math.hypot(dx, dy) < 4) return;
    const id = ++ticket;
    queue = enqueue(queue, id, () => ticket, () =>
      origin.then((rect) => {
        if (!rect || id !== ticket) return;
        const next = moveRect(rect, dx, dy);
        return applyRect(rect, { ...rect, x: next.x, y: next.y });
      }),
    );
  };
  const onUp = () => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("pointercancel", onUp);
    if (ticket > 0) {
      suppressTitleClick = true;
      window.setTimeout(() => {
        suppressTitleClick = false;
      }, 0);
    }
  };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("pointercancel", onUp);
}

export function trackWindowResize(edge: Edge, startX: number, startY: number): void {
  const origin = windowRect();
  let ticket = 0;
  let queue = Promise.resolve();
  const onMove = (event: PointerEvent) => {
    const dx = event.screenX - startX;
    const dy = event.screenY - startY;
    const id = ++ticket;
    queue = enqueue(queue, id, () => ticket, () =>
      origin.then((rect) => {
        if (!rect || id !== ticket) return;
        return applyRect(rect, resizeRect(rect, edge, dx, dy));
      }),
    );
  };
  const onUp = () => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("pointercancel", onUp);
  };
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("pointercancel", onUp);
}
