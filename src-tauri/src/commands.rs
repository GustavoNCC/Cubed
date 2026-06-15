use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::event_bus::EventBus;
use cubed_application::ports::PortManager;
use cubed_application::ports::{
    BackupRepository, ConsoleLine, ConsoleManager, FileSystemManager, JavaManager, ModRepository,
    ModpackRepository, NetworkManager, ProcessManager, ResourceMonitor, ServerRepository,
    TailscaleStatus,
};
use cubed_application::use_cases::{CreateServer, CreateServerInput, DownloadServerJar};
use cubed_application::use_cases::{DeleteBackup, ListBackups, ListMods};
use cubed_application::CubedEvent;
use cubed_domain::entities::{ServerSoftware, ServerStatus, Settings};
use cubed_infrastructure::{
    FileBackupManager, FileModManager, HttpDownloader, JsonSettingsStore, MinecraftConsoleManager,
    MinecraftProcessManager, ModpackInstaller, PostgresSettingsStore, SysInfoResourceMonitor,
    SystemJavaManager, TailscaleNetworkManager, TcpPortManager,
};

// ── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ServerDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub software: String,
    pub port: u16,
    pub status: String,
}

#[derive(Deserialize)]
pub struct CreateServerCmd {
    pub name: String,
    pub version: String,
    pub software: String,
    pub port: u16,
    pub java_path: String,
    pub servers_dir: String,
}

/// Evento emitido al frontend por cada línea de consola.
#[derive(Serialize, Clone)]
pub struct ConsoleLineEvent {
    pub server_id: String,
    pub is_stdout: bool,
    pub text: String,
}

// ── App state ─────────────────────────────────────────────────────────────────

pub enum SettingsStore {
    Json(Arc<JsonSettingsStore>),
    Postgres(Arc<PostgresSettingsStore>),
}

impl SettingsStore {
    pub async fn save(&self, settings: &Settings) -> Result<(), String> {
        match self {
            Self::Json(store) => store.save(settings).map_err(|e| e.to_string()),
            Self::Postgres(store) => store.save(settings).await.map_err(|e| e.to_string()),
        }
    }
}

pub struct AppState {
    pub repo: Arc<dyn ServerRepository>,
    pub fs: Arc<dyn FileSystemManager>,
    pub console: Arc<MinecraftConsoleManager>,
    pub process_mgr: Arc<MinecraftProcessManager>,
    pub resources: Arc<SysInfoResourceMonitor>,
    pub backup_repo: Arc<dyn BackupRepository>,
    pub backup_mgr: Arc<FileBackupManager>,
    pub mod_repo: Arc<dyn ModRepository>,
    pub mod_mgr: Arc<FileModManager>,
    pub modpack_repo: Arc<dyn ModpackRepository>,
    pub modpack_inst: Arc<ModpackInstaller>,
    pub network: Arc<TailscaleNetworkManager>,
    pub java_mgr: Arc<SystemJavaManager>,
    pub downloader: Arc<HttpDownloader>,
    pub event_bus: Arc<EventBus>,
    pub settings_store: Arc<SettingsStore>,
    /// Configuración global mutable y persistida.
    pub settings: Arc<RwLock<Settings>>,
}

// ── Server commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerDto>, String> {
    state
        .repo
        .find_all()
        .await
        .map_err(|e| e.to_string())
        .map(|v| v.iter().map(server_to_dto).collect())
}

#[tauri::command]
pub async fn create_server(
    cmd: CreateServerCmd,
    state: State<'_, AppState>,
) -> Result<ServerDto, String> {
    if cmd.name.trim().is_empty() {
        return Err("El nombre del servidor no puede estar vacío".into());
    }
    if cmd.name.len() > 64 {
        return Err("El nombre del servidor no puede superar 64 caracteres".into());
    }
    if cmd.port < 1024 {
        return Err("El puerto debe ser >= 1024".into());
    }
    if !TcpPortManager::new()
        .is_free(cmd.port)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err(format!(
            "El puerto {} ya está ocupado por el sistema",
            cmd.port
        ));
    }
    let software = parse_software(&cmd.software)?;
    let java = state
        .java_mgr
        .inspect(&cmd.java_path)
        .await
        .map_err(|e| e.to_string())?;
    state
        .java_mgr
        .validate_compatibility(&java, &cmd.version)
        .map_err(|e| e.to_string())?;
    let servers_dir = {
        let settings = state.settings.read().await;
        if cmd.servers_dir.trim().is_empty() {
            settings.servers_dir.clone()
        } else {
            cmd.servers_dir.trim().to_string()
        }
    };
    let uc = CreateServer::new(state.repo.clone(), state.fs.clone());
    let server = uc
        .execute(CreateServerInput {
            name: cmd.name.trim().to_string(),
            version: cmd.version,
            software: software.clone(),
            port: cmd.port,
            java_path: cmd.java_path,
            servers_dir: servers_dir.clone(),
        })
        .await
        .map_err(|e| e.to_string())?;

    let work_dir = format!("{}/{}", servers_dir, server.name());
    let download = DownloadServerJar::new(state.downloader.clone())
        .execute(&software, server.version().as_str(), &work_dir)
        .await;

    match download {
        Ok(downloaded) => {
            if let Err(e) = prepare_downloaded_server(
                &software,
                server.java_path().as_str(),
                &work_dir,
                &downloaded.path,
            )
            .await
            {
                let _ = state.repo.delete(server.id()).await;
                let _ = tokio::fs::remove_dir_all(&work_dir).await;
                return Err(format!("No se pudo preparar el runtime descargado: {}", e));
            }
        }
        Err(e) => {
            let _ = state.repo.delete(server.id()).await;
            let _ = tokio::fs::remove_dir_all(&work_dir).await;
            return Err(format!("No se pudo descargar el servidor: {}", e));
        }
    }

    info!(server_id = %server.id(), name = %server.name(), "server created");
    Ok(server_to_dto(&server))
}

