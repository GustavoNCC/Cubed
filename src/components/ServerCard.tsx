import { useState } from "react";
import {
  Play,
  Square,
  RotateCcw,
  Trash2,
  Terminal,
  Archive,
  Package,
  Layers,
  Check,
  Wifi,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { StatusBadge } from "./StatusBadge";
import { api } from "../api";
import { copyText } from "../clipboard";
import type { Server } from "../types";

interface Props {
  server: Server;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onRestart: (id: string) => void;
  onDelete: (id: string) => void;
  onConsole: () => void;
  onBackups: () => void;
  onMods: () => void;
  onModpacks: () => void;
  loading: boolean;
}

const SOFTWARE_COLORS: Record<string, string> = {
  Paper: "bg-red-500/15 text-red-400",
  Purpur: "bg-purple-500/15 text-purple-400",
  Fabric: "bg-blue-500/15 text-blue-400",
  Forge: "bg-orange-500/15 text-orange-400",
  NeoForge: "bg-amber-500/15 text-amber-400",
};

export function ServerCard({
  server,
  onStart,
  onStop,
  onRestart,
  onDelete,
  onConsole,
  onBackups,
  onMods,
  onModpacks,
  loading,
}: Props) {
  const canStart = server.status === "offline" || server.status === "crashed";
  const canStop = server.status === "running";
  const canRestart = server.status === "running";
  const canDelete = server.status === "offline" || server.status === "crashed";
  const [copied, setCopied] = useState(false);
  const [copyFailed, setCopyFailed] = useState(false);

  async function handleCopyAddress() {
    try {
      const addr = await api.serverConnectAddress(server.id);
      if (addr) {
        await copyText(addr);
        setCopied(true);
        setCopyFailed(false);
        setTimeout(() => setCopied(false), 2000);
      }
    } catch {
      setCopyFailed(true);
      setTimeout(() => setCopyFailed(false), 2000);
    }
  }

  const softwareColor =
    SOFTWARE_COLORS[server.software] ?? "bg-muted text-muted-foreground";

  return (
    <div className="rounded-xl border border-border bg-card shadow-sm overflow-hidden transition-shadow hover:shadow-md">
      {/* Header */}
      <div className="flex items-start justify-between gap-3 p-4 pb-3">
        <div className="min-w-0 flex-1">
          <p className="font-semibold text-card-foreground truncate text-base leading-tight">
            {server.name}
          </p>
          <div className="flex items-center gap-2 mt-1 flex-wrap">
            <span
              className={cn(
                "text-xs font-medium px-1.5 py-0.5 rounded",
                softwareColor,
              )}
            >
              {server.software}
            </span>
            <span className="text-xs text-muted-foreground">
              {server.version}
            </span>
            <span className="text-xs text-muted-foreground">·</span>
            <span className="text-xs text-muted-foreground font-mono">
              :{server.port}
            </span>
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

        <ActionBtn
          onClick={() => onRestart(server.id)}
          disabled={!canRestart || loading}
          variant="ghost"
          icon={<RotateCcw className="h-3.5 w-3.5" />}
        >
          Reiniciar
        </ActionBtn>

        <div className="ml-auto flex items-center gap-1">
          <ActionBtn
            onClick={handleCopyAddress}
            variant="ghost"
            icon={
              copied ? (
                <Check className="h-3.5 w-3.5 text-green-500" />
              ) : (
                <Wifi
                  className={cn(
                    "h-3.5 w-3.5",
                    copyFailed && "text-destructive",
                  )}
                />
              )
            }
            title="Copiar dirección Tailscale"
          >
            {copyFailed ? "Error" : copied ? "Copiado" : "Dirección"}
          </ActionBtn>

          <ActionBtn
            onClick={() => onDelete(server.id)}
            disabled={!canDelete || loading}
            variant="destructive"
            icon={<Trash2 className="h-3.5 w-3.5" />}
            title={
              canDelete
                ? "Eliminar servidor"
                : "Detén el servidor antes de eliminarlo"
            }
          >
            Eliminar
          </ActionBtn>
        </div>
      </div>

      {/* Actions — row 2: secondary */}
      <div className="px-4 pt-2 pb-3 flex items-center gap-1 flex-wrap">
        <ActionBtn
          onClick={onConsole}
          variant="ghost"
          icon={<Terminal className="h-3.5 w-3.5" />}
        >
          Consola
        </ActionBtn>
        <ActionBtn
          onClick={onBackups}
          variant="ghost"
          icon={<Archive className="h-3.5 w-3.5" />}
        >
          Backups
        </ActionBtn>
        <ActionBtn
          onClick={onMods}
          variant="ghost"
          icon={<Package className="h-3.5 w-3.5" />}
        >
          Mods
        </ActionBtn>
        <ActionBtn
          onClick={onModpacks}
          variant="ghost"
          icon={<Layers className="h-3.5 w-3.5" />}
        >
          Modpacks
        </ActionBtn>
      </div>
    </div>
  );
}

// ── ActionBtn ─────────────────────────────────────────────────────────────────

type BtnVariant =
  | "primary"
  | "danger-outline"
  | "destructive"
  | "ghost"
  | "outline";

const VARIANT_CLASSES: Record<BtnVariant, string> = {
  primary:
    "bg-primary text-primary-foreground hover:bg-primary/90 border-transparent",
  "danger-outline":
    "border-destructive/50 text-destructive hover:bg-destructive/10",
  destructive: "bg-destructive/10 text-destructive hover:bg-destructive/20",
  ghost:
    "border-transparent text-muted-foreground hover:text-foreground hover:bg-muted/50",
  outline: "border-border text-foreground hover:bg-muted/50",
};

function ActionBtn({
  onClick,
  disabled,
  variant,
  icon,
  title,
  children,
}: {
  onClick: () => void;
  disabled?: boolean;
  variant: BtnVariant;
  icon?: React.ReactNode;
  title?: string;
  children?: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      className={cn(
        "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md border text-xs font-medium transition-colors",
        "disabled:opacity-40 disabled:cursor-not-allowed",
        VARIANT_CLASSES[variant],
      )}
    >
      {icon}
      {children}
    </button>
  );
}
