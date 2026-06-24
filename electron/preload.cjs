const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("sqlg", {
  listConnections: () => ipcRenderer.invoke("connections:list"),
  addConnection: (connection) => ipcRenderer.invoke("connections:add", connection),
  deleteConnection: (id) => ipcRenderer.invoke("connections:delete", id),
  setDefaultSchema: (connectionId, schema) => ipcRenderer.invoke("connections:set-default-schema", { connectionId, schema }),
  testConnection: (id) => ipcRenderer.invoke("connections:test", id),
  loadConnectionSchema: (connectionId, refresh) => ipcRenderer.invoke("schema:load", { connectionId, refresh }),
  executeQuery: (payload) => ipcRenderer.invoke("query:execute", payload),
  cancelQuery: (queryId) => ipcRenderer.invoke("query:cancel", queryId),
  listQueryHistory: (connectionId) => ipcRenderer.invoke("history:list", connectionId),
  getSettings: () => ipcRenderer.invoke("settings:get"),
  saveSettings: (settings) => ipcRenderer.invoke("settings:save", settings),
});