#[tauri::command]
pub async fn start_server(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ServerDto, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let server = state
        .repo
        .find_by_id(uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;

    start_loaded_server(id, app, &state, server).await
}

#[tauri::command]
pub async fn restart_server(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ServerDto, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut server = state
        .repo
        .find_by_id(uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;

    match server.status() {
        ServerStatus::Running => {
            server.stop().map_err(|e| e.to_string())?;
            state.repo.save(&server).await.map_err(|e| e.to_string())?;
            if let Err(e) = state.console.send_command(uuid, "stop").await {
                warn!(server_id = %uuid, err = %e, "restart fallback: killing process");
                state.process_mgr.kill(uuid).await.ok();
            }

            for _ in 0..30 {
                match state.process_mgr.is_alive(uuid).await {
                    Ok(false) => break,
                    Ok(true) => tokio::time::sleep(tokio::time::Duration::from_secs(1)).await,
                    Err(_) => break,
                }
            }
            if state.process_mgr.is_alive(uuid).await.unwrap_or(false) {
                state.process_mgr.kill(uuid).await.ok();
            }
            state.console.detach(uuid).await;

            if let Ok(Some(mut current)) = state.repo.find_by_id(uuid).await {
                current.mark_offline().ok();
                state.repo.save(&current).await.map_err(|e| e.to_string())?;
                server = current;
            }
        }
        ServerStatus::Offline | ServerStatus::Crashed => {}
        ServerStatus::Starting | ServerStatus::Stopping => {
            return Err(
                "No se puede reiniciar mientras el servidor está cambiando de estado".into(),
            );
        }
    }

    start_loaded_server(id, app, &state, server).await
}

async fn start_loaded_server(
    id: String,
    app: AppHandle,
    state: &AppState,
    mut server: cubed_domain::entities::Server,
) -> Result<ServerDto, String> {
    let uuid = server.id();
    let servers_dir = state.settings.read().await.servers_dir.clone();
    let work_dir = format!("{}/{}", servers_dir, server.name());
    let jar_path = format!("{}/server.jar", work_dir);
    let script_path = format!("{}/cubed-start.sh", work_dir);
    let java = state
        .java_mgr
        .inspect(server.java_path().as_str())
        .await
        .map_err(|e| e.to_string())?;
    state
        .java_mgr
        .validate_compatibility(&java, server.version().as_str())
        .map_err(|e| e.to_string())?;

    // Reject early if no launch target is available
    let has_script = Path::new(&script_path).exists();
    let has_jar = Path::new(&jar_path).exists();
    if !has_script && !has_jar {
        return Err(format!(
            "No se encontró un runtime arrancable en '{}'. Descarga el servidor primero.",
            work_dir
        ));
    }

    // Aceptar EULA automáticamente — todos los servidores Minecraft la requieren y salen
    // inmediatamente en el primer arranque si eula.txt no existe o dice eula=false.
    let eula_path = Path::new(&work_dir).join("eula.txt");
    if !eula_path.exists() {
        if let Err(e) = tokio::fs::write(&eula_path, "eula=true\n").await {
            warn!(server_id = %uuid, error = %e, "no se pudo escribir eula.txt");
        } else {
            info!(server_id = %uuid, "eula.txt creado automáticamente");
        }
    }

    info!(
        server_id = %uuid,
        java_path = %server.java_path(),
        work_dir = %work_dir,
        has_script,
        has_jar,
        "iniciando servidor"
    );

    // Domain transition → Starting
    server.start().map_err(|e| e.to_string())?;
    state.repo.save(&server).await.map_err(|e| e.to_string())?;

    // Spawn actual Java process
    let spawned = if has_script {
        state
            .process_mgr
            .spawn_script_with_io(uuid, &script_path, &work_dir)
            .await
    } else {
        state
            .process_mgr
            .spawn_with_io(
                uuid,
                server.java_path().as_str(),
                &jar_path,
                &work_dir,
                2048,
            )
            .await
    };
    let (pid, stdin, stdout, stderr) = match spawned {
        Ok(spawned) => spawned,
        Err(e) => {
            server.mark_crashed();
            let _ = state.repo.save(&server).await;
            state
                .event_bus
                .emit(CubedEvent::ServerCrashed { server_id: uuid });
            return Err(e.to_string());
        }
    };

    info!(server_id = %uuid, pid, "java process spawned");

    // Register stdin so console commands can be sent
    state.console.register_stdin(uuid, stdin).await;

    // Build a callback that:
    //   1. Forwards lines to the Tauri frontend
    //   2. Detects the "Done" line → marks Running
    //   3. Detects a crash / process end → marks Crashed
    let event_name = format!("console-line:{}", id);
    let app_cb = app.clone();
    let repo_cb = state.repo.clone();
    let eb_cb = state.event_bus.clone();
    let running = Arc::new(AtomicBool::new(false));
    let running2 = running.clone();

    state
        .console
        .attach(
            uuid,
            Box::new(move |line: ConsoleLine| {
                // Forward to frontend
                let evt = ConsoleLineEvent {
                    server_id: line.server_id.to_string(),
                    is_stdout: line.is_stdout,
                    text: line.text.clone(),
                };
                app_cb.emit(&event_name, evt).ok();

                // Detect "Done (X.XXXs)! For help, type "help"" from Minecraft
                if line.is_stdout
                    && !running2.load(Ordering::Relaxed)
                    && line.text.contains("Done")
                    && line.text.contains("For help")
                {
                    running2.store(true, Ordering::Relaxed);
                    let repo = repo_cb.clone();
                    let eb = eb_cb.clone();
                    tokio::spawn(async move {
                        if let Ok(Some(mut srv)) = repo.find_by_id(uuid).await {
                            if srv.mark_running().is_ok() {
                                let _ = repo.save(&srv).await;
                                eb.emit(CubedEvent::ServerStarted { server_id: uuid });
                                info!(server_id = %uuid, "server is now Running");
                            }
                        }
                    });
                }
            }),
        )
        .await
        .map_err(|e| e.to_string())?;

    // Start reading stdout/stderr (lines go through the callback above)
    state.console.spawn_readers(uuid, stdout, stderr).await;

    // Background watcher: when the process dies, mark offline or crashed
    let repo_w = state.repo.clone();
    let proc_w = state.process_mgr.clone();
    let eb_w = state.event_bus.clone();
    let console_w = state.console.clone();
    let running_w = running.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            match proc_w.is_alive(uuid).await {
                Ok(true) => continue,
                _ => break,
            }
        }
        // Process has exited
        console_w.detach(uuid).await;
        if let Ok(Some(mut srv)) = repo_w.find_by_id(uuid).await {
            let was_stopping = *srv.status() == cubed_domain::entities::ServerStatus::Stopping;
            let was_running = running_w.load(Ordering::Relaxed);
            if was_stopping || was_running {
                // Clean shutdown path: Stopping → Offline
                let _ = srv.mark_offline();
                let _ = repo_w.save(&srv).await;
                eb_w.emit(CubedEvent::ServerStopped { server_id: uuid });
                info!(server_id = %uuid, was_running, was_stopping, "proceso terminó → Offline");
            } else {
                // Never reached Running → Crashed
                srv.mark_crashed();
                let _ = repo_w.save(&srv).await;
                eb_w.emit(CubedEvent::ServerCrashed { server_id: uuid });
                warn!(
                    server_id = %uuid,
                    was_running,
                    was_stopping,
                    "proceso terminó sin llegar a Running → Crashed (revisar consola para causa)"
                );
            }
        }
    });

    Ok(server_to_dto(&server))
}

