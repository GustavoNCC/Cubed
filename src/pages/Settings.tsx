import { useEffect, useState } from "react";
import { Save, RefreshCw, CheckCircle, XCircle } from "lucide-react";
import { api } from "../api";
import type { SettingsDto } from "../types";

const INTERVAL_OPTIONS: { label: string; value: number }[] = [
  { label: "Desactivado", value: 0 },
  { label: "30 minutos",  value: 1_800 },
  { label: "1 hora",      value: 3_600 },
  { label: "3 horas",     value: 10_800 },
  { label: "5 horas",     value: 18_000 },
  { label: "12 horas",    value: 43_200 },
  { label: "24 horas",    value: 86_400 },
];

export function Settings() {
  const [form, setForm]       = useState<SettingsDto | null>(null);
  const [saving, setSaving]   = useState(false);
  const [success, setSuccess] = useState(false);
  const [error, setError]     = useState<string | null>(null);

  useEffect(() => {
    api.getSettings().then(setForm).catch((e) => setError(String(e)));
  }, []);

  function field(key: keyof SettingsDto) {
    return (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) =>
      setForm((prev) => prev ? { ...prev, [key]: key === "backup_interval_secs" ? Number(e.target.value) : e.target.value } : prev);
  }

  async function handleSave(e: React.FormEvent) {
    e.preventDefault();
    if (!form) return;
    setSaving(true);
    setError(null);
    setSuccess(false);
    try {
      const saved = await api.saveSettings(form);
      setForm(saved);
      setSuccess(true);
      setTimeout(() => setSuccess(false), 3000);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  if (!form) {
    return (
      <div className="flex items-center justify-center h-32 text-muted-foreground text-sm">
        {error ? (
          <span className="text-destructive">{error}</span>
        ) : (
          <RefreshCw className="h-4 w-4 animate-spin mr-2" />
        )}
        {!error && "Cargando configuración…"}
      </div>
    );
  }

  return (
    <form onSubmit={handleSave} className="flex flex-col gap-6 max-w-xl">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Configuración</h1>
        <span className="text-xs text-muted-foreground font-mono">v1.0.0</span>
      </div>

      {/* Directorios */}
      <Section title="Directorios">
        <Field
          label="Servidores"
          value={form.servers_dir}
          onChange={field("servers_dir")}
          placeholder="/home/cubed/servers"
        />
        <Field
          label="Backups"
          value={form.backups_dir}
          onChange={field("backups_dir")}
          placeholder="/home/cubed/backups"
        />
        <Field
          label="Descargas"
          value={form.downloads_dir}
          onChange={field("downloads_dir")}
          placeholder="/home/cubed/downloads"
        />
      </Section>

      {/* Java */}
      <Section title="Java por defecto">
        <Field
          label="Ruta ejecutable"
          value={form.default_java_path}
          onChange={field("default_java_path")}
          placeholder="/usr/bin/java"
        />
      </Section>

      {/* Backup automático */}
      <Section title="Backups automáticos">
        <div className="flex items-center justify-between px-3 py-2 text-sm">
          <label className="text-muted-foreground" htmlFor="backup-interval">
            Intervalo
          </label>
          <select
            id="backup-interval"
            value={form.backup_interval_secs}
            onChange={field("backup_interval_secs")}
            className="rounded border border-input bg-background px-2 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
          >
            {INTERVAL_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>{o.label}</option>
            ))}
          </select>
        </div>
        {form.backup_interval_secs === 0 && (
          <p className="px-3 pb-2 text-xs text-muted-foreground">
            Los backups automáticos están desactivados.
          </p>
        )}
      </Section>

      {/* Feedback */}
      {success && (
        <div className="flex items-center gap-2 rounded-md border border-green-500/50 bg-green-500/10 px-4 py-2 text-sm text-green-700 dark:text-green-400">
          <CheckCircle className="h-4 w-4 shrink-0" /> Configuración guardada correctamente.
        </div>
      )}
      {error && (
        <div className="flex items-center gap-2 rounded-md border border-destructive/50 bg-destructive/10 px-4 py-2 text-sm text-destructive">
          <XCircle className="h-4 w-4 shrink-0" /> {error}
        </div>
      )}

      <div className="flex justify-end">
        <button
          type="submit"
          disabled={saving}
          className="flex items-center gap-2 rounded-md bg-primary text-primary-foreground px-4 py-2 text-sm font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors"
        >
          {saving ? <RefreshCw className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
          {saving ? "Guardando…" : "Guardar cambios"}
        </button>
      </div>
    </form>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">{title}</h2>
      <div className="divide-y rounded-md border bg-card">{children}</div>
    </div>
  );
}

function Field({
  label, value, onChange, placeholder,
}: {
  label: string;
  value: string;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  placeholder?: string;
}) {
  return (
    <div className="flex items-center justify-between gap-4 px-3 py-2 text-sm">
      <span className="text-muted-foreground shrink-0">{label}</span>
      <input
        type="text"
        value={value}
        onChange={onChange}
        placeholder={placeholder}
        className="flex-1 min-w-0 rounded border border-input bg-background px-2 py-0.5 text-xs font-mono focus:outline-none focus:ring-2 focus:ring-ring text-right"
      />
    </div>
  );
}
