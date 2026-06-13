export function Settings() {
  return (
    <div className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold">Configuración</h1>

      <div className="rounded-lg border bg-card p-6 flex flex-col gap-4">
        <Section title="Directorios">
          <ConfigRow label="Directorio de servidores"  value="/home/cubed/servers" />
          <ConfigRow label="Directorio de backups"     value="/home/cubed/backups" />
          <ConfigRow label="Directorio de descargas"   value="/home/cubed/downloads" />
        </Section>

        <Section title="Java por defecto">
          <ConfigRow label="Ruta" value="/usr/bin/java" />
        </Section>

        <Section title="Backups automáticos">
          <ConfigRow label="Intervalo" value="5 horas" />
        </Section>
      </div>

      <p className="text-xs text-muted-foreground">
        La configuración persistente se implementa en la Fase 2 (PostgreSQL).
      </p>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">{title}</h2>
      <div className="divide-y rounded-md border">{children}</div>
    </div>
  );
}

function ConfigRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between px-3 py-2 text-sm">
      <span className="text-muted-foreground">{label}</span>
      <code className="text-xs bg-muted rounded px-1.5 py-0.5">{value}</code>
    </div>
  );
}
