# Cubed — Referencia técnica completa

> Documento de referencia único. Describe **qué hace cada parte de Cubed y cómo**,
> incluyendo el flujo de datos extremo a extremo, el modelo de almacenamiento, los
> comandos expuestos, el ciclo de vida de los servidores y los puntos delicados
> (gotchas) que ya causaron bugs. El objetivo es evitar repetir problemas resueltos.
>
> Mantener este archivo al día cuando se añadan comandos, rutas o entidades.
> Última actualización: cobertura hasta la unificación de rutas + límites de RAM.

Índice:

1. [Qué es Cubed](#1-qué-es-cubed)
2. [Stack y arquitectura](#2-stack-y-arquitectura)
3. [Mapa de crates y carpetas](#3-mapa-de-crates-y-carpetas)
4. [Modelo de almacenamiento (fuente de verdad)](#4-modelo-de-almacenamiento-fuente-de-verdad)
5. [Rutas: dónde vive cada cosa](#5-rutas-dónde-vive-cada-cosa)
6. [Entidades del dominio](#6-entidades-del-dominio)
7. [Ciclo de vida de un servidor](#7-ciclo-de-vida-de-un-servidor)
8. [Comandos Tauri (API backend↔frontend)](#8-comandos-tauri-apibackendfrontend)
9. [Eventos en tiempo real](#9-eventos-en-tiempo-real)
10. [Arranque de servidores: Java, scripts y EULA](#10-arranque-de-servidores-java-scripts-y-eula)
11. [Descarga de servidores por loader](#11-descarga-de-servidores-por-loader)
12. [Mods y Modpacks](#12-mods-y-modpacks)
13. [Backups](#13-backups)
14. [Gestión de memoria (RAM)](#14-gestión-de-memoria-ram)
15. [Red / Tailscale](#15-red--tailscale)
16. [Inicialización de la app (composition root)](#16-inicialización-de-la-app-composition-root)
17. [Gotchas y causas raíz históricas](#17-gotchas-y-causas-raíz-históricas)

---

## 1. Qué es Cubed

Cubed es una **plataforma local de administración de servidores Minecraft Java**
para Linux (empaquetada como `.deb`). Permite crear, administrar, monitorear y
compartir múltiples servidores desde una GUI de escritorio, sin hosting externo.
Todo corre en la máquina del usuario.

Soporta los loaders: **Paper, Purpur, Fabric, Forge, NeoForge**.

---

## 2. Stack y arquitectura

| Capa         | Tecnología                                   |
| ------------ | -------------------------------------------- |
| Escritorio   | Tauri v2                                      |
| Frontend     | React + TypeScript + TailwindCSS + shadcn/ui |
| Backend      | Rust                                          |
| Persistencia | JSON (por defecto) **o** PostgreSQL + SQLx    |
| Red          | Tailscale                                     |

Arquitectura: **Clean Architecture + DDD + SOLID**. La regla de dependencia es
estricta — el código apunta solo hacia adentro. El dominio no depende de nadie;
la aplicación define puertos (traits) que la infraestructura implementa.

```
Presentation (src/)        → invoca comandos Tauri; NO toca FS/procesos/DB
Tauri bridge (src-tauri/)  → composition root; ensambla capas; #[tauri::command]
Application  (cubed-application) → casos de uso + puertos (traits)
Domain       (cubed-domain)      → entidades, value objects, reglas
Infrastructure (cubed-infrastructure) → JSON/Postgres, FS, procesos Java, Tailscale, APIs
```

Ver también [`ARCHITECTURE.md`](../ARCHITECTURE.md) para el detalle de principios.

---

## 3. Mapa de crates y carpetas

### `crates/cubed-domain` — Dominio (sin dependencias externas)
- `entities/`: `server.rs`, `backup.rs`, `mod_entry.rs`, `modpack.rs`, `settings.rs`
- `value_objects/`: `java_path.rs`, `server_name.rs`, `server_port.rs`, `server_version.rs`
- `error.rs`: `DomainError` / `DomainResult`

### `crates/cubed-application` — Casos de uso + puertos
- `ports/`: traits (interfaces). `ServerRepository`, `ProcessManager`, `JavaManager`,
  `Downloader`, `FileSystemManager`, `ConsoleManager`, `BackupRepository`,
  `ModRepository`, `ModpackRepository`, `NetworkManager`, `PortManager`, `ResourceMonitor`.
- `use_cases/`: lógica de orquestación. `create_server`, `download_server_jar`,
  `start_server`, `run_server`, `create_backup`, `import_modpack`, `add_mod`, etc.
- `events.rs`: `CubedEvent` (eventos de dominio).
- `error.rs`: `ApplicationError` / `ApplicationResult`.

### `crates/cubed-infrastructure` — Implementaciones concretas
- `persistence/`: `json_server_repository.rs`, `postgres_server_repository.rs`,
  `json_settings_store.rs`, `postgres_settings_store.rs`, `server_row.rs`, `db.rs`.
- `backup/`: `file_backup_manager.rs`, `json_backup_repo.rs`, `postgres_backup_repo.rs`.
- `mods/`: `file_mod_manager.rs`, `json_mod_repo.rs`, `postgres_mod_repo.rs`.
- `modpacks/`: `modpack_installer.rs`, `json_modpack_repo.rs`, `postgres_modpack_repo.rs`.
- `process/`: `minecraft_process_manager.rs` (spawn/kill/is_alive de procesos Java).
- `console/`: `minecraft_console_manager.rs` (stdin/stdout/stderr + buffer de líneas).
- `downloader/`: `http_downloader.rs`, `url_builder.rs` (resuelve URLs por loader).
- `java/`: `system_java_manager.rs` (detección y validación de JDK).
- `network/`: `tailscale_manager.rs`.
- `port/`: `tcp_port_manager.rs`.
- `resources/`: `sysinfo_monitor.rs` (CPU/RAM/disco/red).
- `fs/`: `local_file_system.rs`.

### `src-tauri/` — Composition root (bridge)
- `lib.rs`: ensambla todas las capas, elige JSON vs PostgreSQL, registra comandos.
- `commands.rs`: todos los `#[tauri::command]` + DTOs.
- `event_bus.rs`: reenvía `CubedEvent` al frontend vía Tauri emit.

### `src/` — Frontend React
- `api.ts`: wrapper tipado sobre `invoke()`. **Única puerta de entrada al backend.**
- `types.ts`: tipos compartidos (`Server`, `SettingsDto`, `BackupDto`, etc.).
- `pages/`: `Servers`, `Console`, `Mods`, `Modpacks`, `Backups`, `Settings`, `Dashboard`.
- `components/`: `CreateServerModal`, `ServerCard`, `Sidebar`, `TailscalePanel`, etc.
- `hooks/useAppEvents.ts`: se suscribe a los eventos Tauri.
- `tauriRuntime.ts`: `isTauriRuntime()` — guard para entornos sin Tauri.

---

## 4. Modelo de almacenamiento (fuente de verdad)

Cubed tiene **dos backends de persistencia** seleccionados automáticamente al
arrancar según la variable de entorno `DATABASE_URL`:

- **Sin `DATABASE_URL`** → JSON. Archivos en el directorio de datos de la app.
- **Con `DATABASE_URL`** → PostgreSQL vía SQLx.

La selección ocurre en `src-tauri/src/lib.rs` en el `setup()`. Cada repositorio
(`ServerRepository`, `BackupRepository`, `ModRepository`, `ModpackRepository`,
`SettingsStore`) tiene su variante JSON y Postgres detrás del mismo trait.

### Archivos JSON (modo por defecto)
Ubicados en `{app_data_dir}/`:
- `servers.json` — servidores registrados.
- `backups.json` — registro de backups.
- `mods.json` — mods instalados.
- `modpacks.json` — modpacks importados.
- `settings.json` — configuración global.

**Escritura atómica:** todos los repos JSON escriben a un archivo temporal y luego
hacen `rename` para evitar corrupción ante un cierre abrupto.

**Normalización al recargar:** al deserializar servidores, los estados activos
(`Running`/`Starting`/`Stopping`) se normalizan a `Offline`, porque ningún proceso
puede seguir vivo tras reiniciar la app. Esto lo refuerza
`reconcile_startup_server_states()` en el arranque.

> ⚠️ **El registro (JSON/DB) y los archivos en disco son cosas distintas.**
> El registro guarda metadatos (id, nombre, versión, puerto, estado). Los archivos
> reales del servidor (mods, mundo, jars) viven en `servers_dir/<nombre>`. Ver §5.

---

## 5. Rutas: dónde vive cada cosa

**Regla de oro: existe UNA sola ruta oficial por servidor**, calculada por el
backend y propagada al frontend. Nunca construir rutas a mano en el frontend.

### `work_dir` — la fuente de verdad única
```
work_dir = settings.servers_dir + "/" + server.name
```
- Calculado en `server_to_dto(server, servers_dir)` (`commands.rs`).
- Incluido en **cada `ServerDto`** devuelto por `list_servers`, `create_server`,
  `start_server`, `stop_server`, `restart_server`.
- El frontend lo recibe como `server.work_dir` y lo usa para Mods, Backups y Modpacks.

### Directorios de configuración (Settings)
- `servers_dir` — raíz donde se crean las carpetas de los servidores.
- `backups_dir` — raíz de backups.
- `downloads_dir` — descargas temporales.

Por defecto se derivan de `app_data_dir()` (ver `default_settings_for_data_dir`):
```
servers_dir   = {app_data_dir}/servers
backups_dir   = {app_data_dir}/backups
downloads_dir = {app_data_dir}/downloads
```
En Linux empaquetado, `app_data_dir` resuelve a algo como
`~/.local/share/dev.cubed.app/`.

### Estructura física de un servidor
```
{servers_dir}/{nombre}/
├── server.jar            (servidores jar: Paper/Purpur/Fabric)
├── run.sh                (servidores loader: Forge/NeoForge, lo genera el instalador)
├── cubed-start.sh        (wrapper que genera Cubed; exporta PATH de Java y llama run.sh)
├── user_jvm_args.txt     (args JVM para loaders; aquí se aplica la RAM en Forge/NeoForge)
├── libraries/            (Forge/NeoForge)
├── mods/                 (mods .jar)
├── eula.txt              (Cubed lo crea con eula=true antes de arrancar)
├── server.properties     (lo genera Minecraft en el primer arranque)
├── world/                (mundo)
└── logs/
```

> ⚠️ **Bug histórico (resuelto):** el frontend hardcodeaba `/tmp/cubed-dev/servers/...`
> para mods/modpacks/backups, mientras el servidor real corría bajo
> `~/.local/share/dev.cubed.app/servers/...`. Los archivos se importaban a una
> carpeta distinta y el servidor arrancaba sin mods. **Solución:** `work_dir` en el
> DTO como única fuente de verdad. Nunca volver a hardcodear rutas en el frontend.

---

## 6. Entidades del dominio

### `Server` (`entities/server.rs`)
Campos: `id` (Uuid), `name`, `version`, `software` (loader), `port`, `status`,
`java_path`. Métodos de transición: `start()`, `stop()`, `mark_running()`,
`mark_offline()`, `mark_crashed()`, `recover_as_offline()`. Las transiciones
inválidas devuelven error (p. ej. no se puede `start()` un servidor ya `Starting`).

### Value objects (validan en construcción)
- `JavaPath` — debe ser ruta absoluta no vacía (empieza con `/`).
- `ServerName` — nombre del servidor.
- `ServerPort` — puerto válido.
- `ServerVersion` — versión de Minecraft.

### `Settings` (`entities/settings.rs`)
`servers_dir`, `backups_dir`, `downloads_dir`, `default_java_path`,
`backup_interval_secs`, `memory_mb`.
- `memory_mb`: RAM de servidores jar. Default 4096. `#[serde(default)]` para
  retrocompatibilidad con `settings.json` antiguos.
- `validate_memory_mb(mb)`: exige rango **4096–12288 MB** (4–12 GB).
- Constantes `MEMORY_MB_MIN = 4096`, `MEMORY_MB_MAX = 12288`.

### `Backup`, `ModEntry`, `Modpack`
Metadatos de cada artefacto (id, server_id, ruta, tamaño, fecha, formato, etc.).

---

## 7. Ciclo de vida de un servidor

Estados (`ServerStatus`): `Offline`, `Starting`, `Running`, `Stopping`, `Crashed`.

```
Offline ──start()──► Starting ──(consola: "Done … For help")──► Running
   ▲                    │                                          │
   │                    │ (proceso muere sin llegar a Running)     │ stop()
   │                    ▼                                          ▼
   └──────────────── Crashed                                   Stopping ──(proceso muere)──► Offline
```

Flujo de `start_loaded_server` (`commands.rs`):
1. Lee `servers_dir` y `memory_mb` de Settings; valida `memory_mb`.
2. Calcula `work_dir`, `jar_path`, `script_path`.
3. **Resuelve Java de forma robusta** (ver §10).
4. Si hay `cubed-start.sh`, regenera su PATH al Java resuelto y aplica args de memoria.
5. **Escribe `eula.txt` = true** si no existe.
6. Transición de dominio → `Starting`, persiste.
7. Spawnea el proceso:
   - con script → `spawn_script_with_io(uuid, script_path, work_dir)`
   - con jar → `spawn_with_io(uuid, java_path, jar_path, work_dir, memory_mb)`
8. Registra stdin en el `ConsoleManager`; engancha callback que detecta la línea
   `Done (…)! For help, type "help"` → marca `Running` y emite `ServerStarted`.
9. Lanza un **watcher** que cada 2 s verifica `is_alive()`. Cuando el proceso muere:
   - si llegó a `Running` o estaba `Stopping` → `Offline` + `ServerStopped`.
   - si nunca llegó a `Running` → `Crashed` + `ServerCrashed`.

`stop_server`: transición `Stopping`, envía `stop\n` por stdin; si falla, `kill`.

`restart_server`: si `Running`, hace stop+espera (hasta 30 s) y luego arranca.

---

## 8. Comandos Tauri (API backend↔frontend)

Todos registrados en `src-tauri/src/lib.rs` (`invoke_handler`). El frontend los
llama **solo** vía `src/api.ts`.

| Comando | Qué hace |
| ------- | -------- |
| `health_check` | Ping de vida del backend. |
| `list_servers` | Lista servidores (con `work_dir`). |
| `create_server` | Valida puerto/Java, crea registro, descarga jar/instala loader. |
| `start_server` | Carga el servidor y delega en `start_loaded_server`. |
| `stop_server` | Envía `stop` o mata el proceso. |
| `restart_server` | Para (si corre) y vuelve a arrancar. |
| `delete_server` | Borra registro + carpeta. Rechaza si está activo. |
| `subscribe_console` | Devuelve el buffer de líneas de consola. |
| `send_console_command` | Envía un comando por stdin al servidor. |
| `get_console_tail` | Cola de líneas recientes. |
| `get_system_stats` | CPU/RAM/disco/red del host. |
| `get_server_stats` | Stats de un proceso (por pid). |
| `create_backup` | Crea backup (`serverDir` = `work_dir`). |
| `list_backups` / `restore_backup` / `delete_backup` | Gestión de backups. |
| `list_mods` / `install_mod` / `remove_mod` | Gestión de mods (`modsDir` = `work_dir/mods`). |
| `validate_jar` | Comprueba que un archivo es un .jar válido. |
| `install_modpack` / `list_modpacks` / `delete_modpack` | Modpacks (`installDir` = `work_dir`). |
| `suggest_free_port` | Sugiere un puerto libre. |
| `tailscale_is_installed` / `tailscale_status` / `tailscale_ip` | Estado de Tailscale. |
| `server_connect_address` | Dirección de conexión (Tailscale IP + puerto). |
| `detect_java` / `select_java_for_version` | Detección de JDK del sistema. |
| `get_settings` / `save_settings` | Lee/guarda configuración global. |

### DTOs clave (`commands.rs`)
- `ServerDto` — incluye `work_dir`.
- `SettingsDto` / `SaveSettingsCmd` — incluyen `memory_mb`.
- `BackupDto`, `ModDto`, `ModpackDto`, `InstallSummaryDto`, `ConsoleLineEvent`.

---

## 9. Eventos en tiempo real

`CubedEvent` (`cubed-application/src/events.rs`) → reenviados al frontend por
`event_bus.rs`. El frontend escucha en `hooks/useAppEvents.ts`.

| Evento | Canal Tauri |
| ------ | ----------- |
| `ServerStarted { server_id }` | `cubed://server.started` |
| `ServerStopped { server_id }` | `cubed://server.stopped` |
| `ServerCrashed { server_id }` | `cubed://server.crashed` |
| `BackupCreated { server_id, backup_id }` | `cubed://backup.created` |
| `ResourceUpdated { server_id? }` | métricas |
| `TailscaleUpdated { connected, ip }` | estado de red |

Además, cada línea de consola se emite como `console-line:{id}` y el progreso de
modpacks como `modpack-progress:{server_id}`.

---

## 10. Arranque de servidores: Java, scripts y EULA

### Resolución robusta de Java (`start_loaded_server`)
1. Intenta `inspect(stored_java_path)` + `validate_compatibility`.
2. Si falla (la ruta dejó de existir o es incompatible), **autodetecta** un JDK
   compatible vía `select_for_version(version)`.
3. Usa el Java resuelto para spawnear y para regenerar el PATH de `cubed-start.sh`.

`SystemJavaManager` (`java/system_java_manager.rs`):
- Busca en rutas candidatas (`/usr/bin/java`, `/usr/lib/jvm/...`, homebrew en macOS),
  `which java` y `JAVA_HOME`.
- `min_java_for_minecraft`: 1.20.5+ → Java 21; 1.18+ → 17; 1.17+ → 16; resto → 8.

### Scripts de arranque
- **Jar (Paper/Purpur/Fabric)**: `spawn_with_io` ejecuta
  `java -Xms{mem/2}M -Xmx{mem}M -jar server.jar --nogui`.
- **Loader (Forge/NeoForge)**: `cubed-start.sh` (generado por Cubed) exporta el
  PATH del Java resuelto y hace `exec sh ./run.sh --nogui`. `run.sh` lee los args
  de `user_jvm_args.txt` + `unix_args.txt` del loader.

### EULA
Cubed escribe `eula.txt` con `eula=true` antes de spawnear si no existe. Sin esto,
el servidor de Minecraft sale inmediatamente en el primer arranque y Cubed lo
marcaría como `Crashed`.

---

## 11. Descarga de servidores por loader

`downloader/url_builder.rs` resuelve la URL según el loader consultando su API:

| Loader | Fuente |
| ------ | ------ |
| Paper | `api.papermc.io` — último build de la versión. |
| Purpur | `api.purpurmc.org` — URL estática `latest/download`. |
| Fabric | `meta.fabricmc.net` — último loader + installer. |
| Forge | `maven.minecraftforge.net` (¡no `files.minecraftforge.net`!) + `promotions_slim.json` para elegir `-recommended`/`-latest`. |
| NeoForge | `maven.neoforged.net` — última versión por prefijo de MC. |

Para loaders con instalador (Forge/NeoForge), `prepare_installer_based_server`
ejecuta `java -jar installer --installServer`, lo que genera `run.sh` y `libraries/`.

> ⚠️ **Bug histórico (resuelto):** `files.minecraftforge.net` devuelve 404 a
> peticiones programáticas (protección anti-bot). Debe usarse `maven.minecraftforge.net`.

---

## 12. Mods y Modpacks

### Mods
- `install_mod(serverId, sourcePath, modsDir)` — `FileModManager` valida el .jar,
  lo copia a `modsDir` (= `work_dir/mods`) y lo registra.
- `validate_jar` — comprueba la firma del .jar.
- `remove_mod` — borra el archivo y el registro.
- Frontend: `pages/Mods.tsx` usa `${server.work_dir}/mods`.

### Modpacks
- `install_modpack(serverId, sourcePath, installDir)` — `ModpackInstaller`
  descomprime/descarga en `installDir` (= `work_dir`). Soporta `.mrpack` (Modrinth)
  y `.zip` (CurseForge). Emite progreso por `modpack-progress:{id}`.
- Frontend: `pages/Modpacks.tsx` usa `server.work_dir`.

---

## 13. Backups

- `FileBackupManager` (`backup/file_backup_manager.rs`) comprime `server_dir` en
  `backups_dir`. `backups_dir` es mutable (RwLock) y se actualiza al guardar Settings.
- Backup automático: `restart_auto_backup(interval_secs, servers_dir)` programado al
  arranque y al cambiar Settings. `backup_interval_secs = 0` lo desactiva.
- `create_backup(serverId, serverName, serverDir)` — `serverDir` = `work_dir`.
- `restore_backup(backupId, restoreDir)` — restaura en `${work_dir}_restored`.

---

## 14. Gestión de memoria (RAM)

Rango global **4 GB – 12 GB** validado en tres capas:

1. **Frontend** (`pages/Settings.tsx`): el input limita y muestra mensajes.
2. **Backend al guardar** (`save_settings`): `Settings::validate_memory_mb` antes de persistir.
3. **Backend al arrancar** (`start_loaded_server`): valida `memory_mb` antes de spawnear.

Aplicación:
- Servidores jar → `-Xms{mem/2}M -Xmx{mem}M` en `spawn_with_io`.
- Servidores loader → `ensure_loader_memory_args` ajusta `user_jvm_args.txt`.

`memory_mb` se persiste en Settings (JSON/Postgres) y viaja en `SettingsDto`.

---

## 15. Red / Tailscale

`tailscale_manager.rs` envuelve la CLI de Tailscale:
- `tailscale_is_installed`, `tailscale_status`, `tailscale_ip`.
- `server_connect_address` combina la IP de Tailscale con el puerto del servidor
  para dar una dirección compartible. Frontend: `components/TailscalePanel.tsx`.

---

## 16. Inicialización de la app (composition root)

`src-tauri/src/lib.rs` → `setup()`:
1. Resuelve `app_data_dir()`; crea el directorio.
2. Define rutas de `servers.json`, `backups.json`, `mods.json`, `modpacks.json`, `settings.json`.
3. Si `DATABASE_URL` está definida, conecta PostgreSQL; si no, usa JSON.
4. Carga Settings; si usa defaults built-in, los reemplaza por defaults derivados de `app_data_dir`.
5. Instancia repos (JSON o Postgres según el caso).
6. `check_integrity` (solo JSON) + `reconcile_startup_server_states` (normaliza estados a Offline).
7. Crea managers (backup, mod, modpack, proceso, consola, Java, red, recursos).
8. Programa el backup automático.
9. Crea el `EventBus` y registra `AppState`.

---

## 17. Gotchas y causas raíz históricas

Lista de problemas ya resueltos. **No reintroducir.**

1. **Servidores desaparecían al cerrar** → faltaba persistencia. Resuelto con
   `JsonServerRepository` (escritura atómica) y normalización de estados a Offline.
2. **Forge 404 al descargar** → `files.minecraftforge.net` bloquea bots. Usar
   `maven.minecraftforge.net`.
3. **Servidor arranca y vuelve a Offline/Crashed al instante** → faltaba `eula.txt`.
   Cubed ahora escribe `eula=true` antes de spawnear. (`--nogui`, no `nogui`.)
4. **"Iniciar" no hacía nada / se reseteaba** → el `java_path` guardado dejaba de
   resolver y el arranque abortaba en silencio. Resuelto con resolución robusta de
   Java + fallback a `select_for_version` + logs de diagnóstico.
5. **Mods/modpacks en carpeta distinta a la del servidor** → el frontend hardcodeaba
   `/tmp/cubed-dev/servers/...`. Resuelto con `work_dir` en el `ServerDto` como única
   fuente de verdad; el frontend nunca construye rutas.
6. **RAM sin límites** → ahora 4–12 GB validado en frontend, al guardar y al arrancar.

### Reglas para evitar repetir problemas
- **Nunca** construir rutas de servidor en el frontend. Usar siempre `server.work_dir`.
- **Nunca** depender de que el `java_path` guardado siga existiendo: resolver siempre.
- **Nunca** asumir que `eula.txt` existe: escribirlo antes de arrancar.
- **Nunca** usar `files.minecraftforge.net`: usar el maven.
- Toda nueva ruta o carpeta debe derivarse de Settings (`servers_dir`/`backups_dir`/
  `downloads_dir`) o de `work_dir`, jamás de `/tmp` ni rutas absolutas hardcodeadas.
- Al añadir un comando: registrarlo en `lib.rs`, exponerlo en `api.ts`, y
  documentarlo en la tabla de §8.
