import { useEffect, useRef } from "react";
import { autocompletion } from "@codemirror/autocomplete";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { MySQL, sql } from "@codemirror/lang-sql";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { linter, lintGutter, type Diagnostic as CmDiagnostic } from "@codemirror/lint";
import { Compartment, EditorState, StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, drawSelection, highlightActiveLine, keymap, lineNumbers } from "@codemirror/view";
import { tags } from "@lezer/highlight";
import { api, type Diagnostic } from "../app/api";
import { useApp } from "../app/store";
import { completionSchema, lintCatalog } from "../lib/schema";
import { selectedText } from "../lib/dirty";
import type { Tab } from "../app/types";

const setFlash = StateEffect.define<{ from: number; to: number } | null>();
const flashField = StateField.define({
  create() {
    return Decoration.none;
  },
  update(value, transaction) {
    for (const effect of transaction.effects) {
      if (effect.is(setFlash)) {
        if (!effect.value || effect.value.from === effect.value.to) {
          return Decoration.none;
        }
        const from = Math.max(0, Math.min(effect.value.from, transaction.state.doc.length));
        const to = Math.max(from, Math.min(effect.value.to, transaction.state.doc.length));
        return Decoration.set([Decoration.mark({ class: "cm-ran" }).range(from, to)]);
      }
    }
    return value.map(transaction.changes);
  },
  provide: (field) => EditorView.decorations.from(field),
});

const lightHl = HighlightStyle.define([
  { tag: tags.keyword, color: "#ad3da4", fontWeight: "600" },
  { tag: tags.string, color: "#c41a16" },
  { tag: tags.number, color: "#1c00cf" },
  { tag: tags.comment, color: "#8a8a8e", fontStyle: "italic" },
]);

const darkHl = HighlightStyle.define([
  { tag: tags.keyword, color: "#ff7ab2", fontWeight: "600" },
  { tag: tags.string, color: "#ff8170" },
  { tag: tags.number, color: "#d9c97c" },
  { tag: tags.comment, color: "#7f8c98", fontStyle: "italic" },
]);

const schemaCompartment = new Compartment();
const themeCompartment = new Compartment();
let dispatchRun: (sql: string, from: number, to: number) => void = () => {};

