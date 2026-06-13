import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { cn } from "@/lib/utils";

/**
 * Fase 0 — App vacía que verifica el puente Frontend <-> Backend (Tauri).
 * El dashboard real llega en la Fase 8.
 */
function App() {
  const [status, setStatus] = useState<string>("conectando…");
  const [healthy, setHealthy] = useState<boolean>(false);

  useEffect(() => {
    invoke<string>("health_check")
      .then((msg) => {
        setStatus(msg);
        setHealthy(true);
      })
      .catch(() => setStatus("backend no disponible (¿corriendo fuera de Tauri?)"));
  }, []);

  return (
    <main className="flex min-h-screen flex-col items-center justify-center bg-background text-foreground gap-4">
      <h1 className="text-4xl font-bold tracking-tight">Cubed</h1>
      <p className="text-muted-foreground">
        Administrador local de servidores Minecraft · Fase 0
      </p>
      <div
        className={cn(
          "rounded-md border px-4 py-2 text-sm",
          healthy
            ? "border-primary text-primary"
            : "border-border text-muted-foreground",
        )}
      >
        {status}
      </div>
    </main>
  );
}

export default App;