#[tauri::command]
pub async fn stop_server(id: String, state: State<'_, AppState>) -> Result<ServerDto, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut server = state
        .repo
        .find_by_id(uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;

    // Transition domain state (Running → Stopping)
    server.stop().map_err(|e| e.to_string())?;
    state.repo.save(&server).await.map_err(|e| e.to_string())?;

    // Send "stop" command to the server process via stdin
    if let Err(e) = state.console.send_command(uuid, "stop").await {
        warn!(server_id = %uuid, err = %e, "could not send stop via stdin, killing process");
        state.process_mgr.kill(uuid).await.ok();
        server.mark_offline().map_err(|e| e.to_string())?;
        state.repo.save(&server).await.map_err(|e| e.to_string())?;
    }

    state
        .event_bus
        .emit(CubedEvent::ServerStopped { server_id: uuid });
    info!(server_id = %uuid, "server stop requested");
    Ok(server_to_dto(&server))
}

#[tauri::command]
pub async fn delete_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let server = state
        .repo
        .find_by_id(uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;
    if !matches!(
        server.status(),
        ServerStatus::Offline | ServerStatus::Crashed
    ) {
        warn!(server_id = %uuid, "delete rejected: server is running");
        return Err("Detén el servidor antes de eliminarlo".into());
    }
    let servers_dir = state.settings.read().await.servers_dir.clone();
    state
        .fs
        .delete_server_dir(&servers_dir, server.name().as_str())
        .await
        .map_err(|e| e.to_string())?;
    state.repo.delete(uuid).await.map_err(|e| e.to_string())?;
    info!(server_id = %uuid, "server deleted");
    Ok(())
}

// ── Console commands ──────────────────────────────────────────────────────────

/// Suscribe el frontend a la consola de un servidor.
/// Las líneas nuevas se emitirán como eventos Tauri "console-line:<server_id>".
/// Las líneas históricas del buffer también se replayan inmediatamente.
#[tauri::command]
pub async fn subscribe_console(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<ConsoleLineEvent>, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let event_name = format!("console-line:{}", id);
    let app_clone = app.clone();
    let event_clone = event_name.clone();

    state
        .console
        .attach(
            uuid,
            Box::new(move |line: ConsoleLine| {
                let evt = ConsoleLineEvent {
                    server_id: line.server_id.to_string(),
                    is_stdout: line.is_stdout,
                    text: line.text,
                };
                app_clone.emit(&event_clone, evt).ok();
            }),
        )
        .await
        .map_err(|e| e.to_string())?;

    // Return current buffer snapshot for immediate display
    Ok(state
        .console
        .tail(uuid, 500)
        .into_iter()
        .map(line_to_event)
        .collect())
}

/// Envía un comando a stdin del servidor.
#[tauri::command]
pub async fn send_console_command(
    id: String,
    command: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state
        .console
        .send_command(uuid, &command)
        .await
        .map_err(|e| e.to_string())
}

/// Devuelve las últimas N líneas del buffer sin suscribirse.
#[tauri::command]
pub async fn get_console_tail(
    id: String,
    n: usize,
    state: State<'_, AppState>,
) -> Result<Vec<ConsoleLineEvent>, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    Ok(state
        .console
        .tail(uuid, n)
        .into_iter()
        .map(line_to_event)
        .collect())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn server_to_dto(s: &cubed_domain::entities::Server) -> ServerDto {
    ServerDto {
        id: s.id().to_string(),
        name: s.name().to_string(),
        version: s.version().to_string(),
        software: s.software().to_string(),
        port: s.port().value(),
        status: match s.status() {
            ServerStatus::Offline => "offline",
            ServerStatus::Starting => "starting",
            ServerStatus::Running => "running",
            ServerStatus::Stopping => "stopping",
            ServerStatus::Crashed => "crashed",
        }
        .to_string(),
    }
}

fn line_to_event(l: ConsoleLine) -> ConsoleLineEvent {
    ConsoleLineEvent {
        server_id: l.server_id.to_string(),
        is_stdout: l.is_stdout,
        text: l.text,
    }
}

fn parse_software(s: &str) -> Result<ServerSoftware, String> {
    match s {
        "Paper" => Ok(ServerSoftware::Paper),
        "Purpur" => Ok(ServerSoftware::Purpur),
        "Fabric" => Ok(ServerSoftware::Fabric),
        "Forge" => Ok(ServerSoftware::Forge),
        "NeoForge" => Ok(ServerSoftware::NeoForge),
        other => Err(format!("Software desconocido: {}", other)),
    }
}

async fn prepare_downloaded_server(
    software: &ServerSoftware,
    java_path: &str,
    work_dir: &str,
    downloaded_path: &str,
) -> Result<(), String> {
    match software {
        ServerSoftware::Forge | ServerSoftware::NeoForge => {
            prepare_installer_based_server(software, java_path, work_dir, downloaded_path).await
        }
        ServerSoftware::Paper | ServerSoftware::Purpur | ServerSoftware::Fabric => {
            let jar_path = Path::new(work_dir).join("server.jar");
            if Path::new(downloaded_path) != jar_path {
                tokio::fs::rename(downloaded_path, &jar_path)
                    .await
                    .map_err(|e| {
                        format!(
                            "No se pudo preparar server.jar desde '{}': {}",
                            downloaded_path, e
                        )
                    })?;
            }
            Ok(())
        }
    }
}

async fn prepare_installer_based_server(
    software: &ServerSoftware,
    java_path: &str,
    work_dir: &str,
    downloaded_path: &str,
) -> Result<(), String> {
    let installer_name = format!("{}-installer.jar", software.to_string().to_lowercase());
    let installer_path = Path::new(work_dir).join(installer_name);
    if Path::new(downloaded_path) != installer_path {
        tokio::fs::rename(downloaded_path, &installer_path)
            .await
            .map_err(|e| format!("No se pudo preparar el instalador: {}", e))?;
    }

    let status = Command::new(java_path)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installServer")
        .current_dir(work_dir)
        .status()
        .await
        .map_err(|e| format!("No se pudo ejecutar el instalador: {}", e))?;
    if !status.success() {
        return Err(format!(
            "El instalador de {} terminó con código {:?}",
            software,
            status.code()
        ));
    }

    let run_sh = Path::new(work_dir).join("run.sh");
    if run_sh.exists() {
        write_loader_start_script(work_dir, java_path).await?;
        return Ok(());
    }

    if let Some(server_jar) = find_loader_server_jar(work_dir, software).await? {
        let target = Path::new(work_dir).join("server.jar");
        if server_jar != target {
            tokio::fs::copy(&server_jar, &target)
                .await
                .map_err(|e| format!("No se pudo preparar server.jar: {}", e))?;
        }
        return Ok(());
    }

    Err(format!(
        "El instalador de {} no generó run.sh ni un JAR de servidor reconocible",
        software
    ))
}

async fn write_loader_start_script(work_dir: &str, java_path: &str) -> Result<(), String> {
    let script_path = Path::new(work_dir).join("cubed-start.sh");
    let java_dir = Path::new(java_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let content = format!(
        "#!/bin/sh\ncd \"$(dirname \"$0\")\"\nexport PATH='{}':\"$PATH\"\nexec sh ./run.sh --nogui\n",
        shell_single_quote(java_dir)
    );
    tokio::fs::write(&script_path, content)
        .await
        .map_err(|e| format!("No se pudo escribir cubed-start.sh: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = tokio::fs::metadata(&script_path)
            .await
            .map_err(|e| format!("No se pudo leer permisos de cubed-start.sh: {}", e))?
            .permissions();
        permissions.set_mode(0o755);
        tokio::fs::set_permissions(&script_path, permissions)
            .await
            .map_err(|e| format!("No se pudo hacer ejecutable cubed-start.sh: {}", e))?;
    }

    Ok(())
}

async fn find_loader_server_jar(
    work_dir: &str,
    software: &ServerSoftware,
) -> Result<Option<PathBuf>, String> {
    let base = PathBuf::from(work_dir);
    let prefix = software.to_string().to_lowercase();
    tokio::task::spawn_blocking(move || {
        let entries = std::fs::read_dir(&base).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let lower = name.to_lowercase();
            if lower.starts_with(&prefix)
                && lower.ends_with(".jar")
                && !lower.contains("installer")
                && !lower.contains("client")
            {
                return Ok(Some(path));
            }
        }
        Ok(None)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn shell_single_quote(value: &str) -> String {
    value.replace('\'', "'\\''")
}

// ── Resource commands ─────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct SystemStatsDto {
    pub cpu_percent: f32,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
}

#[derive(Serialize, Clone)]
pub struct ServerStatsDto {
    pub server_id: String,
    pub cpu_percent: f32,
    pub ram_bytes: u64,
    pub uptime_secs: u64,
}

#[tauri::command]
pub async fn get_system_stats(state: State<'_, AppState>) -> Result<SystemStatsDto, String> {
    let s = state
        .resources
        .system_stats()
        .await
        .map_err(|e| e.to_string())?;
    Ok(SystemStatsDto {
        cpu_percent: s.cpu_percent,
        ram_used_bytes: s.ram_used_bytes,
        ram_total_bytes: s.ram_total_bytes,
        disk_used_bytes: s.disk_used_bytes,
        disk_total_bytes: s.disk_total_bytes,
        net_rx_bytes: s.net_rx_bytes,
        net_tx_bytes: s.net_tx_bytes,
    })
}

#[tauri::command]
pub async fn get_server_stats(
    id: String,
    pid: u32,
    state: State<'_, AppState>,
) -> Result<Option<ServerStatsDto>, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let opt = state
        .resources
        .server_stats(uuid, pid)
        .await
        .map_err(|e| e.to_string())?;
    Ok(opt.map(|s| ServerStatsDto {
        server_id: s.server_id.to_string(),
        cpu_percent: s.cpu_percent,
        ram_bytes: s.ram_bytes,
        uptime_secs: s.uptime_secs,
    }))
}

// ── Backup commands ───────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct BackupDto {
    pub id: String,
    pub server_id: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

fn backup_to_dto(b: &cubed_domain::entities::Backup) -> BackupDto {
    BackupDto {
        id: b.id().to_string(),
        server_id: b.server_id().to_string(),
        path: b.path().to_string(),
        size_bytes: b.size_bytes(),
        created_at: b.created_at().to_rfc3339(),
    }
}

/// Crea un backup manual del servidor.
/// `server_dir` debe ser la ruta absoluta al directorio del servidor.
#[tauri::command]
pub async fn create_backup(
    server_id: String,
    server_name: String,
    server_dir: String,
    state: State<'_, AppState>,
) -> Result<BackupDto, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;
    let backup = state
        .backup_mgr
        .backup_server(uuid, &server_name, &server_dir)
        .await
        .map_err(|e| e.to_string())?;
    state.event_bus.emit(CubedEvent::BackupCreated {
        server_id: uuid,
        backup_id: backup.id(),
    });
    debug!(server_id = %uuid, backup_id = %backup.id(), "backup created");
    Ok(backup_to_dto(&backup))
}

/// Lista todos los backups de un servidor, ordenados por fecha desc.
#[tauri::command]
pub async fn list_backups(
    server_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<BackupDto>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;
    let uc = ListBackups::new(state.backup_repo.clone());
    let list = uc.execute(uuid).await.map_err(|e| e.to_string())?;
    Ok(list.iter().map(backup_to_dto).collect())
}

/// Restaura un backup en el directorio indicado.
#[tauri::command]
pub async fn restore_backup(
    backup_id: String,
    restore_dir: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&backup_id).map_err(|e| e.to_string())?;
    state
        .backup_mgr
        .restore_backup(uuid, &restore_dir)
        .await
        .map_err(|e| e.to_string())
}

/// Elimina un backup del registro y opcionalmente del disco.
#[tauri::command]
pub async fn delete_backup(
    backup_id: String,
    delete_file: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&backup_id).map_err(|e| e.to_string())?;
    let uc = DeleteBackup::new(state.backup_repo.clone());
    let path = uc.execute(uuid).await.map_err(|e| e.to_string())?;
    if delete_file {
        let _ = tokio::fs::remove_file(&path).await; // best-effort
    }
    Ok(())
}