export function QueryEditor({
  tab,
  onRun,
  onRunStatement,
  onCancel,
  onNewTab,
  onNewWindow,
}: {
  tab: Tab;
  onRun: (sql: string, from: number, to: number) => void;
  onRunStatement: (cursor: number) => void;
  onCancel: () => void;
  onNewTab: () => void;
  onNewWindow: () => void;
}) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const handlers = useRef({ onRun, onRunStatement, onCancel });
  handlers.current = { onRun, onRunStatement, onCancel };
  dispatchRun = (sql, from, to) => handlers.current.onRun(sql, from, to);
  const selection = selectedText(tab.sql, tab.selectionAnchor, tab.selectionHead);
  const snapshot = useApp((state) => (tab.connectionName ? state.schemas[tab.connectionName] : undefined));
  const theme = useApp((state) => state.theme);
  const defaultSchema = useApp((state) => state.connections.find((item) => item.name === tab.connectionName)?.schema ?? null);

  useEffect(() => {
    const parent = hostRef.current;
    if (!parent) return;
    const current = useApp.getState().tabs.find((item) => item.id === tab.id) ?? tab;
    const view = new EditorView({
      parent,
      state: EditorState.create({
        doc: current.sql,
        selection: { anchor: current.selectionAnchor ?? current.cursor, head: current.selectionHead ?? current.cursor },
        extensions: [
          lineNumbers(),
          highlightActiveLine(),
          drawSelection(),
          history(),
          lintGutter(),
          flashField,
          schemaCompartment.of(sqlExtension()),
          themeCompartment.of(syntaxHighlighting(document.documentElement.dataset.theme === "dark" ? darkHl : lightHl)),
          sqlLinter(),
          explainLinter(),
          autocompletion(),
          keymap.of([
            { key: "Mod-Enter", run: () => runSelection(viewRef.current) },
            { key: "Shift-Mod-Enter", run: () => { const head = viewRef.current?.state.selection.main.head ?? 0; handlers.current.onRunStatement(head); return true; } },
            { key: "Mod-.", run: () => { handlers.current.onCancel(); return true; } },
            ...defaultKeymap,
            ...historyKeymap,
          ]),
          EditorView.updateListener.of((update) => {
            if (!update.docChanged && !update.selectionSet) return;
            const head = update.state.selection.main;
            useApp.getState().updateTab(tab.id, {
              sql: update.state.doc.toString(),
              cursor: head.head,
              selectionAnchor: head.anchor,
              selectionHead: head.head,
            });
          }),
          EditorView.theme({
            "&": { height: "100%", fontSize: "12.5px" },
            ".cm-scroller": { fontFamily: "Menlo, SF Mono, monospace", lineHeight: "20px" },
            ".cm-gutters": { background: "transparent", borderRight: "1px solid var(--bd)" },
            ".cm-ran": { background: "rgba(10, 100, 216, 0.18)" },
          }),
        ],
      }),
    });
    viewRef.current = view;
    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // Mount once per tab. External reloads go through externalRev.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tab.id]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    if (view.state.doc.toString() !== tab.sql) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: tab.sql } });
    }
  }, [tab.externalRev, tab.sql, tab.id]);

  useEffect(() => {
    viewRef.current?.dispatch({
      effects: schemaCompartment.reconfigure(
        sql({
          dialect: MySQL,
          upperCaseKeywords: true,
          schema: completionSchema(snapshot, defaultSchema) as Record<string, string[]>,
          defaultSchema: defaultSchema ?? undefined,
        }),
      ),
    });
  }, [snapshot, defaultSchema, tab.id]);

  useEffect(() => {
    viewRef.current?.dispatch({
      effects: themeCompartment.reconfigure(syntaxHighlighting(theme === "dark" ? darkHl : lightHl)),
    });
  }, [theme, tab.id]);

  useEffect(() => {
    const view = viewRef.current;
    const flash = tab.flash;
    if (!view || !flash) return;
    view.dispatch({ effects: setFlash.of({ from: flash.from, to: flash.to }) });
    const timer = window.setTimeout(() => view.dispatch({ effects: setFlash.of(null) }), 600);
    return () => window.clearTimeout(timer);
  }, [tab.flash?.token, tab.id]);

  return (
    <section className="mid">
      <div className="edhead">
        <span>SQL</span>
        {tab.diskNotice ? <span className="err">{tab.diskNotice}</span> : null}
        <button className="sb" type="button" onClick={onNewTab}>
          New Tab <kbd>⌘T</kbd>
        </button>
        <button className="sb" type="button" onClick={onNewWindow}>
          New Window <kbd>⌘N</kbd>
        </button>
        <button className="run" type="button" onClick={() => runSelection(viewRef.current)} disabled={tab.running}>
          {selection ? "Run Selection ⌘↵" : "Run ⌘↵"}
        </button>
      </div>
      <div className="editor" ref={hostRef} />
    </section>
  );

  function sqlExtension() {
    const state = useApp.getState();
    const connection = state.connections.find((item) => item.name === tab.connectionName);
    const snapshot = tab.connectionName ? state.schemas[tab.connectionName] : undefined;
    return sql({
      dialect: MySQL,
      upperCaseKeywords: true,
      schema: completionSchema(snapshot, connection?.schema ?? null) as Record<string, string[]>,
      defaultSchema: connection?.schema ?? undefined,
    });
  }
}

function runSelection(view: EditorView | null): boolean {
  if (!view) return true;
  const main = view.state.selection.main;
  const sqlText = main.empty ? view.state.doc.toString() : view.state.sliceDoc(main.from, main.to);
  const from = main.empty ? 0 : main.from;
  const to = main.empty ? view.state.doc.length : main.to;
  dispatchRun(sqlText, from, to);
  return true;
}

function currentTab(): Tab | undefined {
  const state = useApp.getState();
  return state.tabs.find((tab) => tab.id === state.activeTabId);
}

function sqlLinter() {
  return linter(async (view) => {
    const tab = currentTab();
    const snapshot = tab?.connectionName ? useApp.getState().schemas[tab.connectionName] : undefined;
    const connection = useApp.getState().connections.find((item) => item.name === tab?.connectionName);
    try {
      const found = await api.lint(view.state.doc.toString(), lintCatalog(snapshot, connection?.schema ?? null));
      return found.map(toCm);
    } catch {
      return [];
    }
  }, { delay: 250 });
}

function explainLinter() {
  return linter(async (view) => {
    const tab = currentTab();
    const connection = useApp.getState().connections.find((item) => item.name === tab?.connectionName);
    if (!connection?.explain_lint || !tab?.connectionName) return [];
    const text = view.state.doc.toString().trim();
    if (!/^select\b/i.test(text) || text.includes(";")) return [];
    try {
      const message = await api.explain(tab.connectionName, text);
      if (!message) return [];
      return [{ from: 0, to: Math.min(6, text.length), severity: "info" as const, message }];
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      if (message.includes("select-only") || message.includes("single select")) return [];
      return [{ from: 0, to: Math.min(6, view.state.doc.length), severity: "warning" as const, message }];
    }
  }, { delay: 1000 });
}

function toCm(diag: Diagnostic): CmDiagnostic {
  return {
    from: diag.from,
    to: Math.max(diag.to, diag.from),
    severity: diag.severity === "warning" ? "warning" : "error",
    message: diag.message,
    actions: diag.actions.map((action) => ({
      name: action.name,
      apply(view, from, to) {
        view.dispatch({ changes: { from, to, insert: action.text } });
      },
    })),
  };
}

