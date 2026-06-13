import { useEffect, useState } from "react";
import { ChevronLeft, Layers, Trash2, Upload, CheckCircle, XCircle, Loader2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Server } from "../types";

interface ModpackDto {
  id: string;
  server_id: string;
  name: string;
  format: string;
  source_path: string;
}

interface InstallSummaryDto {
  modpack: ModpackDto;
  total_files: number;
  downloaded: number;
  skipped: number;
  loader_info: string | null;
}

interface ProgressEvent {
  total: number;
  done: number;
  file: string;
}

interface Props {
  server: Server;
  onBack: () => void;
}

export function Modpacks({ server, onBack }: Props) {
  const [modpacks, setModpacks]     = useState<ModpackDto[]>([]);
  const [error, setError]           = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress]     = useState<ProgressEvent | null>(null);
  const [summary, setSummary]       = useState<InstallSummaryDto | null>(null);

  async function refresh() {
    try {
      const list = await invoke<ModpackDto[]>("list_modpacks", { serverId: server.id });
      setModpacks(list);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => { refresh(); }, [server.id]);

  async function handleInstallClick() {
    setError(null);
    setSummary(null);

    // Native file picker — no prompt(), no manual paths
    const selected = await open({
      title: "Seleccionar modpack",
      filters: [
        { name: "Modpack", extensions: ["mrpack", "zip"] },
      ],
      multiple: false,
      directory: false,
    });

    if (!selected) return; // user cancelled
    const sourcePath = typeof selected === "string" ? selected : selected[0];
    if (!sourcePath) return;

    setInstalling(true);
    setProgress(null);

    const eventName = `modpack-progress:${server.id}`;
    // Subscribe to progress events BEFORE invoking so we don't miss early events
    const unlisten = await listen<ProgressEvent>(eventName, (evt) => {
      setProgress(evt.payload);
    });

    try {
      const installDir = `/tmp/cubed-dev/servers/${server.name}`;
      const result = await invoke<InstallSummaryDto>("install_modpack", {
        serverId: server.id,
        sourcePath,
        installDir,
      });
      setSummary(result);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      unlisten();
      setInstalling(false);
      setProgress(null);
    }
  }

  async function handleDelete(id: string) {
    try {
      await invoke("delete_modpack", { modpackId: id });
      setModpacks((prev) => prev.filter((m) => m.id !== id));
    } catch (e) {
      setError(String(e));
    }
  }

  const progressPct = progress && progress.total > 0
    ? Math.round((progress.done / progress.total) * 100)
    : null;

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
            <Layers className="h-5 w-5 text-primary" />
            Modpacks — <span className="text-primary">{server.name}</span>
          </h1>
        </div>
        <button
          onClick={handleInstallClick}
          disabled={installing}
          className="flex items-center gap-1.5 rounded-md bg-primary text-primary-foreground px-3 py-1.5 text-sm font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors"
        >
          {installing
            ? <Loader2 className="h-3.5 w-3.5 animate-spin" />
            : <Upload className="h-3.5 w-3.5" />}
          {installing ? "Instalando…" : "Instalar modpack"}
        </button>
      </div>

      {/* Progress */}
      {installing && (
        <div className="rounded-lg border bg-card p-4 flex flex-col gap-2">
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground truncate max-w-xs">
              {progress ? `Descargando: ${progress.file}` : "Preparando archivos…"}
            </span>
            {progressPct !== null && (
              <span className="text-xs font-mono tabular-nums">{progressPct}%</span>
            )}
          </div>
          <div className="h-2 rounded-full bg-muted overflow-hidden">
            <div
              className="h-full rounded-full bg-primary transition-all duration-300"
              style={{ width: progressPct !== null ? `${progressPct}%` : "0%" }}
            />
          </div>
          {progress && (
            <p className="text-xs text-muted-foreground">
              {progress.done} / {progress.total} archivos
            </p>
          )}
        </div>
      )}

      {/* Summary */}
      {summary && (
        <div className="rounded-lg border border-green-500/50 bg-green-500/10 p-4 text-sm">
          <div className="flex items-center gap-2 font-medium text-green-700 dark:text-green-400 mb-1">
            <CheckCircle className="h-4 w-4 shrink-0" />
            <span>Modpack instalado: {summary.modpack.name}</span>
          </div>
          <p className="text-muted-foreground ml-6">
            {summary.downloaded}/{summary.total_files} archivos descargados
            {summary.skipped > 0 ? ` · ${summary.skipped} omitidos` : ""}
            {summary.loader_info ? ` · ${summary.loader_info}` : ""}
          </p>
        </div>
      )}

      {error && (
        <div className="flex items-center gap-2 rounded-md border border-destructive/50 bg-destructive/10 px-4 py-2 text-sm text-destructive">
          <XCircle className="h-4 w-4 shrink-0" /> {error}
        </div>
      )}

      {modpacks.length === 0 ? (
        <div className="flex-1 flex items-center justify-center rounded-lg border border-dashed text-center text-muted-foreground p-12">
          <div>
            <p className="text-sm">No hay modpacks instalados.</p>
            <p className="text-xs mt-1">Soporta <strong>.mrpack</strong> (Modrinth) y <strong>.zip</strong> (CurseForge).</p>
          </div>
        </div>
      ) : (
        <ul className="divide-y rounded-lg border bg-card">
          {modpacks.map((m) => (
            <li key={m.id} className="flex items-center justify-between px-4 py-3 gap-4">
              <div className="flex items-center gap-2 min-w-0">
                <Layers className="h-4 w-4 text-primary shrink-0" />
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{m.name}</p>
                  <p className="text-xs text-muted-foreground">{m.format}</p>
                </div>
              </div>
              <button
                onClick={() => handleDelete(m.id)}
                className="flex items-center gap-1 rounded px-2 py-1 text-xs text-destructive hover:bg-destructive/10 transition-colors shrink-0"
              >
                <Trash2 className="h-3.5 w-3.5" /> Eliminar
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
