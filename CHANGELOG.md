# Changelog

Todos los cambios notables de Cubed se documentan aquí.
Formato basado en [Keep a Changelog](https://keepachangelog.com/es/1.1.0/)
y versionado [SemVer](https://semver.org/lang/es/).

## [Unreleased]

## [0.5.0] — Fase 5: Port Manager

### Added
- **Puerto `PortManager`** (`cubed-application/ports/port_manager.rs`) — `is_free`, `find_free_from`, `validate`.
- **`TcpPortManager`** (`cubed-infrastructure/port/`) — implementación real vía `TcpListener::bind`:
  - `is_free` intenta bind en `0.0.0.0:<port>` para detectar si está ocupado por el SO.
  - `find_free_from` itera desde `start` hasta 65535 buscando el primer puerto libre.
  - `validate` rechaza puertos < 1024 y puertos ocupados por el SO.
- **`ReservePort`** — caso de uso que combina validación de red + validación de BD (sin duplicados entre servidores de Cubed); también ofrece `suggest_free` para autocompletar.
- 6 tests con puertos reales (ocupar con `TcpListener::bind("0.0.0.0:0")` y verificar detección).

### Resultado
No se pueden crear servidores con puertos duplicados ni ocupados por el sistema.

## [0.4.0] — Fase 4: Java Manager

### Added
- **Puerto `JavaManager`** (`cubed-application/ports/java_manager.rs`) — `detect_installations`, `inspect`, `validate_compatibility`, `select_for_version`.
- **`JavaInstallation`** — struct con `path`, `major_version` y `version_string`.
- **`SystemJavaManager`** (`cubed-infrastructure/java/`) — implementación real:
  - Sondea candidatos estáticos (Ubuntu + macOS Homebrew), `which java` y `$JAVA_HOME`.
  - Parsea la salida de `java -version` (stderr) incluyendo el formato legacy `"1.8.x"`.
  - Ordena por versión descendente; `select_for_version` elige la mínima compatible.
- **Tabla de compatibilidad** Minecraft → Java mínimo:
  - `< 1.17` → Java 8 | `1.17.x` → 16 | `1.18–1.20.4` → 17 | `≥ 1.20.5` → 21.
- **`SelectJava`** — caso de uso con `list`, `for_version` e `inspect_and_validate`.
- 11 tests unitarios para parsing de versión, tabla de compatibilidad, validación y detección.

### Resultado
Cubed sabe si puede ejecutar Minecraft antes de intentar arrancar un servidor.

## [0.3.0] — Fase 3: File System Manager

### Added
- **Puerto `FileSystemManager`** (`cubed-application/ports/file_system.rs`) — trait async con `init_cubed_dirs`, `init_server_dirs`, `delete_server_dir`, `server_dir`, `ensure_writable`.
- **`LocalFileSystem`** (`cubed-infrastructure/fs/`) — implementación real sobre el FS local:
  - `init_cubed_dirs` → crea `/home/cubed/{servers,backups,downloads,temp,config,logs}`.
  - `init_server_dirs` → crea `<servers_dir>/<name>/{mods,world,config,logs}`.
  - `delete_server_dir` → elimina el árbol del servidor (no falla si no existe).
  - `ensure_writable` → valida acceso de escritura.
- **`InitFileSystem`** — caso de uso para inicializar la estructura global al arrancar.
- `CreateServer` ahora también crea el directorio del servidor tras persistirlo.
- `DeleteServer` ahora también elimina el directorio del servidor antes de borrar el registro.
- 5 tests unitarios en `LocalFileSystem` usando directorios temporales (`tempfile`).

### Resultado
Cubed puede generar servidores físicamente en disco.

## [0.2.0] — Fase 2: Persistencia

### Added
- **SQLx 0.9** (`runtime-tokio`, `tls-native-tls`, `postgres`, `uuid`, `chrono`, `migrate`) en workspace.
- **Migraciones SQL** (`cubed-infrastructure/migrations/`):
  - `0001_create_servers.sql` — tabla `servers` con restricción de puerto único.
  - `0002_create_backups.sql` — tabla `backups` con FK a `servers`.
  - `0003_create_settings.sql` — tabla `settings` (fila única con valores por defecto).
- **`PostgresServerRepository`** — implementación completa del puerto `ServerRepository`:
  - `save` con upsert (`ON CONFLICT (id) DO UPDATE`).
  - `find_by_id`, `find_all`, `delete`, `port_in_use`.
- **`ServerRow`** — mapeo `FromRow` de PostgreSQL → entidades de dominio con conversión de `software` y `status`.
- **`db::connect`** — crea el pool y ejecuta migraciones automáticamente al arrancar.
- Los servidores sobreviven al reinicio de Cubed.

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
