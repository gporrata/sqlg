import type { Connection, ConnectionInput, ConnectionSchemaMetadata, QueryHistoryEntry, QueryResult, Settings } from "./types";

declare global {
  interface Window {
    sqlg: {
      listConnections(): Promise<Connection[]>;
      addConnection(connection: ConnectionInput): Promise<Connection>;
      deleteConnection(id: string): Promise<boolean>;
      testConnection(id: string): Promise<boolean>;
      loadConnectionSchema(connectionId: string, refresh?: boolean): Promise<ConnectionSchemaMetadata>;
      executeQuery(payload: { connectionId: string; sql: string; queryId?: string }): Promise<QueryResult>;
      cancelQuery(queryId: string): Promise<boolean>;
      listQueryHistory(connectionId: string): Promise<QueryHistoryEntry[]>;
      getSettings(): Promise<Settings>;
      saveSettings(settings: Settings): Promise<Settings>;
    };
  }
}

export {};
