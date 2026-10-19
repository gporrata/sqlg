import { trackWindowResize } from "../lib/windowChrome";
import type { Edge } from "../lib/windowFrame";

const edges: Edge[] = ["n", "e", "s", "w", "ne", "nw", "se", "sw"];

export function WindowEdges() {
  return (
    <>
      {edges.map((edge) => (
        <div
          key={edge}
          className={`resize ${edge}`}
          onPointerDown={(event) => {
            if (event.button !== 0) return;
            event.preventDefault();
            event.stopPropagation();
            trackWindowResize(edge, event.screenX, event.screenY);
          }}
        />
      ))}
    </>
  );
}
