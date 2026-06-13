import { LayoutDashboard, Server, Settings } from "lucide-react";
import { cn } from "@/lib/utils";
import { CubedLogo } from "./CubedLogo";

type Page = "dashboard" | "servers" | "settings";

const NAV = [
  { id: "dashboard" as Page, label: "Dashboard", Icon: LayoutDashboard },
  { id: "servers" as Page, label: "Servidores", Icon: Server },
  { id: "settings" as Page, label: "Configuración", Icon: Settings },
];

interface Props {
  current: Page;
  onChange: (p: Page) => void;
}

export function Sidebar({ current, onChange }: Props) {
  return (
    <aside className="relative flex h-screen w-56 flex-col border-r border-border bg-card overflow-hidden">
      {/* Neon left-edge accent */}
      <div className="absolute left-0 top-0 bottom-0 w-[2px] bg-gradient-to-b from-transparent via-primary to-transparent opacity-70" />

      {/* Logo */}
      <div className="flex items-center gap-3 px-4 py-5 border-b border-border">
        <div className="shrink-0 drop-shadow-[0_0_6px_rgba(168,85,247,0.6)]">
          <CubedLogo size={34} />
        </div>
        <div className="flex flex-col leading-none">
          <span className="font-bold tracking-wider text-sm text-foreground">
            CUBED
          </span>
          <span className="text-[10px] text-muted-foreground font-mono tracking-widest">
            SERVER MGR
          </span>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex flex-col gap-0.5 p-2 flex-1">
        {NAV.map(({ id, label, Icon }) => (
          <button
            key={id}
            onClick={() => onChange(id)}
            className={cn(
              "group relative flex items-center gap-2.5 rounded-md px-3 py-2.5 text-sm font-medium transition-all duration-150 w-full text-left",
              current === id
                ? "bg-primary/10 text-primary border border-primary/30 neon-border"
                : "text-muted-foreground hover:bg-muted hover:text-foreground border border-transparent",
            )}
          >
            {/* Active indicator bar */}
            {current === id && (
              <span className="absolute left-0 top-1/2 -translate-y-1/2 h-5 w-[3px] rounded-r-full bg-primary" />
            )}
            <Icon
              className={cn(
                "h-4 w-4 shrink-0 transition-colors",
                current === id ? "text-primary" : "group-hover:text-primary/70",
              )}
            />
            {label}
          </button>
        ))}
      </nav>

      {/* Footer */}
      <div className="px-4 py-3 border-t border-border">
        <div className="flex items-center justify-between">
          <p className="text-xs text-muted-foreground font-mono">v1.0.0</p>
          <span className="inline-flex items-center gap-1 text-[10px] text-accent/80 font-mono">
            <span className="h-1.5 w-1.5 rounded-full bg-accent animate-pulse" />
            online
          </span>
        </div>
      </div>
    </aside>
  );
}

export type { Page };
