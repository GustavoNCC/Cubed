import { useCallback, useEffect, useState } from "react";
import {
  ArchiveRestore,
  CheckCircle,
  ChevronLeft,
  Download,
  Plus,
  Trash2,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { Server } from "../types";

interface BackupDto {
  id: string;
  server_id: string;
  path: string;
  size_bytes: number;
  created_at: string;
}

interface Props {
  server: Server;
  onBack: () => void;
}

function fmt_bytes(b: number): string {
  if (b >= 1e9) return `${(b / 1e9).toFixed(2)} GB`;
  if (b >= 1e6) return `${(b / 1e6).toFixed(1)} MB`;
  return `${(b / 1e3).toFixed(0)} KB`;
}

function fmt_date(iso: string): string {
  return new Date(iso).toLocaleString();
}

export function Backups({ server, onBack }: Props) {
  const [backups, setBackups] = useState<BackupDto[]>([]);
  const [loading, setLoading] = useState(false);
  const [working, setWorking] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<BackupDto[]>("list_backups", {
        serverId: server.id,
      });
      setBackups(list);
    } catch (e) {
      setError(String(e));
    }
  }, [server.id]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  async function handleCreate() {
    setLoading(true);
    setError(null);
    setInfo(null);
    try {
      await invoke("create_backup", {
        serverId: server.id,
        serverName: server.name,
        serverDir: `/tmp/cubed-dev/servers/${server.name}`,
      });
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleDelete(id: string) {
    setWorking(id);
    setError(null);
    setInfo(null);
    try {
      await invoke("delete_backup", { backupId: id, deleteFile: true });
      setBackups((prev) => prev.filter((b) => b.id !== id));
    } catch (e) {
      setError(String(e));
    } finally {
      setWorking(null);
    }
  }

  async function handleRestore(id: string) {
    setWorking(id);
    setError(null);
    setInfo(null);
    try {
      const restoreDir = `/tmp/cubed-dev/servers/${server.name}_restored`;
      await invoke("restore_backup", { backupId: id, restoreDir });
      setInfo(`Backup restaurado en: ${restoreDir}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setWorking(null);
    }
  }

  return (
    <div className="flex flex-col gap-4 h-full">
      {/* Header */}
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <button
            onClick={onBack}
            className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors"
          >
            <ChevronLeft className="h-4 w-4" /> Volver
          </button>
          <h1 className="text-xl font-bold flex items-center gap-2">
            <Download className="h-5 w-5 text-primary" />
            Backups — <span className="text-primary">{server.name}</span>
          </h1>
        </div>
        <button
          onClick={handleCreate}
          disabled={loading}
          className="flex items-center gap-1.5 rounded-md bg-primary text-primary-foreground px-3 py-1.5 text-sm font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors"
        >
          <Plus className="h-3.5 w-3.5" />
          {loading ? "Creando…" : "Crear backup"}
        </button>
      </div>

      {info && (
        <div className="flex items-center gap-2 rounded-md border border-primary/40 bg-primary/10 px-4 py-2 text-sm text-primary">
          <CheckCircle className="h-4 w-4 shrink-0" /> {info}
        </div>
      )}
      {error && (
        <div className="rounded-md border border-destructive/50 bg-destructive/10 px-4 py-2 text-sm text-destructive">
          {error}
        </div>
      )}

      {/* List */}
      {backups.length === 0 ? (
        <div className="flex-1 flex items-center justify-center rounded-lg border border-dashed text-center text-muted-foreground p-12">
          <div>
            <p className="text-sm">No hay backups todavía.</p>
            <p className="text-xs mt-1">
              Pulsa <strong>Crear backup</strong> para hacer el primero.
            </p>
          </div>
        </div>
      ) : (
        <ul className="divide-y rounded-lg border bg-card">
          {backups.map((b) => (
            <li
              key={b.id}
              className="flex items-center justify-between px-4 py-3 gap-4"
            >
              <div className="min-w-0">
                <p className="text-sm font-medium truncate">
                  {b.path.split("/").pop()}
                </p>
                <p className="text-xs text-muted-foreground">
                  {fmt_date(b.created_at)} · {fmt_bytes(b.size_bytes)}
                </p>
              </div>
              <div className="flex gap-2 shrink-0">
                <button
                  onClick={() => handleRestore(b.id)}
                  disabled={working === b.id}
                  title="Restaurar"
                  className="flex items-center gap-1 rounded px-2 py-1 text-xs text-muted-foreground hover:text-foreground hover:bg-muted disabled:opacity-50 transition-colors"
                >
                  <ArchiveRestore className="h-3.5 w-3.5" /> Restaurar
                </button>
                <button
                  onClick={() => handleDelete(b.id)}
                  disabled={working === b.id}
                  title="Eliminar"
                  className="flex items-center gap-1 rounded px-2 py-1 text-xs text-destructive hover:bg-destructive/10 disabled:opacity-50 transition-colors"
                >
                  <Trash2 className="h-3.5 w-3.5" /> Eliminar
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
