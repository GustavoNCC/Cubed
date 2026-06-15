import { useState, useEffect, useRef } from "react";
import { X, RefreshCw, HelpCircle, Cpu, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { api } from "../api";
import type { CreateServerForm, JavaInstallationDto } from "../types";

const SOFTWARE_OPTIONS = [
  {
    value: "Paper",
    label: "Paper",
    description:
      "El más popular. Alto rendimiento, compatible con plugins Bukkit/Spigot.",
    badge: "Recomendado",
    badgeClass: "bg-primary/20 text-primary",
    mods: false,
  },
  {
    value: "Purpur",
    label: "Purpur",
    description:
      "Como Paper pero con más opciones de configuración y características extra.",
    badge: "Plugins",
    badgeClass: "bg-purple-500/20 text-purple-400",
    mods: false,
  },
  {
    value: "Fabric",
    label: "Fabric",
    description: "Ligero y moderno. Compatible con mods .jar de Fabric/Quilt.",
    badge: "Mods",
    badgeClass: "bg-blue-500/20 text-blue-400",
    mods: true,
  },
  {
    value: "Forge",
    label: "Forge",
    description:
      "El estándar clásico para modpacks. Compatible con la mayoría de mods.",
    badge: "Mods",
    badgeClass: "bg-orange-500/20 text-orange-400",
    mods: true,
  },
  {
    value: "NeoForge",
    label: "NeoForge",
    description:
      "Sucesor moderno de Forge. Más activo y actualizado para 1.20+.",
    badge: "Mods",
    badgeClass: "bg-amber-500/20 text-amber-400",
    mods: true,
  },
];

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
  const [portLoading, setPortLoading] = useState(false);
  const [portSuggested, setPortSuggested] = useState(false);
  const [javaList, setJavaList] = useState<JavaInstallationDto[]>([]);
  const [javaLoading, setJavaLoading] = useState(false);
  const [javaAutoSet, setJavaAutoSet] = useState(false);
  const [showJavaPicker, setShowJavaPicker] = useState(false);
  const versionDebounce = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Al montar: sugerir puerto, detectar Java y cargar servers_dir real desde settings
  useEffect(() => {
    suggestPort();
    detectJava(DEFAULTS.version);
    api.getSettings().then((s) => set("servers_dir", s.servers_dir)).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function set<K extends keyof CreateServerForm>(
    key: K,
    value: CreateServerForm[K],
  ) {
    setForm((f) => ({ ...f, [key]: value }));
  }

  async function suggestPort() {
    setPortLoading(true);
    try {
      const port = await api.suggestFreePort();
      setForm((f) => ({ ...f, port }));
      setPortSuggested(true);
    } catch {
      // keep default
    } finally {
      setPortLoading(false);
    }
  }

  // Detect Java and auto-select best match for the given MC version
  async function detectJava(mcVersion: string) {
    setJavaLoading(true);
    setJavaAutoSet(false);
    try {
      const [list, best] = await Promise.all([
        api.detectJava(),
        api.selectJavaForVersion(mcVersion).catch(() => null),
      ]);
      setJavaList(list);
      if (best) {
        setForm((f) => ({ ...f, java_path: best.path }));
        setJavaAutoSet(true);
      }
    } catch {
      // ignore — user can set manually
    } finally {
      setJavaLoading(false);
    }
  }

  function handleVersionChange(v: string) {
    set("version", v);
    setJavaAutoSet(false);
    if (versionDebounce.current) clearTimeout(versionDebounce.current);
    versionDebounce.current = setTimeout(() => {
      if (v.match(/^\d+\.\d+/)) detectJava(v);
    }, 500);
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

  const selectedSoftware = SOFTWARE_OPTIONS.find(
    (s) => s.value === form.software,
  );

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm p-4">
      <div className="w-full max-w-lg rounded-xl border border-border bg-card shadow-2xl shadow-primary/10 flex flex-col overflow-hidden neon-border">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border bg-card">
          <h2 className="text-base font-semibold text-foreground">
            Nuevo servidor
          </h2>
          <button
            onClick={onClose}
            className="text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <form
          onSubmit={handleSubmit}
          className="flex flex-col gap-4 px-6 py-4 overflow-y-auto max-h-[80vh]"
        >
          {/* Nombre */}
          <Field label="Nombre del servidor">
            <input
              required
              value={form.name}
              onChange={(e) => set("name", e.target.value)}
              placeholder="survival, creative, smp..."
              className="input"
              maxLength={64}
            />
          </Field>

          {/* Versión + Puerto */}
          <div className="grid grid-cols-2 gap-3">
            <Field label="Versión de Minecraft">
              <input
                required
                value={form.version}
                onChange={(e) => handleVersionChange(e.target.value)}
                placeholder="1.21.4"
                className="input"
              />
            </Field>

            <Field label="Puerto">
              <div className="flex gap-1.5">
                <input
                  required
                  type="number"
                  min={1024}
                  max={65535}
                  value={form.port}
                  onChange={(e) => {
                    set("port", Number(e.target.value));
                    setPortSuggested(false);
                  }}
                  className={cn(
                    "input flex-1 min-w-0",
                    portSuggested && "border-primary/50",
                  )}
                />
                <button
                  type="button"
                  onClick={suggestPort}
                  disabled={portLoading}
                  title="Buscar puerto libre"
                  className="flex items-center justify-center w-9 rounded-md border border-border bg-muted hover:bg-muted/80 transition-colors disabled:opacity-40"
                >
                  <RefreshCw
                    className={cn(
                      "h-3.5 w-3.5 text-muted-foreground",
                      portLoading && "animate-spin",
                    )}
                  />
                </button>
              </div>
              {portSuggested && (
                <p className="text-xs text-primary mt-0.5">
                  Puerto libre detectado
                </p>
              )}
            </Field>
          </div>

          {/* Software */}
          <Field label="Software del servidor">
            <div className="grid grid-cols-1 gap-1.5">
              {SOFTWARE_OPTIONS.map((opt) => (
                <label
                  key={opt.value}
                  className={cn(
                    "flex items-start gap-3 rounded-lg border p-3 cursor-pointer transition-colors",
                    form.software === opt.value
                      ? "border-primary/50 bg-primary/5 neon-border"
                      : "border-border hover:border-muted-foreground/50 hover:bg-muted/30",
                  )}
                >
                  <input
                    type="radio"
                    name="software"
                    value={opt.value}
                    checked={form.software === opt.value}
                    onChange={() => set("software", opt.value)}
                    className="mt-0.5 accent-primary"
                  />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium">{opt.label}</span>
                      <span
                        className={cn(
                          "text-xs px-1.5 py-0.5 rounded font-medium",
                          opt.badgeClass,
                        )}
                      >
                        {opt.badge}
                      </span>
                    </div>
                    <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">
                      {opt.description}
                    </p>
                  </div>
                </label>
              ))}
            </div>
          </Field>

          {/* Java — auto-detected */}
          <Field label="Java">
            <div className="flex gap-1.5 items-start flex-col">
              <div className="relative w-full">
                <div className="flex gap-1.5">
                  <div className="relative flex-1">
                    <input
                      required
                      value={form.java_path}
                      onChange={(e) => {
                        set("java_path", e.target.value);
                        setJavaAutoSet(false);
                      }}
                      placeholder="/usr/bin/java"
                      className={cn(
                        "input font-mono text-xs pr-8",
                        javaAutoSet && "border-primary/50",
                      )}
                    />
                    {javaLoading && (
                      <RefreshCw className="absolute right-2 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground animate-spin" />
                    )}
                  </div>
                  {javaList.length > 0 && (
                    <button
                      type="button"
                      onClick={() => setShowJavaPicker((v) => !v)}
                      title="Seleccionar Java instalado"
                      className="flex items-center justify-center w-9 rounded-md border border-border bg-muted hover:bg-muted/80 transition-colors"
                    >
                      <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
                    </button>
                  )}
                </div>

                {/* Java picker dropdown */}
                {showJavaPicker && javaList.length > 0 && (
                  <div className="absolute left-0 right-0 top-full mt-1 z-10 rounded-md border border-border bg-card shadow-lg shadow-black/40 overflow-hidden">
                    {javaList.map((j) => (
                      <button
                        key={j.path}
                        type="button"
                        onClick={() => {
                          set("java_path", j.path);
                          setJavaAutoSet(true);
                          setShowJavaPicker(false);
                        }}
                        className={cn(
                          "w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-muted transition-colors",
                          form.java_path === j.path && "bg-primary/10",
                        )}
                      >
                        <Cpu className="h-3.5 w-3.5 text-primary shrink-0" />
                        <div className="min-w-0">
                          <p className="text-xs font-mono truncate">{j.path}</p>
                          <p className="text-[10px] text-muted-foreground">
                            Java {j.major_version}
                          </p>
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {javaAutoSet && !javaLoading && (
                <p className="text-xs text-primary flex items-center gap-1">
                  <Cpu className="h-3 w-3" />
                  Java detectado automáticamente para Minecraft {form.version}
                </p>
              )}
              {!javaLoading && javaList.length === 0 && (
                <p className="text-xs text-amber-400">
                  No se encontró Java. Instala OpenJDK:{" "}
                  <code>sudo apt install openjdk-21-jdk</code>
                </p>
              )}
            </div>
          </Field>

          {/* Avanzado */}
          <details className="group">
            <summary className="text-xs text-muted-foreground cursor-pointer select-none flex items-center gap-1 hover:text-foreground transition-colors list-none">
              <HelpCircle className="h-3.5 w-3.5" />
              Directorio de servidores
            </summary>
            <div className="mt-3">
              <Field label="Directorio">
                <input
                  required
                  value={form.servers_dir}
                  onChange={(e) => set("servers_dir", e.target.value)}
                  className="input font-mono text-xs"
                />
              </Field>
            </div>
          </details>

          {/* Software hint for mods */}
          {selectedSoftware?.mods && (
            <div className="rounded-md bg-accent/10 border border-accent/20 px-3 py-2 text-xs text-accent/90">
              {form.software} es compatible con mods .jar. Podrás instalarlos
              desde la sección Mods una vez creado el servidor.
            </div>
          )}

          {error && <p className="text-sm text-destructive">{error}</p>}

          <div className="flex gap-2 pt-1">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 rounded-md border border-border px-4 py-2 text-sm hover:bg-muted transition-colors"
            >
              Cancelar
            </button>
            <button
              type="submit"
              disabled={loading}
              className="flex-1 rounded-md bg-primary text-primary-foreground px-4 py-2 text-sm font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors neon-primary"
            >
              {loading ? "Creando…" : "Crear servidor"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
        {label}
      </span>
      {children}
    </label>
  );
}
