import { useEffect, useState, useCallback } from "react";
import { Sidebar, type Page } from "./components/Sidebar";
import { Dashboard } from "./pages/Dashboard";
import { Servers } from "./pages/Servers";
import { Settings } from "./pages/Settings";
import { Console } from "./pages/Console";
import { api } from "./api";
import type { Server, CreateServerForm } from "./types";

function App() {
  const [page, setPage]             = useState<Page>("dashboard");
  const [servers, setServers]       = useState<Server[]>([]);
  const [error, setError]           = useState<string | null>(null);
  const [consoleServer, setConsole] = useState<Server | null>(null);

  const refresh = useCallback(async () => {
    try {
      setServers(await api.listServers());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  async function handleCreate(form: CreateServerForm) {
    const server = await api.createServer(form);
    setServers((prev) => [...prev, server]);
  }

  async function handleStart(id: string) {
    const updated = await api.startServer(id);
    setServers((prev) => prev.map((s) => s.id === id ? updated : s));
  }

  async function handleStop(id: string) {
    const updated = await api.stopServer(id);
    setServers((prev) => prev.map((s) => s.id === id ? updated : s));
  }

  async function handleDelete(id: string) {
    await api.deleteServer(id);
    setServers((prev) => prev.filter((s) => s.id !== id));
  }

  return (
    <div className="flex h-screen bg-background text-foreground overflow-hidden">
      <Sidebar current={page} onChange={setPage} />

      <main className="flex-1 overflow-y-auto p-6">
        {error && (
          <div className="mb-4 rounded-md border border-destructive/50 bg-destructive/10 px-4 py-2 text-sm text-destructive">
            {error}
          </div>
        )}

        {page === "dashboard" && <Dashboard servers={servers} />}
        {page === "servers" && !consoleServer && (
          <Servers
            servers={servers}
            onRefresh={refresh}
            onStart={handleStart}
            onStop={handleStop}
            onDelete={handleDelete}
            onCreate={handleCreate}
            onConsole={setConsole}
          />
        )}
        {page === "servers" && consoleServer && (
          <Console server={consoleServer} onBack={() => setConsole(null)} />
        )}
        {page === "settings" && <Settings />}
      </main>
    </div>
  );
}

export default App;
