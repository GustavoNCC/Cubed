import { useState } from "react";
import { Plus } from "lucide-react";
import { ServerCard } from "../components/ServerCard";
import { CreateServerModal } from "../components/CreateServerModal";
import type { Server, CreateServerForm } from "../types";

interface Props {
  servers: Server[];
  onRefresh: () => void;
  onStart: (id: string) => Promise<void>;
  onStop: (id: string) => Promise<void>;
  onDelete: (id: string) => Promise<void>;
  onCreate: (form: CreateServerForm) => Promise<void>;
  onConsole: (server: Server) => void;
  onBackups: (server: Server) => void;
  onMods: (server: Server) => void;
  onModpacks: (server: Server) => void;
}

export function Servers({
  servers,
  onRefresh: _onRefresh,
  onStart,
  onStop,
  onDelete,
  onCreate,
  onConsole,
  onBackups,
  onMods,
  onModpacks,
}: Props) {
  const [showCreate, setShowCreate] = useState(false);
  const [loadingId, setLoadingId] = useState<string | null>(null);

  async function withLoading(id: string, fn: () => Promise<void>) {
    setLoadingId(id);
    try {
      await fn();
    } finally {
      setLoadingId(null);
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Servidores</h1>
        <button
          onClick={() => setShowCreate(true)}
          className="flex items-center gap-2 rounded-md bg-primary text-primary-foreground px-3 py-1.5 text-sm font-medium hover:bg-primary/90 transition-colors neon-primary"
        >
          <Plus className="h-4 w-4" /> Nuevo
        </button>
      </div>

      {servers.length === 0 ? (
        <div className="rounded-lg border border-dashed p-12 text-center text-muted-foreground">
          <p className="text-sm">
            Sin servidores. Crea uno pulsando <strong>Nuevo</strong>.
          </p>
        </div>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {servers.map((s) => (
            <ServerCard
              key={s.id}
              server={s}
              loading={loadingId === s.id}
              onStart={(id) => withLoading(id, () => onStart(id))}
              onStop={(id) => withLoading(id, () => onStop(id))}
              onDelete={(id) => withLoading(id, () => onDelete(id))}
              onConsole={() => onConsole(s)}
              onBackups={() => onBackups(s)}
              onMods={() => onMods(s)}
              onModpacks={() => onModpacks(s)}
            />
          ))}
        </div>
      )}

      {showCreate && (
        <CreateServerModal
          onClose={() => setShowCreate(false)}
          onCreate={onCreate}
        />
      )}
    </div>
  );
}
