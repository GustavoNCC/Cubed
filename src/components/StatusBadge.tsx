import { cn } from "@/lib/utils";
import type { ServerStatus } from "../types";

const STATUS_STYLES: Record<ServerStatus, string> = {
  offline: "bg-muted/60 text-muted-foreground border border-border",
  starting: "bg-yellow-500/10 text-yellow-300 border border-yellow-500/30",
  running: "bg-primary/10 text-primary border border-primary/30 neon-primary",
  stopping: "bg-orange-500/10 text-orange-300 border border-orange-500/30",
  crashed: "bg-destructive/10 text-destructive border border-destructive/30",
};

const STATUS_DOT: Record<ServerStatus, string> = {
  offline: "bg-muted-foreground",
  starting: "bg-yellow-400 animate-pulse",
  running: "bg-primary animate-pulse",
  stopping: "bg-orange-400 animate-pulse",
  crashed: "bg-destructive",
};

const STATUS_LABEL: Record<ServerStatus, string> = {
  offline: "Offline",
  starting: "Iniciando",
  running: "Activo",
  stopping: "Deteniendo",
  crashed: "Caído",
};

export function StatusBadge({ status }: { status: ServerStatus }) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium",
        STATUS_STYLES[status],
      )}
    >
      <span
        className={cn("h-1.5 w-1.5 rounded-full shrink-0", STATUS_DOT[status])}
      />
      {STATUS_LABEL[status]}
    </span>
  );
}
