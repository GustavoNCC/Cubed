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

## Estado

Version 1.0
