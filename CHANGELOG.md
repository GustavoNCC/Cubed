# Changelog

Todos los cambios notables de Cubed se documentan aquí.
Formato basado en [Keep a Changelog](https://keepachangelog.com/es/1.1.0/)
y versionado [SemVer](https://semver.org/lang/es/).

## [Unreleased]

## [0.1.0] — Fase 1: Core del Sistema

### Added
- **Entidades de dominio** (`cubed-domain`):
  - `Server` — agregado raíz con ciclo de vida completo (Offline → Starting → Running → Stopping → Crashed).
  - `Backup` — snapshot de un servidor con ruta y tamaño.
  - `ModEntry` — mod individual instalado en un servidor.
  - `Modpack` — modpack importado (`.mrpack` / `.zip`).
  - `Settings` — configuración global de Cubed con valores por defecto.
- **Value Objects** validados:
  - `ServerName` (1-64 chars, sin espacios).
  - `ServerPort` (>= 1024).
  - `ServerVersion` (formato X.Y o X.Y.Z).
  - `JavaPath` (ruta absoluta).
- **Errores de dominio** tipados: `Validation`, `InvalidTransition`, `ServerNotFound`.
- **Casos de uso** (`cubed-application`):
  - `CreateServer` — crea y persiste un servidor, rechaza puertos duplicados.
  - `DeleteServer` — elimina un servidor offline.
  - `StartServer` — transiciona a Starting.
  - `StopServer` — transiciona a Stopping.
  - `RestartServer` — cicla el servidor sin intervención manual.
- **Puerto** `ServerRepository` (trait async) para desacoplar persistencia.
- **Iconos PNG RGBA** mínimos para compilación de Tauri en dev.
- **27 tests unitarios e integración** (24 dominio + 2 aplicación + 1 infraestructura).

### Resultado
Cubed ya entiende qué es un servidor. El workspace compila y todos los tests pasan.

## [0.0.0] — Fase 0: Fundación del Proyecto

### Added
- Workspace Cargo multi-crate con las capas de Clean Architecture:
  `cubed-domain`, `cubed-application`, `cubed-infrastructure`.
- Aplicación Tauri v2 (`src-tauri`) como composition root, con comando
  `health_check` que verifica el puente Frontend ↔ Backend.
- Frontend base React + TypeScript + Vite con TailwindCSS y tokens de shadcn/ui.
- App vacía funcional que comprueba la conexión con el backend.
- Configuración de herramientas: ESLint, Prettier, Rustfmt y Clippy.
- Documentación inicial: `README.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`.
- Pipeline de CI (GitHub Actions) para lint + fmt + clippy + tests.
- `.gitignore` para Node, Rust y Tauri.

### Resultado
Aplicación vacía que compila y arranca. Fundación lista para la Fase 1 (Core del
dominio).
