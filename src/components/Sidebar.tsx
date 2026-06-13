import { LayoutDashboard, Server, Settings } from "lucide-react";
import { cn } from "@/lib/utils";

type Page = "dashboard" | "servers" | "settings";

const NAV = [
  { id: "dashboard" as Page, label: "Dashboard",    Icon: LayoutDashboard },
  { id: "servers"   as Page, label: "Servidores",   Icon: Server          },
  { id: "settings"  as Page, label: "Configuración", Icon: Settings        },
];

interface Props {
  current: Page;
  onChange: (p: Page) => void;
}

export function Sidebar({ current, onChange }: Props) {
  return (
    <aside className="flex h-screen w-56 flex-col border-r bg-card">
      <div className="flex items-center gap-2 px-4 py-5 border-b">
        <div className="flex h-7 w-7 items-center justify-center rounded-md bg-primary text-primary-foreground text-sm font-bold">
          C
        </div>
        <span className="font-semibold tracking-tight">Cubed</span>
      </div>

      <nav className="flex flex-col gap-1 p-2 flex-1">
        {NAV.map(({ id, label, Icon }) => (
          <button
            key={id}
            onClick={() => onChange(id)}
            className={cn(
              "flex items-center gap-2.5 rounded-md px-3 py-2 text-sm font-medium transition-colors w-full text-left",
              current === id
                ? "bg-primary/10 text-primary"
                : "text-muted-foreground hover:bg-muted hover:text-foreground"
            )}
          >
            <Icon className="h-4 w-4 shrink-0" />
            {label}
          </button>
        ))}
      </nav>

      <div className="px-4 py-3 border-t">
        <p className="text-xs text-muted-foreground">Fase 8 · v0.8.0</p>
      </div>
    </aside>
  );
}

export type { Page };
