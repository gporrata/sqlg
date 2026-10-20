import { useEffect } from "react";
import { THEMES } from "../app/themes";

export function SettingsModal({
  theme,
  onTheme,
  onClose,
}: {
  theme: string;
  onTheme: (theme: string) => void;
  onClose: () => void;
}) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="mask" role="presentation" onMouseDown={onClose}>
      <div
        className="modal themes"
        role="dialog"
        aria-labelledby="settings-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="mt settings-title">
          <span id="settings-title">Settings</span>
          <button className="x" type="button" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </div>
        <p className="settings-label">Theme</p>
        <div className="theme-list" role="listbox" aria-label="Theme">
          {THEMES.map((item) => (
            <button
              key={item.id}
              type="button"
              role="option"
              aria-selected={item.id === theme}
              className={item.id === theme ? "theme-row on" : "theme-row"}
              onClick={() => onTheme(item.id)}
            >
              <span className="swatch" aria-hidden="true">
                <i style={{ background: item.vars["--ed-bg"] }} />
                <i style={{ background: item.vars["--syn-keyword"] }} />
                <i style={{ background: item.vars["--syn-string"] }} />
                <i style={{ background: item.vars["--syn-number"] }} />
              </span>
              <span className="theme-name">{item.name}</span>
              {item.id === theme ? <span className="theme-check">✓</span> : null}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
