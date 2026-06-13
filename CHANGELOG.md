# Changelog

Todos los cambios notables de Cubed se documentan aquí.
Formato basado en [Keep a Changelog](https://keepachangelog.com/es/1.1.0/)
y versionado [SemVer](https://semver.org/lang/es/).

## [Unreleased]

## [0.14.0] — Fase 14: Network Manager (Tailscale)

### Added
- **Puerto `NetworkManager`** (`cubed-application/src/ports/network_manager.rs`):
  - `TailscaleStatus` enum: `NotInstalled`, `Disconnected`, `Connected { ip, hostname }`.
  - Trait async `is_installed()`, `status()`, `tailscale_ip()`.
- **`TailscaleNetworkManager`** (`cubed-infrastructure/src/network/`):
  - Detecta el binario `tailscale` en PATH y rutas conocidas (macOS, Linux, Windows).
  - Ejecuta `tailscale status --json` y parsea `BackendState`, `Self.TailscaleIPs[0]`, `Self.HostName`.
  - 3 tests unitarios (is_installed, status, ip consistent).
- **4 comandos Tauri nuevos** (`src-tauri/src/commands.rs`):
  - `tailscale_is_installed` → `bool`.
  - `tailscale_status` → `TailscaleStatusDto { state, ip, hostname }`.
  - `tailscale_ip` → `Option<String>`.
  - `server_connect_address(server_id)` → `Option<String>` (`<ts_ip>:<port>`).
- **`TailscalePanel`** (`src/components/TailscalePanel.tsx`):
  - Muestra estado Tailscale (no instalado / desconectado / conectado + IP).
  - Polling cada 5 s para mantener estado fresco.
  - Botón "Copiar IP" al portapapeles.
  - Integrado en el Dashboard.
- **Botón "Dirección"** en `ServerCard`:
  - Llama a `server_connect_address` y copia `<ts_ip>:<port>` al portapapeles.
  - Feedback visual "Copiado ✓" durante 2 s.

### Tests
- 51 tests pasando (`cargo test`). Build frontend: ✓ (211 KB JS gzip: 63 KB).

## [0.13.0] — Fase 13: Modpack Manager

### Added
- **Puerto `ModpackRepository`** (`cubed-application/src/ports/modpack_repository.rs`):
  - `save`, `find_by_id`, `find_by_server`, `delete`.
- **Caso de uso `ImportModpack`** (`cubed-application/src/use_cases/import_modpack.rs`):
  - Detecta formato por extensión (`.mrpack` → Modrinth, `.zip` → CurseForge/genérico).
  - 3 tests unitarios.
- **`InMemoryModpackRepo`** (`cubed-infrastructure/src/modpacks/`) — repositorio en memoria.
- **`ModpackInstaller`** (`cubed-infrastructure/src/modpacks/`):
  - **`.mrpack`**: lee `modrinth.index.json` del ZIP, filtra archivos server-side, descarga cada uno desde sus mirrors con `reqwest`, extrae info de loaders (Fabric, Forge, etc.).
  - **`.zip` CurseForge**: lee `manifest.json`, intenta descargar desde URLs directas (omite los que requieren API key).
  - **`.zip` genérico**: extrae todos los `.jar` directamente al directorio `mods/`.
  - Emite progreso `InstallProgress { total, done, current_file }` vía callback.
  - 3 tests unitarios (path inválido, formato no soportado, .mrpack real).
- **Dependencia `zip 2`** añadida a cubed-infrastructure.
- **3 comandos Tauri nuevos** (`src-tauri/src/commands.rs`):
  - `install_modpack(server_id, source_path, install_dir)` → `InstallSummaryDto` (emite eventos `modpack-progress:<id>`).
  - `list_modpacks(server_id)` → `Vec<ModpackDto>`.
  - `delete_modpack(modpack_id)`.
- **Frontend — `Modpacks.tsx`** (`src/pages/Modpacks.tsx`):
  - Instalación con barra de progreso en tiempo real (eventos Tauri).
  - Resumen post-instalación: archivos descargados, omitidos, loader info.
  - Lista de modpacks instalados con acción Eliminar.
  - Soporta `.mrpack` y `.zip` vía selector de archivo.
  - Botón "Modpacks" añadido a cada `ServerCard`.

### Tests
- 48 tests pasando (`cargo test`). Build frontend: ✓ (206 KB JS gzip: 63 KB).

## [0.12.0] — Fase 12: Mod Manager

### Added
- **Puerto `ModRepository`** (`cubed-application/src/ports/mod_repository.rs`):
  - `save`, `find_by_id`, `find_by_server`, `delete`.
- **Casos de uso** (`cubed-application/src/use_cases/`):
  - `AddMod` — valida extensión `.jar`, verifica servidor y persiste el mod.
  - `ListMods` — lista mods de un servidor ordenados por nombre.
  - `RemoveMod` — elimina del repositorio y devuelve la ruta del archivo.
- **`InMemoryModRepo`** (`cubed-infrastructure/src/mods/`) — repositorio en memoria para dev/tests.
- **`FileModManager`** (`cubed-infrastructure/src/mods/`):
  - `validate_jar` — verifica cabecera PK (`PK\x03\x04`) sin copiar el archivo.
  - `install_mod` — valida, copia el .jar a `mods/` y registra en el repositorio.
  - `list_mods` — lista ordenada por nombre desde el repositorio.
  - `remove_mod` — borra el .jar del disco (best-effort) y lo elimina del repositorio.
- **4 comandos Tauri nuevos** (`src-tauri/src/commands.rs`):
  - `list_mods(server_id)` → `Vec<ModDto>`.
  - `install_mod(server_id, source_path, mods_dir)` → `ModDto`.
  - `remove_mod(mod_id)`.
  - `validate_jar(path)` → `bool`.
- **Frontend — `Mods.tsx`** (`src/pages/Mods.tsx`):
  - Lista de mods instalados con nombre, ruta y botón Eliminar.
  - Botón "Instalar mod" con selector de archivo (`.jar`) y validación previa.
  - Mensajes de éxito/error diferenciados.
  - Navegación desde cada `ServerCard` con botón "Mods".

### Tests
- 45 tests pasando (`cargo test`). Build frontend: ✓ (200 KB JS gzip: 62 KB).

## [0.11.0] — Fase 11: Backup Manager

### Added
- **Puerto `BackupRepository`** (`cubed-application/src/ports/backup_repository.rs`):
  - `save`, `find_by_id`, `find_by_server`, `delete`.
- **Casos de uso** (`cubed-application/src/use_cases/`):
  - `CreateBackup` — verifica existencia del servidor y persiste el backup.
  - `ListBackups` — lista backups de un servidor ordenados por fecha desc.
  - `DeleteBackup` — elimina del repositorio y devuelve la ruta para borrar el archivo.
- **`InMemoryBackupRepo`** (`cubed-infrastructure/src/backup/`) — repositorio en memoria para dev/tests.
- **`FileBackupManager`** (`cubed-infrastructure/src/backup/`):
  - `backup_server` — crea archivo `.tar.gz` con `tar -czf` y persiste metadatos.
  - `restore_backup` — extrae el `.tar.gz` en el directorio indicado con `tar -xzf`.
  - `start_scheduler / stop_scheduler` — ejecuta backups automáticos cada N segundos en una tarea tokio.
- **`InMemoryServerRepo`** movida a `cubed-infrastructure/src/persistence/in_memory.rs` y reexportada públicamente.
- **4 comandos Tauri nuevos** (`src-tauri/src/commands.rs`):
  - `create_backup(server_id, server_name, server_dir)` → `BackupDto`.
  - `list_backups(server_id)` → `Vec<BackupDto>`.
  - `restore_backup(backup_id, restore_dir)`.
  - `delete_backup(backup_id, delete_file)`.
- **Frontend — `Backups.tsx`** (`src/pages/Backups.tsx`):
  - Lista de backups con fecha, tamaño y acciones Restaurar/Eliminar.
  - Botón "Crear backup" manual.
  - Navegación desde cada `ServerCard` con botón "Backups".
- **`types.ts`** — interfaz `BackupDto`.
- **`api.ts`** — métodos `listBackups`, `createBackup`, `restoreBackup`, `deleteBackup`.

### Tests
- 42 tests pasando (`cargo test`). Build frontend: ✓ (195 KB JS gzip: 61 KB).

## [0.10.0] — Fase 10: Resource Manager

### Added
- **Dependencia `sysinfo 0.32`** — monitoreo de CPU, RAM, disco y red del SO anfitrión.
- **Puerto `ResourceMonitor`** (`cubed-application/src/ports/resource_monitor.rs`):
  - `SystemStats` — CPU%, RAM usada/total, disco usada/total, red RX/TX acumulados.
  - `ServerStats` — CPU%, RAM (RSS), uptime en segundos de un proceso concreto.
  - Trait async `system_stats()` y `server_stats(id, pid)`.
- **`SysInfoResourceMonitor`** (`cubed-infrastructure/src/resources/`):
  - `system_stats()` — agrega CPU global, memoria, todos los discos y todas las interfaces de red.
  - `server_stats(pid)` — localiza el proceso por PID y devuelve su CPU%, RAM y uptime.
  - Instancia única con `Mutex<System>` (sysinfo no es `Send` sin sincronización).
  - 3 tests unitarios.
- **2 comandos Tauri nuevos** (`src-tauri/src/commands.rs`):
  - `get_system_stats` → `SystemStatsDto`.
  - `get_server_stats(id, pid)` → `Option<ServerStatsDto>`.
- **Dashboard actualizado** (`src/pages/Dashboard.tsx`):
  - Sección "Recursos del sistema" con 4 tarjetas: CPU, RAM, Disco, Red.
  - Barra de progreso con semáforo de color (verde/amarillo/rojo según uso).
  - Polling automático cada 3 s para mantener valores frescos.
- **`types.ts`** — nuevas interfaces `SystemStats` y `ServerStats`.
- **`api.ts`** — métodos `getSystemStats()` y `getServerStats(id, pid)`.

### Tests
- 40 tests pasando (`cargo test`). Build frontend: ✓ (190 KB JS gzip: 60 KB).

## [0.9.0] — Fase 9: Console Manager

### Added
- **Puerto `ConsoleManager`** (`cubed-application/src/ports/console.rs`):
  - `ConsoleLine { server_id, is_stdout, text }`.
  - `ConsoleCallback` — closure enviable entre hilos.
  - Trait `ConsoleManager` con `attach`, `send_command`, `tail`.
- **`MinecraftConsoleManager`** (`cubed-infrastructure/src/console/`):
  - Buffer circular de 500 líneas (`VecDeque<ConsoleLine>`) por servidor.
  - `spawn_readers` — dos tareas `tokio` que leen stdout/stderr línea a línea.
  - `register_stdin` — almacena `ChildStdin` para escritura posterior.
  - `attach` — instala callback y replaya el buffer histórico al nuevo suscriptor.
  - `tail` — acceso síncrono al buffer con `try_lock`.
- **3 comandos Tauri nuevos** (`src-tauri/src/commands.rs`):
  - `subscribe_console` — adjunta callback de Tauri events (`console-line:<id>`) y devuelve el histórico.
  - `send_console_command` — escribe a stdin del proceso.
  - `get_console_tail` — últimas N líneas sin suscripción.
- **Frontend — `Console.tsx`** (`src/pages/Console.tsx`):
  - Terminal oscura con salida stdout (zinc-200) y stderr (amarillo).
  - Suscripción en tiempo real vía `@tauri-apps/api/event`.
  - Historial de 1000 líneas en memoria; auto-scroll al final.
  - Input de comandos con envío por `Enter` o botón.
  - Botón "Volver" para regresar a la lista de servidores.
- **Botón "Consola"** añadido a `ServerCard` (siempre habilitado).
- **Navegación** actualizada en `App.tsx` para mostrar `Console` cuando se selecciona un servidor.

### Tests
- 37 tests pasando (`cargo test`). Build frontend: ✓ (186 KB JS gzip: 59 KB).

## [0.8.0] — Fase 8: Frontend Base

### Added
- **Comandos Tauri** (`src-tauri/src/commands.rs`):
  - `list_servers`, `create_server`, `start_server`, `stop_server`, `delete_server`.
  - `AppState` con `ServerRepository` + `FileSystemManager` inyectados.
- **`InMemoryServerRepo`** en `src-tauri` — repositorio en RAM para desarrollo sin PostgreSQL.
- **`api.ts`** — capa de acceso al backend: todas las llamadas `invoke` centralizadas.
- **Sidebar** con navegación entre Dashboard, Servidores y Configuración.
- **Página Dashboard** — tarjetas de estadísticas (total, activos, offline, caídos) + tabla de actividad reciente.
- **Página Servidores** — grid de `ServerCard` con acciones Iniciar/Detener/Eliminar; modal de creación con validación.
- **Página Configuración** — vista de los valores por defecto de `Settings`.
- **`StatusBadge`** — badge de estado con colores por estado del servidor.
- **`CreateServerModal`** — formulario completo (nombre, versión, software, puerto, Java, directorio).
- Token CSS `--destructive` y clase utilitaria `.input` en `index.css`.
- Tailwind: color `destructive` añadido.
- TypeScript: sin errores (`tsc --noEmit`). Build de producción: ✓ (181 KB JS gzip: 57 KB).

### Resultado
Primera versión visual utilizable.

## [0.7.0] — Fase 7: Server Manager

### Added
- **Puerto `ProcessManager`** (`cubed-application/ports/process_manager.rs`) — `spawn`, `stop`, `kill`, `is_alive`, `list_active`; devuelve `ProcessInfo` con `server_id` y `pid`.
- **`MinecraftProcessManager`** (`cubed-infrastructure/process/`) — gestión real de procesos con `tokio::process::Child`:
  - `spawn`: lanza `java -Xms{n/2}M -Xmx{n}M -jar server.jar --nogui` con stdin/stdout/stderr capturados.
  - `stop`: escribe `stop\n` en stdin del proceso (parada limpia de Minecraft).
  - `kill`: envía SIGKILL y elimina el proceso del mapa.
  - `is_alive`: usa `try_wait()` sin bloquear; limpia el mapa si el proceso terminó.
  - `list_active`: retorna snapshot de todos los PIDs activos (sync, no-blocking con `try_lock`).
- **`RunServer`** — caso de uso que orquesta transición de estado de dominio + `ProcessManager`: `start`, `stop`, `kill`, `restart`.
- **`MonitorServer`** — caso de uso que sincroniza el estado de dominio con la realidad del proceso (detecta crash inesperado y marca `Crashed`).
- 5 tests con procesos reales del SO (`true`/`sleep` en Unix, `cmd`/`ping` en Windows).

### Resultado
Primer servidor ejecutándose desde Cubed.

## [0.6.0] — Fase 6: Downloader Manager

### Added
- **Puerto `Downloader`** (`cubed-application/ports/downloader.rs`) — `download`, `build_url`; devuelve `DownloadedJar` con ruta y tamaño.
- **`HttpDownloader`** (`cubed-infrastructure/downloader/`) — descarga con streaming chunk-a-chunk via `reqwest` + `futures-util`.
- **`url_builder`** — resolución de URLs por software:
  - **Paper**: consulta `api.papermc.io/v2` → último build → URL de descarga directa.
  - **Purpur**: URL estática `api.purpurmc.org/v2/purpur/{mc}/latest/download`.
  - **Fabric**: consulta `meta.fabricmc.net` → último loader + último installer → URL server jar.
  - **Forge**: consulta `promotions_slim.json` de Forge Maven → versión recomendada o latest.
  - **NeoForge**: consulta Maven de NeoForge filtrando por `{minor}.{patch}.*`.
- **`DownloadServerJar`** — caso de uso con `execute` y `preview_url`.
- 5 tests unitarios (formato JAR name, URLs estáticas, distinción red/sin-red).

### Resultado
Cubed crea servidores sin descargas manuales.

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
