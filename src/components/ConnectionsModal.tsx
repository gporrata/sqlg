import { useEffect, useState } from "react";
import type { Connection } from "../app/types";

type Draft = {
  original: string | null;
  name: string;
  host: string;
  port: string;
  user: string;
  password: string;
  schema: string;
  ssl: boolean;
  explainLint: boolean;
  color: string;
};

const COLORS = ["", "green", "red", "blue", "orange", "purple", "teal"];

function draftFrom(connection: Connection | undefined): Draft {
  return {
    original: connection?.name ?? null,
    name: connection?.name ?? "",
    host: connection?.host ?? "127.0.0.1",
    port: String(connection?.port ?? 3306),
    user: connection?.user ?? "",
    password: "",
    schema: connection?.schema ?? "",
    ssl: connection?.ssl ?? false,
    explainLint: connection?.explain_lint ?? false,
    color: connection?.color ?? "",
  };
}

export function ConnectionsModal({
  connections,
  onClose,
  onSave,
  onRemove,
  onTest,
}: {
  connections: Connection[];
  onClose: () => void;
  onSave: (draft: Draft) => Promise<void>;
  onRemove: (name: string) => Promise<void>;
  onTest: (draft: Draft) => Promise<string>;
}) {
  const [selected, setSelected] = useState(connections[0]?.name ?? "");
  const [draft, setDraft] = useState<Draft>(() => draftFrom(connections[0]));
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  useEffect(() => {
    const current = connections.find((connection) => connection.name === selected);
    setDraft(draftFrom(current));
  }, [selected, connections]);

  function add() {
    setSelected("");
    setDraft({
      original: null,
      name: "",
      host: "127.0.0.1",
      port: "3306",
      user: "",
      password: "",
      schema: "",
      ssl: false,
      explainLint: false,
      color: "",
    });
    setMessage("");
    setError("");
  }

  return (
    <div className="mask" role="presentation">
      <div className="modal" role="dialog" aria-labelledby="conn-title">
        <div className="mt" id="conn-title">
          Connections
        </div>
        <div className="mb">
          <div className="ml">
            {connections.map((connection) => (
              <button
                key={connection.name}
                type="button"
                className={connection.name === selected ? "tr sel" : "tr"}
                onClick={() => setSelected(connection.name)}
              >
                {connection.name}
              </button>
            ))}
            <div className="pm">
              <button type="button" onClick={add} aria-label="Add connection">
                +
              </button>
              <button
                type="button"
                aria-label="Remove connection"
                onClick={() => {
                  if (selected) void onRemove(selected).then(() => setSelected(connections.find((item) => item.name !== selected)?.name ?? ""));
                }}
              >
                −
              </button>
            </div>
          </div>
          <form
            className="form"
            onSubmit={(event) => {
              event.preventDefault();
              setError("");
              void onSave(draft).then(
                () => {
                  setSelected(draft.name);
                  setMessage("Saved");
                },
                (err: unknown) => setError(err instanceof Error ? err.message : String(err)),
              );
            }}
          >
            <label htmlFor="c-name">Name</label>
            <input id="c-name" value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} />
            <label htmlFor="c-host">Host</label>
            <input id="c-host" value={draft.host} onChange={(event) => setDraft({ ...draft, host: event.target.value })} />
            <label htmlFor="c-port">Port</label>
            <input id="c-port" value={draft.port} onChange={(event) => setDraft({ ...draft, port: event.target.value })} />
            <label htmlFor="c-user">User</label>
            <input id="c-user" value={draft.user} onChange={(event) => setDraft({ ...draft, user: event.target.value })} />
            <label htmlFor="c-pass">Password</label>
            <input id="c-pass" type="password" value={draft.password} placeholder={draft.original ? "Leave blank to keep" : ""} onChange={(event) => setDraft({ ...draft, password: event.target.value })} />
            <label htmlFor="c-schema">Default schema</label>
            <input id="c-schema" value={draft.schema} onChange={(event) => setDraft({ ...draft, schema: event.target.value })} />
            <label htmlFor="c-color">Badge color</label>
            <select id="c-color" value={draft.color} onChange={(event) => setDraft({ ...draft, color: event.target.value })}>
              {COLORS.map((color) => (
                <option key={color || "auto"} value={color}>
                  {color || "Automatic"}
                </option>
              ))}
            </select>
            <label>SSL</label>
            <button className={draft.ssl ? "tog on" : "tog"} type="button" aria-pressed={draft.ssl} onClick={() => setDraft({ ...draft, ssl: !draft.ssl })} />
            <label htmlFor="c-explain">Explain lint</label>
            <input id="c-explain" type="checkbox" checked={draft.explainLint} onChange={(event) => setDraft({ ...draft, explainLint: event.target.checked })} />
            <div className="form-msg">
              {error ? <span className="err">{error}</span> : null}
              {message ? <span className="ok">{message}</span> : null}
            </div>
            <div className="mf flat">
              <button
                className="btn"
                type="button"
                onClick={() => {
                  setError("");
                  setMessage("");
                  void onTest(draft).then(
                    (version) => setMessage(version),
                    (err: unknown) => setError(err instanceof Error ? err.message : String(err)),
                  );
                }}
              >
                Test Connection
              </button>
              <span className="grow" />
              <button className="btn" type="button" onClick={onClose}>
                Cancel
              </button>
              <button className="btn p" type="submit">
                Save
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}

export type { Draft };