// ── Mod commands ──────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ModDto {
    pub id: String,
    pub server_id: String,
    pub file_name: String,
    pub path: String,
}

fn mod_to_dto(m: &cubed_domain::entities::ModEntry) -> ModDto {
    ModDto {
        id: m.id().to_string(),
        server_id: m.server_id().to_string(),
        file_name: m.file_name().to_string(),
        path: m.path().to_string(),
    }
}

/// Lista los mods de un servidor (ordenados por nombre).
#[tauri::command]
pub async fn list_mods(
    server_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ModDto>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;
    let uc = ListMods::new(state.mod_repo.clone());
    let list = uc.execute(uuid).await.map_err(|e| e.to_string())?;
    Ok(list.iter().map(mod_to_dto).collect())
}

/// Instala un mod: valida el .jar, lo copia a mods/ y lo registra.
#[tauri::command]
pub async fn install_mod(
    server_id: String,
    source_path: String,
    mods_dir: String,
    state: State<'_, AppState>,
) -> Result<ModDto, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;
    let entry = state
        .mod_mgr
        .install_mod(uuid, &source_path, &mods_dir)
        .await
        .map_err(|e| e.to_string())?;
    Ok(mod_to_dto(&entry))
}

/// Elimina un mod: borra el .jar y lo quita del registro.
#[tauri::command]
pub async fn remove_mod(mod_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let uuid = Uuid::parse_str(&mod_id).map_err(|e| e.to_string())?;
    state
        .mod_mgr
        .remove_mod(uuid)
        .await
        .map_err(|e| e.to_string())
}

