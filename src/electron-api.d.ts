import type { Connection, ConnectionInput, QueryHistoryEntry, QueryResult, Settings } from "./types";

declare global {
  interface Window {
    sqlg: {
      listConnections(): Promise<Connection[]>;
      addConnection(connection: ConnectionInput): Promise<Connection>;
      deleteConnection(id: string): Promise<boolean>;
      testConnection(id: string): Promise<boolean>;
      executeQuery(payload: { connectionId: string; sql: string }): Promise<QueryResult>;
      listQueryHistory(connectionId: string): Promise<QueryHistoryEntry[]>;
      getSettings(): Promise<Settings>;
      saveSettings(settings: Settings): Promise<Settings>;
    };
  }
}

export {};
