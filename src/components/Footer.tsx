import { formatElapsed } from "../lib/footer";
import { GearIcon } from "./Icons";
import type { Connection, Tab } from "../app/types";

export function Footer({
  tab,
  connection,
  onSettings,
}: {
  tab: Tab | undefined;
  connection: Connection | undefined;
  onSettings: () => void;
}) {
  const host = connection ? `${connection.name} · ${connection.host}:${connection.port}` : "";
  return (
    <footer className="footer">
      <button
        className="gear"
        type="button"
        aria-label="Settings"
        onPointerDown={(event) => event.stopPropagation()}
        onClick={onSettings}
      >
        <GearIcon />
      </button>
      {tab?.footer.tone === "ok" ? <span className="ok2">✓</span> : null}
      <span className={tab?.footer.tone === "fail" ? "err" : ""}>{tab?.footer.text}</span>
      <span className="r">
        {host ? <span>{host}</span> : null}
        {tab?.footer.elapsedMs != null ? <span>{formatElapsed(tab.footer.elapsedMs)}</span> : null}
      </span>
    </footer>
  );
}
