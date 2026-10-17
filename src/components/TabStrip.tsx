import { badgeTone } from "../lib/badge";
import { isDirty } from "../lib/dirty";
import { deriveTitle, tabTooltip } from "../lib/titles";
import { useApp } from "../app/store";
import type { Connection, Tab } from "../app/types";

export function TabStrip({ onClose, onNew }: { onClose: (id: string) => void; onNew: () => void }) {
  const tabs = useApp((state) => state.tabs);
  const activeTabId = useApp((state) => state.activeTabId);
  const connections = useApp((state) => state.connections);
  const setActive = useApp((state) => state.setActive);
  return (
    <div className="titlebar">
      <div className="tabs">
        {tabs.map((tab) => (
          <TabChip
            key={tab.id}
            tab={tab}
            connection={connections.find((item) => item.name === tab.connectionName) ?? null}
            active={tab.id === activeTabId}
            onSelect={() => setActive(tab.id)}
            onClose={() => onClose(tab.id)}
          />
        ))}
        <button className="plus" type="button" onClick={onNew} aria-label="New tab">
          +
        </button>
      </div>
    </div>
  );
}

function TabChip({
  tab,
  connection,
  active,
  onSelect,
  onClose,
}: {
  tab: Tab;
  connection: Connection | null;
  active: boolean;
  onSelect: () => void;
  onClose: () => void;
}) {
  const title = deriveTitle(tab.sql);
  const dirty = isDirty(tab);
  const missing = tab.connectionName != null && connection == null;
  return (
    <div className={active ? "tab" : "tab off"} title={tabTooltip(tab.sql, tab.filePath)} onClick={onSelect}>
      {tab.connectionName ? (
        <span className={`pill ${badgeTone(connection?.name ?? tab.connectionName, connection?.color)}`}>
          {missing ? "missing" : tab.connectionName}
        </span>
      ) : null}
      {title.placeholder ? <em className="placeholder">New query</em> : <span className={title.placeholder ? "" : "ttl"}>{title.label}</span>}
      <button
        className={dirty ? "x dirty" : "x"}
        type="button"
        aria-label={dirty ? "Unsaved changes" : "Close tab"}
        onClick={(event) => {
          event.stopPropagation();
          onClose();
        }}
      >
        {dirty ? <i className="udot" /> : "×"}
      </button>
    </div>
  );
}
