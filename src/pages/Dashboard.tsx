import type { Server } from "../types";

interface Props {
  servers: Server[];
}

export function Dashboard({ servers }: Props) {
  const total    = servers.length;
  const running  = servers.filter((s) => s.status === "running").length;
  const crashed  = servers.filter((s) => s.status === "crashed").length;
  const offline  = servers.filter((s) => s.status === "offline").length;

  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Dashboard</h1>

      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <StatCard label="Total" value={total} />
        <StatCard label="Activos"   value={running} accent="green" />
        <StatCard label="Offline"   value={offline} />
        <StatCard label="Caídos"    value={crashed}  accent="red" />
      </div>

      {total === 0 && (
        <div className="rounded-lg border border-dashed p-12 text-center text-muted-foreground">
          <p className="text-sm">No hay servidores todavía.</p>
          <p className="text-xs mt-1">Ve a <strong>Servidores</strong> para crear el primero.</p>
        </div>
      )}

      {total > 0 && (
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
