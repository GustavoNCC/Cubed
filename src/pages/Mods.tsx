import { useEffect, useRef, useState } from "react";
import { ChevronLeft, Package, Trash2, Upload, CheckCircle, XCircle } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { Server } from "../types";

interface ModDto {
  id: string;
  server_id: string;
  file_name: string;
  path: string;
}

interface Props {
  server: Server;
  onBack: () => void;
}

export function Mods({ server, onBack }: Props) {
  const [mods, setMods]       = useState<ModDto[]>([]);
  const [working, setWorking] = useState<string | null>(null);
  const [error, setError]     = useState<string | null>(null);
  const [info, setInfo]       = useState<string | null>(null);
  const fileInputRef          = useRef<HTMLInputElement>(null);

  async function refresh() {
    try {
      const list = await invoke<ModDto[]>("list_mods", { serverId: server.id });
      setMods(list);
    } catch (e) { setError(String(e)); }
  }

  useEffect(() => { refresh(); }, [server.id]);

  function handleInstall() {
    setError(null);
    setInfo(null);
    fileInputRef.current?.click();
  }

  async function handleFileSelected(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    // In Tauri the file object has a real path via the webkitRelativePath or
    // we use a path prompt as fallback — ask the user for the absolute path.
    const sourcePath = (file as unknown as { path?: string }).path ?? prompt(
      `Introduce la ruta absoluta al archivo:\n${file.name}`
    );
    e.target.value = ""; // reset input
    if (!sourcePath) return;

    setWorking("install");
    try {
      const valid = await invoke<boolean>("validate_jar", { path: sourcePath });
      if (!valid) { setError("El archivo no es un JAR válido (cabecera inválida)."); return; }

      const modsDir = `/tmp/cubed-dev/servers/${server.name}/mods`;
      await invoke("install_mod", { serverId: server.id, sourcePath, modsDir });
      await refresh();
      setInfo("Mod instalado correctamente.");
    } catch (err) { setError(String(err)); } finally { setWorking(null); }
  }

  async function handleRemove(id: string) {
    setWorking(id);
    setError(null);
    try {
      await invoke("remove_mod", { modId: id });
      setMods((prev) => prev.filter((m) => m.id !== id));
    } catch (e) { setError(String(e)); } finally { setWorking(null); }
  }

  return (
    <div className="flex flex-col gap-4 h-full">
      <input
        ref={fileInputRef}
        type="file"
        accept=".jar"
        className="hidden"
        onChange={handleFileSelected}
      />
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
            <Package className="h-5 w-5 text-primary" />
            Mods — <span className="text-primary">{server.name}</span>
          </h1>
        </div>
        <button
          onClick={handleInstall}
          disabled={working === "install"}
          className="flex items-center gap-1.5 rounded-md bg-primary text-primary-foreground px-3 py-1.5 text-sm font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors"
        >
          <Upload className="h-3.5 w-3.5" />
          {working === "install" ? "Instalando…" : "Instalar mod"}
        </button>
      </div>

      {error && (
        <div className="flex items-center gap-2 rounded-md border border-destructive/50 bg-destructive/10 px-4 py-2 text-sm text-destructive">
          <XCircle className="h-4 w-4 shrink-0" /> {error}
        </div>
      )}
      {info && (
        <div className="flex items-center gap-2 rounded-md border border-green-500/50 bg-green-500/10 px-4 py-2 text-sm text-green-700 dark:text-green-400">
          <CheckCircle className="h-4 w-4 shrink-0" /> {info}
        </div>
      )}

      {/* List */}
      {mods.length === 0 ? (
        <div className="flex-1 flex items-center justify-center rounded-lg border border-dashed text-center text-muted-foreground p-12">
          <div>
            <p className="text-sm">No hay mods instalados.</p>
            <p className="text-xs mt-1">Pulsa <strong>Instalar mod</strong> para añadir un .jar.</p>
          </div>
        </div>
      ) : (
        <ul className="divide-y rounded-lg border bg-card">
          {mods.map((m) => (
            <li key={m.id} className="flex items-center justify-between px-4 py-3 gap-4">
              <div className="flex items-center gap-2 min-w-0">
                <Package className="h-4 w-4 text-primary shrink-0" />
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{m.file_name}</p>
                  <p className="text-xs text-muted-foreground truncate">{m.path}</p>
                </div>
              </div>
              <button
                onClick={() => handleRemove(m.id)}
                disabled={working === m.id}
                className="flex items-center gap-1 rounded px-2 py-1 text-xs text-destructive hover:bg-destructive/10 disabled:opacity-50 transition-colors shrink-0"
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
