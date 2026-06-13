import { Play, Square, Trash2, Terminal } from "lucide-react";
import { cn } from "@/lib/utils";
import { StatusBadge } from "./StatusBadge";
import type { Server } from "../types";

interface Props {
  server: Server;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onDelete: (id: string) => void;
  onConsole: () => void;
  loading: boolean;
}

export function ServerCard({ server, onStart, onStop, onDelete, onConsole, loading }: Props) {
  const canStart  = server.status === "offline" || server.status === "crashed";
  const canStop   = server.status === "running";
  const canDelete = server.status === "offline" || server.status === "crashed";

  return (
    <div className="rounded-lg border bg-card p-4 shadow-sm flex flex-col gap-3">
      <div className="flex items-start justify-between gap-2">
        <div>
          <p className="font-semibold text-card-foreground">{server.name}</p>
          <p className="text-sm text-muted-foreground">
            {server.software} {server.version} · :{server.port}
          </p>
        </div>
        <StatusBadge status={server.status} />
      </div>

      <div className="flex gap-2">
        <button
          onClick={() => onStart(server.id)}
          disabled={!canStart || loading}
          className={cn(
            "flex items-center gap-1.5 rounded px-3 py-1.5 text-xs font-medium transition-colors",
            canStart && !loading
              ? "bg-primary text-primary-foreground hover:bg-primary/90"
              : "bg-muted text-muted-foreground cursor-not-allowed"
          )}
        >
          <Play className="h-3 w-3" /> Iniciar
        </button>

        <button
          onClick={() => onStop(server.id)}
          disabled={!canStop || loading}
          className={cn(
            "flex items-center gap-1.5 rounded px-3 py-1.5 text-xs font-medium transition-colors",
            canStop && !loading
              ? "bg-secondary text-secondary-foreground hover:bg-secondary/80"
              : "bg-muted text-muted-foreground cursor-not-allowed"
          )}
        >
          <Square className="h-3 w-3" /> Detener
        </button>

        <button
          onClick={onConsole}
          className="flex items-center gap-1.5 rounded px-3 py-1.5 text-xs font-medium transition-colors text-muted-foreground hover:text-foreground hover:bg-muted"
        >
          <Terminal className="h-3 w-3" /> Consola
        </button>

        <button
          onClick={() => onDelete(server.id)}
          disabled={!canDelete || loading}
          className={cn(
            "ml-auto flex items-center gap-1.5 rounded px-3 py-1.5 text-xs font-medium transition-colors",
            canDelete && !loading
              ? "text-destructive hover:bg-destructive/10"
              : "text-muted-foreground cursor-not-allowed"
          )}
        >
          <Trash2 className="h-3 w-3" /> Eliminar
        </button>
      </div>
    </div>
  );
}
