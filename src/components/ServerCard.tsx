import { useState } from "react";
import {
  Play, Square, Trash2, Terminal, Archive,
  Package, Layers, Check, Wifi,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { StatusBadge } from "./StatusBadge";
import { api } from "../api";
import type { Server } from "../types";

interface Props {
  server: Server;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onDelete: (id: string) => void;
  onConsole: () => void;
  onBackups: () => void;
  onMods: () => void;
  onModpacks: () => void;
  loading: boolean;
}

const SOFTWARE_COLORS: Record<string, string> = {
  Paper:    "bg-red-500/15 text-red-400",
  Purpur:   "bg-purple-500/15 text-purple-400",
  Fabric:   "bg-blue-500/15 text-blue-400",
  Forge:    "bg-orange-500/15 text-orange-400",
  NeoForge: "bg-amber-500/15 text-amber-400",
};

export function ServerCard({
  server, onStart, onStop, onDelete,
  onConsole, onBackups, onMods, onModpacks, loading,
}: Props) {
  const canStart  = server.status === "offline" || server.status === "crashed";
  const canStop   = server.status === "running";
  const canDelete = server.status === "offline" || server.status === "crashed";
  const [copied, setCopied] = useState(false);

  async function handleCopyAddress() {
    try {
      const addr = await api.serverConnectAddress(server.id);
      if (addr) {
        await navigator.clipboard.writeText(addr);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }
    } catch { /* ignore */ }
  }

  const softwareColor = SOFTWARE_COLORS[server.software] ?? "bg-muted text-muted-foreground";

  return (
    <div className="rounded-xl border border-border bg-card shadow-sm overflow-hidden transition-shadow hover:shadow-md">
      {/* Header */}
      <div className="flex items-start justify-between gap-3 p-4 pb-3">
        <div className="min-w-0 flex-1">
          <p className="font-semibold text-card-foreground truncate text-base leading-tight">
            {server.name}
          </p>
          <div className="flex items-center gap-2 mt-1 flex-wrap">
            <span className={cn("text-xs font-medium px-1.5 py-0.5 rounded", softwareColor)}>
              {server.software}
            </span>
            <span className="text-xs text-muted-foreground">{server.version}</span>
            <span className="text-xs text-muted-foreground">·</span>
            <span className="text-xs text-muted-foreground font-mono">:{server.port}</span>
          </div>
        </div>
        <StatusBadge status={server.status} />
      </div>

      {/* Divider */}
      <div className="h-px bg-border mx-4" />

      {/* Actions — row 1: primary */}
      <div className="px-4 pt-3 flex items-center gap-2 flex-wrap">
        <ActionBtn
          onClick={() => onStart(server.id)}
          disabled={!canStart || loading}
          variant="primary"
          icon={<Play className="h-3.5 w-3.5" />}
        >
          Iniciar
        </ActionBtn>

        <ActionBtn
          onClick={() => onStop(server.id)}
          disabled={!canStop || loading}
          variant="danger-outline"
          icon={<Square className="h-3.5 w-3.5" />}
        >
          Detener
        </ActionBtn>

        <div className="ml-auto flex items-center gap-1">
          <ActionBtn
            onClick={handleCopyAddress}
            variant="ghost"
            icon={copied
              ? <Check className="h-3.5 w-3.5 text-green-500" />
              : <Wifi className="h-3.5 w-3.5" />}
            title="Copiar dirección Tailscale"
          >
            {copied ? "Copiado" : "Dirección"}
          </ActionBtn>

          <ActionBtn
            onClick={() => onDelete(server.id)}
            disabled={!canDelete || loading}
            variant="destructive"
            icon={<Trash2 className="h-3.5 w-3.5" />}
            title={canDelete ? "Eliminar servidor" : "Detén el servidor antes de eliminarlo"}
          >
            Eliminar
          </ActionBtn>
        </div>
      </div>

      {/* Actions — row 2: secondary */}
      <div className="px-4 pt-2 pb-4 flex items-center gap-2 flex-wrap">
        <NavBtn onClick={onConsole} icon={<Terminal className="h-3.5 w-3.5" />}>Consola</NavBtn>
        <NavBtn onClick={onBackups} icon={<Archive className="h-3.5 w-3.5" />}>Backups</NavBtn>
        <NavBtn onClick={onMods} icon={<Package className="h-3.5 w-3.5" />}>Mods</NavBtn>
        <NavBtn onClick={onModpacks} icon={<Layers className="h-3.5 w-3.5" />}>Modpacks</NavBtn>
      </div>
    </div>
  );
}

/* ── Sub-components ───────────────────────────────────────────────────────── */

type Variant = "primary" | "danger-outline" | "destructive" | "ghost";

function ActionBtn({
  children, onClick, disabled, variant, icon, title,
}: {
  children?: React.ReactNode;
  onClick: () => void;
  disabled?: boolean;
  variant: Variant;
  icon: React.ReactNode;
  title?: string;
}) {
  const base = "inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-40";
  const styles: Record<Variant, string> = {
    primary:       "bg-primary text-primary-foreground hover:bg-primary/90",
    "danger-outline": "border border-destructive/50 text-destructive hover:bg-destructive/10",
    destructive:   "text-destructive hover:bg-destructive/10",
    ghost:         "text-muted-foreground hover:text-foreground hover:bg-muted",
  };
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      className={cn(base, styles[variant])}
    >
      {icon}{children}
    </button>
  );
}

function NavBtn({ children, onClick, icon }: { children: React.ReactNode; onClick: () => void; icon: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      className="inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium text-muted-foreground bg-muted/50 hover:bg-muted hover:text-foreground transition-colors"
    >
      {icon}{children}
    </button>
  );
}
