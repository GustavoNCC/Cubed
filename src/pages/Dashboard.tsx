import { useEffect, useState } from "react";
import { Cpu, HardDrive, MemoryStick, Network } from "lucide-react";
import { api } from "../api";
import type { Server, SystemStats } from "../types";

interface Props {
  servers: Server[];
}

function fmt_bytes(b: number): string {
  if (b >= 1e9) return `${(b / 1e9).toFixed(1)} GB`;
  if (b >= 1e6) return `${(b / 1e6).toFixed(0)} MB`;
  return `${(b / 1e3).toFixed(0)} KB`;
}

function pct(used: number, total: number): number {
  if (total === 0) return 0;
  return Math.round((used / total) * 100);
}

export function Dashboard({ servers }: Props) {
  const total   = servers.length;
  const running = servers.filter((s) => s.status === "running").length;
  const crashed = servers.filter((s) => s.status === "crashed").length;
  const offline = servers.filter((s) => s.status === "offline").length;

  const [sys, setSys] = useState<SystemStats | null>(null);

  useEffect(() => {
    let alive = true;
    async function poll() {
      while (alive) {
        try {
          const stats = await api.getSystemStats();
          if (alive) setSys(stats);
        } catch {
          // silently ignore — may fail in browser dev mode
        }
        await new Promise((r) => setTimeout(r, 3000));
      }
    }
    poll();
    return () => { alive = false; };
  }, []);

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Dashboard</h1>

      {/* Server counters */}
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <StatCard label="Total"   value={total} />
        <StatCard label="Activos" value={running} accent="green" />
        <StatCard label="Offline" value={offline} />
        <StatCard label="Caídos"  value={crashed} accent="red" />
      </div>

      {/* System resources */}
      {sys && (
        <div>
          <h2 className="text-sm font-medium text-muted-foreground mb-2">Recursos del sistema</h2>
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
            <ResourceCard
              icon={<Cpu className="h-4 w-4" />}
              label="CPU"
              value={`${sys.cpu_percent.toFixed(1)}%`}
              bar={sys.cpu_percent / 100}
            />
            <ResourceCard
              icon={<MemoryStick className="h-4 w-4" />}
              label="RAM"
              value={`${fmt_bytes(sys.ram_used_bytes)} / ${fmt_bytes(sys.ram_total_bytes)}`}
              bar={pct(sys.ram_used_bytes, sys.ram_total_bytes) / 100}
            />
            <ResourceCard
              icon={<HardDrive className="h-4 w-4" />}
              label="Disco"
              value={`${fmt_bytes(sys.disk_used_bytes)} / ${fmt_bytes(sys.disk_total_bytes)}`}
              bar={pct(sys.disk_used_bytes, sys.disk_total_bytes) / 100}
            />
            <ResourceCard
              icon={<Network className="h-4 w-4" />}
              label="Red ↓/↑"
              value={`${fmt_bytes(sys.net_rx_bytes)} / ${fmt_bytes(sys.net_tx_bytes)}`}
              bar={null}
            />
          </div>
        </div>
      )}

      {total === 0 ? (
        <div className="rounded-lg border border-dashed p-12 text-center text-muted-foreground">
          <p className="text-sm">No hay servidores todavía.</p>
          <p className="text-xs mt-1">Ve a <strong>Servidores</strong> para crear el primero.</p>
        </div>
      ) : (
        <div>
          <h2 className="text-sm font-medium text-muted-foreground mb-2">Actividad reciente</h2>
          <ul className="divide-y rounded-lg border bg-card">
            {servers.slice(0, 5).map((s) => (
              <li key={s.id} className="flex items-center justify-between px-4 py-3 text-sm">
                <span className="font-medium">{s.name}</span>
                <span className="text-muted-foreground">{s.software} {s.version} · :{s.port}</span>
                <span className={`text-xs ${s.status === "running" ? "text-green-600" : s.status === "crashed" ? "text-red-600" : "text-muted-foreground"}`}>
                  {s.status}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

function StatCard({ label, value, accent }: { label: string; value: number; accent?: "green" | "red" }) {
  return (
    <div className="rounded-lg border bg-card p-4 flex flex-col gap-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className={`text-3xl font-bold ${accent === "green" ? "text-green-600" : accent === "red" ? "text-red-600" : "text-foreground"}`}>
        {value}
      </span>
    </div>
  );
}

function ResourceCard({
  icon, label, value, bar,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  bar: number | null;
}) {
  return (
    <div className="rounded-lg border bg-card p-4 flex flex-col gap-2">
      <div className="flex items-center gap-1.5 text-muted-foreground text-xs">
        {icon}
        <span>{label}</span>
      </div>
      <span className="text-sm font-medium">{value}</span>
      {bar !== null && (
        <div className="h-1.5 rounded-full bg-muted overflow-hidden">
          <div
            className={`h-full rounded-full transition-all ${bar > 0.85 ? "bg-red-500" : bar > 0.65 ? "bg-yellow-500" : "bg-primary"}`}
            style={{ width: `${Math.min(bar * 100, 100)}%` }}
          />
        </div>
      )}
    </div>
  );
}
