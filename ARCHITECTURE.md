# Arquitectura de Cubed

Cubed sigue **Clean Architecture**, **DDD** y **SOLID**. La regla de dependencia
es estricta: el código fuente solo apunta hacia adentro. La presentación y la
infraestructura dependen de la aplicación y el dominio; el dominio no depende de
nadie.

```
┌──────────────────────────────────────────────────────────────┐
│  Presentation  (src/  · React + Tailwind + shadcn/ui)          │
│      └── invoca comandos Tauri (no toca FS, procesos ni DB)     │
├──────────────────────────────────────────────────────────────┤
│  Tauri Bridge  (src-tauri/  · composition root)                │
│      └── ensambla las capas y expone #[tauri::command]          │
├──────────────────────────────────────────────────────────────┤
│  Application   (crates/cubed-application)                       │
│      └── casos de uso + puertos (traits)                        │
├──────────────────────────────────────────────────────────────┤
│  Domain        (crates/cubed-domain)                            │
│      └── entidades, value objects, reglas de negocio            │
├──────────────────────────────────────────────────────────────┤
│  Infrastructure(crates/cubed-infrastructure)                    │
│      └── PostgreSQL/SQLx, FS, procesos Java, Tailscale, APIs     │
└──────────────────────────────────────────────────────────────┘
```

## Mapeo de capas a crates

| Capa            | Crate / carpeta             | Depende de            |
| --------------- | --------------------------- | --------------------- |
| Dominio         | `cubed-domain`              | (nada)                |
| Aplicación      | `cubed-application`         | domain                |
| Infraestructura | `cubed-infrastructure`      | domain, application   |
| Presentación    | `src/` (React)              | comandos Tauri        |
| Composition root| `src-tauri/`                | las tres capas Rust   |

## Principios

- **Inversión de dependencias**: la aplicación define traits (puertos) que la
  infraestructura implementa. El dominio nunca importa SQLx, Tauri ni `std::process`.
- **Frontend tonto respecto al sistema**: la UI solo muestra datos y dispara
  comandos `invoke()`. Toda acción cruza el puente Tauri.
- **Testabilidad**: dominio y aplicación se testean sin DB ni filesystem mediante
  dobles de prueba de los puertos.

## Módulos previstos (por Roadmap)

Server Manager, Java Manager, Port Manager, Console Manager, Resource Manager,
Backup Manager, Mod Manager, Modpack Manager, Downloader Manager, Network Manager
(Tailscale), Database Layer (PostgreSQL/SQLx), File System Manager y Event System
(eventos Tauri). Cada uno se implementa en la fase correspondiente del Roadmap.

## Decisiones (ADR breves)

- **Tauri v2 sobre Electron**: menor consumo, integración nativa con Rust, buen
  soporte Linux.
- **Workspace Cargo multi-crate**: hace explícita y verificable la frontera entre
  capas (un crate no puede importar otro si no está declarado).
- **PostgreSQL + SQLx** sobre archivos planos: consultas tipadas y verificadas,
  escalable a multiusuario/roles en el futuro.
