import { useState } from "react";
import { X } from "lucide-react";
import type { CreateServerForm } from "../types";

const SOFTWARES = ["Paper", "Purpur", "Fabric", "Forge", "NeoForge"];

interface Props {
  onClose: () => void;
  onCreate: (form: CreateServerForm) => Promise<void>;
}

const DEFAULTS: CreateServerForm = {
  name: "",
  version: "1.21.4",
  software: "Paper",
  port: 25565,
  java_path: "/usr/bin/java",
  servers_dir: "/tmp/cubed-dev/servers",
};

export function CreateServerModal({ onClose, onCreate }: Props) {
  const [form, setForm] = useState<CreateServerForm>(DEFAULTS);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  function set<K extends keyof CreateServerForm>(key: K, value: CreateServerForm[K]) {
    setForm((f) => ({ ...f, [key]: value }));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      await onCreate(form);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <div className="w-full max-w-md rounded-xl border bg-card shadow-lg p-6 flex flex-col gap-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">Nuevo servidor</h2>
          <button onClick={onClose} className="text-muted-foreground hover:text-foreground">
            <X className="h-4 w-4" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="flex flex-col gap-3">
          <Field label="Nombre">
            <input
              required
              value={form.name}
              onChange={(e) => set("name", e.target.value)}
              placeholder="survival"
              className="input"
            />
          </Field>

          <div className="grid grid-cols-2 gap-3">
            <Field label="Versión Minecraft">
              <input
                required
                value={form.version}
                onChange={(e) => set("version", e.target.value)}
                placeholder="1.21.4"
                className="input"
              />
            </Field>
            <Field label="Puerto">
              <input
                required
                type="number"
                min={1024}
                max={65535}
                value={form.port}
                onChange={(e) => set("port", Number(e.target.value))}
                className="input"
              />
            </Field>
          </div>

          <Field label="Software">
            <select
              value={form.software}
              onChange={(e) => set("software", e.target.value)}
              className="input"
            >
              {SOFTWARES.map((s) => <option key={s}>{s}</option>)}
            </select>
          </Field>

          <Field label="Java (ruta absoluta)">
            <input
              required
              value={form.java_path}
              onChange={(e) => set("java_path", e.target.value)}
              placeholder="/usr/bin/java"
              className="input"
            />
          </Field>

          <Field label="Directorio de servidores">
            <input
              required
              value={form.servers_dir}
              onChange={(e) => set("servers_dir", e.target.value)}
              className="input"
            />
          </Field>

          {error && <p className="text-sm text-destructive">{error}</p>}

          <div className="flex gap-2 pt-1">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 rounded-md border px-4 py-2 text-sm hover:bg-muted transition-colors"
            >
              Cancelar
            </button>
            <button
              type="submit"
              disabled={loading}
              className="flex-1 rounded-md bg-primary text-primary-foreground px-4 py-2 text-sm font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors"
            >
              {loading ? "Creando…" : "Crear servidor"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-medium text-muted-foreground">{label}</span>
      {children}
    </label>
  );
}
