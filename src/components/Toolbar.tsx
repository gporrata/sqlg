import { LAYOUTS } from "../lib/layout";
import { useApp } from "../app/store";
import { LayoutGlyph } from "./Icons";
import type { Tab } from "../app/types";

export function Toolbar({ tab }: { tab: Tab }) {
  const connections = useApp((state) => state.connections);
  const status = useApp((state) => state.status);
  const updateTab = useApp((state) => state.updateTab);
  const setConnectionsOpen = useApp((state) => state.setConnectionsOpen);
  const dot = tab.connectionName ? status[tab.connectionName] ?? "unknown" : "unknown";
  return (
    <div className="toolbar">
      <label className="pop btn">
        <i className={`dot ${dot}`} />
        <select
          aria-label="Connection"
          value={tab.connectionName ?? ""}
          onChange={(event) => updateTab(tab.id, { connectionName: event.target.value || null })}
        >
          <option value="">No connection</option>
          {connections.map((connection) => (
            <option key={connection.name} value={connection.name}>
              {connection.name}
            </option>
          ))}
        </select>
      </label>
      <button className="btn" type="button" onClick={() => setConnectionsOpen(true)} aria-label="Connections">
        Connections…
      </button>
      <div className="seg">
        {LAYOUTS.map((layout) => (
          <button
            key={layout.id}
            type="button"
            className={tab.layout === layout.id ? "on" : ""}
            aria-label={layout.label}
            title={layout.label}
            aria-pressed={tab.layout === layout.id}
            onClick={() => updateTab(tab.id, { layout: layout.id })}
          >
            <LayoutGlyph mode={layout.id} />
          </button>
        ))}
      </div>
    </div>
  );
}
