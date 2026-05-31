const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("sqlg", {
  listConnections: () => ipcRenderer.invoke("connections:list"),
  addConnection: (connection) => ipcRenderer.invoke("connections:add", connection),
  deleteConnection: (id) => ipcRenderer.invoke("connections:delete", id),
  testConnection: (id) => ipcRenderer.invoke("connections:test", id),
  executeQuery: (payload) => ipcRenderer.invoke("query:execute", payload),
  cancelQuery: (queryId) => ipcRenderer.invoke("query:cancel", queryId),
  listQueryHistory: (connectionId) => ipcRenderer.invoke("history:list", connectionId),
  getSettings: () => ipcRenderer.invoke("settings:get"),
  saveSettings: (settings) => ipcRenderer.invoke("settings:save", settings),
});