/// Valida si un archivo es un .jar válido (cabecera PK).
#[tauri::command]
pub async fn validate_jar(path: String) -> Result<bool, String> {
    match cubed_infrastructure::FileModManager::validate_jar(&path).await {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}

// ── Modpack commands ──────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct ModpackDto {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub format: String,
    pub source_path: String,
}

#[derive(Serialize, Clone)]
pub struct InstallSummaryDto {
    pub modpack: ModpackDto,
    pub total_files: usize,
    pub downloaded: usize,
    pub skipped: usize,
    pub loader_info: Option<String>,
}

fn modpack_to_dto(m: &cubed_domain::entities::Modpack) -> ModpackDto {
    ModpackDto {
        id: m.id().to_string(),
        server_id: m.server_id().to_string(),
        name: m.name().to_string(),
        format: m.format().to_string(),
        source_path: m.source_path().to_string(),
    }
}

/// Instala un modpack (.mrpack o .zip) en el directorio del servidor.
/// Las líneas de progreso se emiten como eventos Tauri "modpack-progress:<server_id>".
#[tauri::command]
pub async fn install_modpack(
    server_id: String,
    source_path: String,
    install_dir: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<InstallSummaryDto, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;
    let event_name = format!("modpack-progress:{}", server_id);
    let app_clone = app.clone();
    let event_clone = event_name.clone();

    let (modpack, summary) = state
        .modpack_inst
        .install(uuid, &source_path, &install_dir, move |progress| {
            let _ = app_clone.emit(
                &event_clone,
                serde_json::json!({
                    "total": progress.total,
                    "done":  progress.done,
                    "file":  progress.current_file,
                }),
            );
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(InstallSummaryDto {
        modpack: modpack_to_dto(&modpack),
        total_files: summary.total_files,
        downloaded: summary.downloaded,
        skipped: summary.skipped,
        loader_info: summary.loader_info,
    })
}

/// Lista los modpacks importados para un servidor.
#[tauri::command]
pub async fn list_modpacks(
    server_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ModpackDto>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;
    let mut list = state
        .modpack_repo
        .find_by_server(uuid)
        .await
        .map_err(|e| e.to_string())?;
    list.sort_by(|a, b| a.name().cmp(b.name()));
    Ok(list.iter().map(modpack_to_dto).collect())
}

/// Elimina el registro de un modpack (no borra los mods ya instalados).
#[tauri::command]
pub async fn delete_modpack(modpack_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let uuid = Uuid::parse_str(&modpack_id).map_err(|e| e.to_string())?;
    state
        .modpack_repo
        .delete(uuid)
        .await
        .map_err(|e| e.to_string())
}

// ── Network / Tailscale commands ──────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct TailscaleStatusDto {
    /// "not_installed" | "disconnected" | "connected"
    pub state: String,
    pub ip: Option<String>,
    pub hostname: Option<String>,
}

fn tailscale_status_to_dto(s: TailscaleStatus) -> TailscaleStatusDto {
    match s {
        TailscaleStatus::NotInstalled => TailscaleStatusDto {
            state: "not_installed".into(),
            ip: None,
            hostname: None,
        },
        TailscaleStatus::Disconnected => TailscaleStatusDto {
            state: "disconnected".into(),
            ip: None,
            hostname: None,
        },
        TailscaleStatus::Connected { ip, hostname } => TailscaleStatusDto {
            state: "connected".into(),
            ip: Some(ip),
            hostname: Some(hostname),
        },
    }
}

/// Detecta si Tailscale está instalado en el sistema.
#[tauri::command]
pub async fn tailscale_is_installed(state: State<'_, AppState>) -> Result<bool, String> {
    state
        .network
        .is_installed()
        .await
        .map_err(|e| e.to_string())
}

/// Devuelve el estado actual de Tailscale (not_installed | disconnected | connected).
#[tauri::command]
pub async fn tailscale_status(state: State<'_, AppState>) -> Result<TailscaleStatusDto, String> {
    let s = state.network.status().await.map_err(|e| e.to_string())?;
    Ok(tailscale_status_to_dto(s))
}

/// Devuelve la IP de Tailscale si está conectado.
#[tauri::command]
pub async fn tailscale_ip(state: State<'_, AppState>) -> Result<Option<String>, String> {
    state
        .network
        .tailscale_ip()
        .await
        .map_err(|e| e.to_string())
}

/// Devuelve el primer puerto libre >= 25565 que no esté en uso por el SO
/// ni por ningún servidor registrado en Cubed.
#[tauri::command]
pub async fn suggest_free_port(state: State<'_, AppState>) -> Result<u16, String> {
    let mgr = TcpPortManager::new();
    let used: std::collections::HashSet<u16> = state
        .repo
        .find_all()
        .await
        .map_err(|e| e.to_string())?
        .iter()
        .map(|s| s.port().value())
        .collect();

    for candidate in 25565u16..=65534 {
        if !used.contains(&candidate) && mgr.is_free(candidate).await.unwrap_or(false) {
            return Ok(candidate);
        }
    }
    Err("No se encontró un puerto libre".into())
}

/// Construye la dirección de conexión al servidor: `<tailscale_ip>:<port>`.
/// Útil para copiar al portapapeles.
#[tauri::command]
pub async fn server_connect_address(
    server_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;
    let server = state
        .repo
        .find_by_id(uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", server_id))?;
    let ip = state
        .network
        .tailscale_ip()
        .await
        .map_err(|e| e.to_string())?;
    Ok(ip.map(|addr| format!("{}:{}", addr, server.port().value())))
}

// ── Java detection commands ───────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct JavaInstallationDto {
    pub path: String,
    pub major_version: u32,
    pub version_string: String,
}

/// Devuelve todas las instalaciones de Java encontradas en el sistema.
#[tauri::command]
pub async fn detect_java(state: State<'_, AppState>) -> Result<Vec<JavaInstallationDto>, String> {
    let installations = state
        .java_mgr
        .detect_installations()
        .await
        .map_err(|e| e.to_string())?;
    Ok(installations
        .into_iter()
        .map(|j| JavaInstallationDto {
            path: j.path,
            major_version: j.major_version,
            version_string: j.version_string,
        })
        .collect())
}

/// Selecciona la instalación de Java más adecuada para una versión de Minecraft.
/// Devuelve la ruta al ejecutable.
#[tauri::command]
pub async fn select_java_for_version(
    mc_version: String,
    state: State<'_, AppState>,
) -> Result<JavaInstallationDto, String> {
    let inst = state
        .java_mgr
        .select_for_version(&mc_version)
        .await
        .map_err(|e| e.to_string())?;
    Ok(JavaInstallationDto {
        path: inst.path,
        major_version: inst.major_version,
        version_string: inst.version_string,
    })
}

// ── Settings commands ─────────────────────────────────────────────────────────

/// DTO de configuración expuesto al frontend.
#[derive(Serialize, Clone)]
pub struct SettingsDto {
    pub servers_dir: String,
    pub backups_dir: String,
    pub downloads_dir: String,
    pub default_java_path: String,
    /// Intervalo de backup automático en segundos (0 = desactivado).
    pub backup_interval_secs: u64,
}

#[derive(Deserialize)]
pub struct SaveSettingsCmd {
    pub servers_dir: String,
    pub backups_dir: String,
    pub downloads_dir: String,
    pub default_java_path: String,
    pub backup_interval_secs: u64,
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<SettingsDto, String> {
    let s = state.settings.read().await;
    Ok(SettingsDto {
        servers_dir: s.servers_dir.clone(),
        backups_dir: s.backups_dir.clone(),
        downloads_dir: s.downloads_dir.clone(),
        default_java_path: s.default_java_path.clone(),
        backup_interval_secs: s.backup_interval_secs,
    })
}

#[tauri::command]
pub async fn save_settings(
    cmd: SaveSettingsCmd,
    state: State<'_, AppState>,
) -> Result<SettingsDto, String> {
    if cmd.servers_dir.trim().is_empty() {
        return Err("El directorio de servidores no puede estar vacío".into());
    }
    if cmd.backups_dir.trim().is_empty() {
        return Err("El directorio de backups no puede estar vacío".into());
    }
    if cmd.downloads_dir.trim().is_empty() {
        return Err("El directorio de descargas no puede estar vacío".into());
    }
    if cmd.default_java_path.trim().is_empty() {
        return Err("La ruta de Java por defecto no puede estar vacía".into());
    }

    let next = Settings {
        servers_dir: cmd.servers_dir.trim().to_string(),
        backups_dir: cmd.backups_dir.trim().to_string(),
        downloads_dir: cmd.downloads_dir.trim().to_string(),
        default_java_path: cmd.default_java_path.trim().to_string(),
        backup_interval_secs: cmd.backup_interval_secs,
    };

    ensure_storage_dir("servidores", &next.servers_dir).await?;
    ensure_storage_dir("backups", &next.backups_dir).await?;
    ensure_storage_dir("descargas", &next.downloads_dir).await?;
    ensure_absolute_path("Java por defecto", &next.default_java_path)?;

    state.settings_store.save(&next).await?;
    state
        .backup_mgr
        .set_backups_dir(next.backups_dir.clone())
        .await;

    {
        let mut s = state.settings.write().await;
        *s = next;
    }

    // Reschedule automatic backup with the new interval
    {
        let s = state.settings.read().await;
        state
            .backup_mgr
            .restart_auto_backup(s.backup_interval_secs, s.servers_dir.clone())
            .await;
        info!(
            interval_secs = s.backup_interval_secs,
            "backup scheduler updated"
        );
    }

    let s = state.settings.read().await;
    Ok(SettingsDto {
        servers_dir: s.servers_dir.clone(),
        backups_dir: s.backups_dir.clone(),
        downloads_dir: s.downloads_dir.clone(),
        default_java_path: s.default_java_path.clone(),
        backup_interval_secs: s.backup_interval_secs,
    })
}

fn ensure_absolute_path(label: &str, path: &str) -> Result<(), String> {
    if !Path::new(path).is_absolute() {
        return Err(format!("{} debe ser una ruta absoluta", label));
    }
    Ok(())
}

async fn ensure_storage_dir(label: &str, path: &str) -> Result<(), String> {
    ensure_absolute_path(label, path)?;

    tokio::fs::create_dir_all(path).await.map_err(|e| {
        format!(
            "No se pudo crear el directorio de {} '{}': {}",
            label, path, e
        )
    })?;

    let meta = tokio::fs::metadata(path).await.map_err(|e| {
        format!(
            "No se pudo leer el directorio de {} '{}': {}",
            label, path, e
        )
    })?;
    if !meta.is_dir() {
        return Err(format!(
            "La ruta de {} no es un directorio: {}",
            label, path
        ));
    }

    let probe = Path::new(path).join(format!(".cubed-write-test-{}", Uuid::new_v4()));
    tokio::fs::write(&probe, b"ok").await.map_err(|e| {
        format!(
            "El directorio de {} no es escribible '{}': {}",
            label, path, e
        )
    })?;
    let _ = tokio::fs::remove_file(&probe).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_path_validation_rejects_relative_paths() {
        assert!(ensure_absolute_path("servidores", "cubed/servers").is_err());
    }

    #[tokio::test]
    async fn storage_dir_validation_creates_and_writes_directory() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("servers");

        ensure_storage_dir("servidores", target.to_str().unwrap())
            .await
            .unwrap();

        assert!(target.is_dir());
    }
}
