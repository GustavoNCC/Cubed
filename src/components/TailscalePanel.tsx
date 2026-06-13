import { useEffect, useState } from "react";
import { Wifi, WifiOff, AlertCircle, Copy, Check } from "lucide-react";
import { api } from "../api";
import type { TailscaleStatusDto } from "../types";

export function TailscalePanel() {
  const [status, setStatus]   = useState<TailscaleStatusDto | null>(null);
  const [copied, setCopied]   = useState(false);

  useEffect(() => {
    let alive = true;
    async function poll() {
      while (alive) {
        try {
          const s = await api.tailscaleStatus();
          if (alive) setStatus(s);
        } catch {
          // silently ignore — may fail in browser dev mode
        }
        await new Promise((r) => setTimeout(r, 5000));
      }
    }
    poll();
    return () => { alive = false; };
  }, []);

  async function copyIp() {
    if (!status?.ip) return;
    try {
      await navigator.clipboard.writeText(status.ip);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch { /* clipboard not available */ }
  }

  if (!status) return null;

  return (
    <div className="rounded-lg border bg-card p-4 flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground font-medium">Tailscale</span>
        <StatusIcon state={status.state} />
      </div>

      {status.state === "not_installed" && (
        <p className="text-sm text-muted-foreground">No instalado</p>
      )}
      {status.state === "disconnected" && (
        <p className="text-sm text-muted-foreground">Desconectado</p>
      )}
      {status.state === "connected" && (
        <div className="flex items-center justify-between gap-2">
          <div>
            <p className="text-sm font-medium font-mono">{status.ip}</p>
            {status.hostname && (
              <p className="text-xs text-muted-foreground">{status.hostname}</p>
            )}
          </div>
          <button
            onClick={copyIp}
            title="Copiar IP"
            className="flex items-center gap-1 rounded px-2 py-1 text-xs text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
          >
            {copied ? <Check className="h-3.5 w-3.5 text-green-500" /> : <Copy className="h-3.5 w-3.5" />}
            {copied ? "Copiado" : "Copiar"}
          </button>
        </div>
      )}
    </div>
  );
}

function StatusIcon({ state }: { state: string }) {
  if (state === "connected")
    return <Wifi className="h-4 w-4 text-primary" />;
  if (state === "disconnected")
    return <WifiOff className="h-4 w-4 text-accent" />;
  return <AlertCircle className="h-4 w-4 text-muted-foreground" />;
}
