import type { Connection, ConnectionInput, QueryResult, Settings } from "./types";

declare global {
  interface Window {
    sqlg: {
      listConnections(): Promise<Connection[]>;
      addConnection(connection: ConnectionInput): Promise<Connection>;
      deleteConnection(id: string): Promise<boolean>;
      testConnection(id: string): Promise<boolean>;
      executeQuery(payload: { connectionId: string; sql: string }): Promise<QueryResult>;
      getSettings(): Promise<Settings>;
      saveSettings(settings: Settings): Promise<Settings>;
    };
  }
}

export {};
