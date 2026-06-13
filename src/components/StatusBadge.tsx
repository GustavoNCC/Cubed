import { cn } from "@/lib/utils";
import type { ServerStatus } from "../types";

const STATUS_STYLES: Record<ServerStatus, string> = {
  offline:  "bg-secondary text-secondary-foreground",
  starting: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
  running:  "bg-green-100  text-green-800  dark:bg-green-900  dark:text-green-200",
  stopping: "bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200",
  crashed:  "bg-red-100    text-red-800    dark:bg-red-900    dark:text-red-200",
};

const STATUS_LABEL: Record<ServerStatus, string> = {
  offline:  "Offline",
  starting: "Iniciando",
  running:  "Activo",
  stopping: "Deteniendo",
  crashed:  "Caído",
};

export function StatusBadge({ status }: { status: ServerStatus }) {
  return (
    <span className={cn("inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium", STATUS_STYLES[status])}>
      {STATUS_LABEL[status]}
    </span>
  );
}
