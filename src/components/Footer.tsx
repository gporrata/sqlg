import { formatElapsed } from "../lib/footer";
import { MoonIcon, SunIcon } from "./Icons";
import type { Connection, Tab } from "../app/types";

export function Footer({
  tab,
  connection,
  theme,
  onTheme,
}: {
  tab: Tab | undefined;
  connection: Connection | undefined;
  theme: "light" | "dark";
  onTheme: (theme: "light" | "dark") => void;
}) {
  const host = connection ? `${connection.name} · ${connection.host}:${connection.port}` : "";
  return (
    <footer className="footer">
      <div className="theme">
        <button
          type="button"
          className={theme === "light" ? "on" : ""}
          aria-label="Light"
          onPointerDown={(event) => event.stopPropagation()}
          onClick={() => onTheme("light")}
        >
          <SunIcon />
        </button>
        <button
          type="button"
          className={theme === "dark" ? "on" : ""}
          aria-label="Dark"
          onPointerDown={(event) => event.stopPropagation()}
          onClick={() => onTheme("dark")}
        >
          <MoonIcon />
        </button>
      </div>
      {tab?.footer.tone === "ok" ? <span className="ok2">✓</span> : null}
      <span className={tab?.footer.tone === "fail" ? "err" : ""}>{tab?.footer.text}</span>
      <span className="r">
        {host ? <span>{host}</span> : null}
        {tab?.footer.elapsedMs != null ? <span>{formatElapsed(tab.footer.elapsedMs)}</span> : null}
      </span>
    </footer>
  );
}
