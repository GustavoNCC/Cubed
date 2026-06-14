import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ChevronLeft, Send } from "lucide-react";
import { cn } from "@/lib/utils";
import { api } from "../api";
import { isTauriRuntime } from "../tauriRuntime";
import type { ConsoleLine, Server } from "../types";

interface Props {
  server: Server;
  onBack: () => void;
}

export function Console({ server, onBack }: Props) {
  const [lines, setLines] = useState<ConsoleLine[]>([]);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    async function setup() {
      if (!isTauriRuntime()) return;

      // Subscribe and get history
      const history = await api.subscribeConsole(server.id);
      setLines(history);

      // Listen for new lines via Tauri event
      unlisten = await listen<ConsoleLine>(
        `console-line:${server.id}`,
        (evt) => {
          setLines((prev) => [...prev.slice(-999), evt.payload]);
        },
      );
    }

    setup().catch(console.error);
    return () => {
      unlisten?.();
    };
  }, [server.id]);

  // Auto-scroll to bottom
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [lines]);

  async function handleSend(e: React.FormEvent) {
    e.preventDefault();
    const cmd = input.trim();
    if (!cmd) return;
    setSending(true);
    try {
      await api.sendConsoleCommand(server.id, cmd);
      setInput("");
    } catch (err) {
      console.error(err);
    } finally {
      setSending(false);
    }
  }

  return (
    <div className="flex flex-col h-full gap-4">
      <div className="flex items-center gap-3">
        <button
          onClick={onBack}
          className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          <ChevronLeft className="h-4 w-4" /> Volver
        </button>
        <h1 className="text-xl font-bold">
          Consola — <span className="text-primary">{server.name}</span>
        </h1>
        <span className="text-xs text-muted-foreground">
          {server.software} {server.version} · :{server.port}
        </span>
      </div>

      {/* Terminal */}
      <div className="flex-1 overflow-y-auto rounded-lg border bg-zinc-950 p-3 font-mono text-xs leading-relaxed min-h-0">
        {lines.length === 0 ? (
          <p className="text-zinc-500">
            Sin salida todavía. Inicia el servidor para ver la consola.
          </p>
        ) : (
          lines.map((l, i) => (
            <div
              key={i}
              className={cn(
                "whitespace-pre-wrap break-all",
                l.is_stdout ? "text-zinc-200" : "text-yellow-400",
              )}
            >
              {l.text}
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>

      {/* Input */}
      <form onSubmit={handleSend} className="flex gap-2">
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Escribe un comando (p.ej. say Hola)"
          disabled={sending}
          className="flex-1 rounded-md border bg-background px-3 py-2 text-sm font-mono outline-none focus:ring-2 focus:ring-ring transition-shadow"
        />
        <button
          type="submit"
          disabled={sending || !input.trim()}
          className="flex items-center gap-1.5 rounded-md bg-primary text-primary-foreground px-4 py-2 text-sm font-medium hover:bg-primary/90 disabled:opacity-50 transition-colors"
        >
          <Send className="h-3.5 w-3.5" />
          Enviar
        </button>
      </form>
    </div>
  );
}
