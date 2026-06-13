# Cubed

> Plataforma local de administración de servidores Minecraft Java para Ubuntu Linux.

Cubed permite **crear, administrar, monitorear y compartir** múltiples servidores
de Minecraft desde una interfaz gráfica moderna, sin depender de hosting externo.
La experiencia busca acercarse a un panel profesional (estilo Azure) pero corriendo
por completo en la máquina del usuario.

## Stack

| Capa        | Tecnología                                   |
| ----------- | -------------------------------------------- |
| Escritorio  | Tauri v2                                      |
| Frontend    | React + TypeScript + TailwindCSS + shadcn/ui |
| Backend     | Rust                                          |
| Persistencia| PostgreSQL + SQLx                             |
| Red         | Tailscale                                     |

La arquitectura sigue **Clean Architecture + DDD + SOLID**. Ver
[`ARCHITECTURE.md`](./ARCHITECTURE.md).

## Estructura del proyecto

```
cubed/
├── src/                      # Frontend React (capa de presentación)
├── src-tauri/                # App Tauri + composition root (Rust)
├── crates/
│   ├── cubed-domain/         # Capa de Dominio (reglas de negocio)
│   ├── cubed-application/    # Capa de Aplicación (casos de uso, puertos)
│   └── cubed-infrastructure/ # Capa de Infraestructura (PostgreSQL, FS, Java…)
├── Cargo.toml                # Workspace Rust
└── package.json              # Frontend + scripts Tauri
```

## Requisitos

- Node.js 20+ y npm
- Rust (toolchain estable) + `cargo`
- Dependencias de sistema de Tauri en Linux: ver
  <https://v2.tauri.app/start/prerequisites/>
- (Más adelante) PostgreSQL 15+

## Puesta en marcha

```bash
# 1. Instalar dependencias del frontend
npm install

# 2. Ejecutar en modo desarrollo (lanza Vite + ventana Tauri)
npm run tauri dev

# 3. Build de producción (.deb / .AppImage)
npm run tauri build
```

## Calidad de código

```bash
npm run lint          # ESLint (frontend)
npm run format        # Prettier
cargo fmt --all       # Rustfmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Estado

Fase 0 completada (fundación del proyecto). Ver el progreso por fases en
[`CHANGELOG.md`](./CHANGELOG.md) y el plan en el Roadmap.

## Licencia

MIT.
