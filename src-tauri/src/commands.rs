use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use cubed_domain::entities::{ServerSoftware, ServerStatus, Settings};
use cubed_application::use_cases::{CreateServer, CreateServerInput};
use cubed_application::ports::{BackupRepository, ConsoleLine, ConsoleManager, FileSystemManager, ModpackRepository, ModRepository, NetworkManager, ProcessManager, ResourceMonitor, ServerRepository, TailscaleStatus};
use cubed_application::use_cases::{DeleteBackup, ListBackups, ListMods};
use cubed_application::CubedEvent;
use cubed_infrastructure::{FileBackupManager, FileModManager, MinecraftConsoleManager, MinecraftProcessManager, ModpackInstaller, SysInfoResourceMonitor, TailscaleNetworkManager, TcpPortManager};
use cubed_application::ports::PortManager;
use crate::event_bus::EventBus;

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

pub struct AppState {
    pub repo:        Arc<dyn ServerRepository>,
    pub fs:          Arc<dyn FileSystemManager>,
    pub console:     Arc<MinecraftConsoleManager>,
    pub process_mgr: Arc<MinecraftProcessManager>,
    /// Directorio base donde se almacenan los servidores.
    pub servers_dir: String,
    pub resources:   Arc<SysInfoResourceMonitor>,
    pub backup_repo: Arc<dyn BackupRepository>,
    pub backup_mgr:  Arc<FileBackupManager>,
    pub mod_repo:      Arc<dyn ModRepository>,
    pub mod_mgr:       Arc<FileModManager>,
    pub modpack_repo:  Arc<dyn ModpackRepository>,
    pub modpack_inst:  Arc<ModpackInstaller>,
    pub network:       Arc<TailscaleNetworkManager>,
    pub event_bus:     Arc<EventBus>,
    /// Configuración global mutable en memoria (persistida en futuras fases).
    pub settings:      Arc<RwLock<Settings>>,
}

// ── Server commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerDto>, String> {
    state.repo.find_all().await
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
    let software = parse_software(&cmd.software)?;
    let uc = CreateServer::new(state.repo.clone(), state.fs.clone());
    let server = uc.execute(CreateServerInput {
        name: cmd.name.trim().to_string(),
        version: cmd.version,
        software,
        port: cmd.port,
        java_path: cmd.java_path,
        servers_dir: cmd.servers_dir,
    }).await.map_err(|e| e.to_string())?;
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
    let mut server = state.repo.find_by_id(uuid).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;

    let work_dir  = format!("{}/{}", state.servers_dir, server.name());
    let jar_path  = format!("{}/server.jar", work_dir);

    // Reject early if JAR is missing
    if !std::path::Path::new(&jar_path).exists() {
        return Err(format!(
            "JAR no encontrado en '{}'. Descarga el servidor primero.", jar_path
        ));
    }

    // Domain transition → Starting
    server.start().map_err(|e| e.to_string())?;
    state.repo.save(&server).await.map_err(|e| e.to_string())?;

    // Spawn actual Java process
    let (pid, stdin, stdout, stderr) = state.process_mgr
        .spawn_with_io(uuid, server.java_path().as_str(), &jar_path, &work_dir, 2048)
        .await
        .map_err(|e| e.to_string())?;

    info!(server_id = %uuid, pid, "java process spawned");

    // Register stdin so console commands can be sent
    state.console.register_stdin(uuid, stdin).await;

    // Build a callback that:
    //   1. Forwards lines to the Tauri frontend
    //   2. Detects the "Done" line → marks Running
    //   3. Detects a crash / process end → marks Crashed
    let event_name = format!("console-line:{}", id);
    let app_cb    = app.clone();
    let repo_cb   = state.repo.clone();
    let eb_cb     = state.event_bus.clone();
    let running   = Arc::new(AtomicBool::new(false));
    let running2  = running.clone();

    state.console.attach(uuid, Box::new(move |line: ConsoleLine| {
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
            let eb   = eb_cb.clone();
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
    })).await.map_err(|e| e.to_string())?;

    // Start reading stdout/stderr (lines go through the callback above)
    state.console.spawn_readers(uuid, stdout, stderr).await;

    // Background watcher: when the process dies, mark offline or crashed
    let repo_w   = state.repo.clone();
    let proc_w   = state.process_mgr.clone();
    let eb_w     = state.event_bus.clone();
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
            let was_running  = running_w.load(Ordering::Relaxed);
            if was_stopping || was_running {
                // Clean shutdown path: Stopping → Offline
                let _ = srv.mark_offline();
            } else {
                // Never reached Running → Crashed
                srv.mark_crashed();
            }
            let _ = repo_w.save(&srv).await;
            eb_w.emit(CubedEvent::ServerStopped { server_id: uuid });
            info!(server_id = %uuid, "process exited, status updated");
        }
    });

    Ok(server_to_dto(&server))
}

#[tauri::command]
pub async fn stop_server(id: String, state: State<'_, AppState>) -> Result<ServerDto, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut server = state.repo.find_by_id(uuid).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;

    // Transition domain state (Running → Stopping)
    server.stop().map_err(|e| e.to_string())?;
    state.repo.save(&server).await.map_err(|e| e.to_string())?;

    // Send "stop" command to the server process via stdin
    if let Err(e) = state.console.send_command(uuid, "stop").await {
        warn!(server_id = %uuid, err = %e, "could not send stop via stdin, killing process");
        state.process_mgr.kill(uuid).await.ok();
    }

    state.event_bus.emit(CubedEvent::ServerStopped { server_id: uuid });
    info!(server_id = %uuid, "server stop requested");
    Ok(server_to_dto(&server))
}

#[tauri::command]
pub async fn delete_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let server = state.repo.find_by_id(uuid).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;
    if server.is_running() {
        warn!(server_id = %uuid, "delete rejected: server is running");
        return Err("No se puede eliminar un servidor en ejecución".into());
    }
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

    state.console
        .attach(uuid, Box::new(move |line: ConsoleLine| {
            let evt = ConsoleLineEvent {
                server_id: line.server_id.to_string(),
                is_stdout: line.is_stdout,
                text: line.text,
            };
            app_clone.emit(&event_clone, evt).ok();
        }))
        .await
        .map_err(|e| e.to_string())?;

    // Return current buffer snapshot for immediate display
    Ok(state.console.tail(uuid, 500).into_iter().map(line_to_event).collect())
}

/// Envía un comando a stdin del servidor.
#[tauri::command]
pub async fn send_console_command(
    id: String,
    command: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state.console
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
    Ok(state.console.tail(uuid, n).into_iter().map(line_to_event).collect())
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
            ServerStatus::Offline  => "offline",
            ServerStatus::Starting => "starting",
            ServerStatus::Running  => "running",
            ServerStatus::Stopping => "stopping",
            ServerStatus::Crashed  => "crashed",
        }.to_string(),
    }
}

fn line_to_event(l: ConsoleLine) -> ConsoleLineEvent {
    ConsoleLineEvent { server_id: l.server_id.to_string(), is_stdout: l.is_stdout, text: l.text }
}

fn parse_software(s: &str) -> Result<ServerSoftware, String> {
    match s {
        "Paper"    => Ok(ServerSoftware::Paper),
        "Purpur"   => Ok(ServerSoftware::Purpur),
        "Fabric"   => Ok(ServerSoftware::Fabric),
        "Forge"    => Ok(ServerSoftware::Forge),
        "NeoForge" => Ok(ServerSoftware::NeoForge),
        other => Err(format!("Software desconocido: {}", other)),
    }
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
    let s = state.resources.system_stats().await.map_err(|e| e.to_string())?;
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
    let opt = state.resources.server_stats(uuid, pid).await.map_err(|e| e.to_string())?;
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
    let backup = state.backup_mgr
        .backup_server(uuid, &server_name, &server_dir)
        .await
        .map_err(|e| e.to_string())?;
    state.event_bus.emit(CubedEvent::BackupCreated { server_id: uuid, backup_id: backup.id() });
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
    state.backup_mgr
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
    let entry = state.mod_mgr
        .install_mod(uuid, &source_path, &mods_dir)
        .await
        .map_err(|e| e.to_string())?;
    Ok(mod_to_dto(&entry))
}

/// Elimina un mod: borra el .jar y lo quita del registro.
#[tauri::command]
pub async fn remove_mod(
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&mod_id).map_err(|e| e.to_string())?;
    state.mod_mgr.remove_mod(uuid).await.map_err(|e| e.to_string())
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

    let (modpack, summary) = state.modpack_inst
        .install(uuid, &source_path, &install_dir, move |progress| {
            let _ = app_clone.emit(&event_clone, serde_json::json!({
                "total": progress.total,
                "done":  progress.done,
                "file":  progress.current_file,
            }));
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
    let mut list = state.modpack_repo
        .find_by_server(uuid)
        .await
        .map_err(|e| e.to_string())?;
    list.sort_by(|a, b| a.name().cmp(b.name()));
    Ok(list.iter().map(modpack_to_dto).collect())
}

/// Elimina el registro de un modpack (no borra los mods ya instalados).
#[tauri::command]
pub async fn delete_modpack(
    modpack_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&modpack_id).map_err(|e| e.to_string())?;
    state.modpack_repo.delete(uuid).await.map_err(|e| e.to_string())
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
        TailscaleStatus::NotInstalled => TailscaleStatusDto { state: "not_installed".into(), ip: None, hostname: None },
        TailscaleStatus::Disconnected => TailscaleStatusDto { state: "disconnected".into(), ip: None, hostname: None },
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
    state.network.is_installed().await.map_err(|e| e.to_string())
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
    state.network.tailscale_ip().await.map_err(|e| e.to_string())
}

/// Devuelve el primer puerto libre >= 25565 que no esté en uso por el SO
/// ni por ningún servidor registrado en Cubed.
#[tauri::command]
pub async fn suggest_free_port(state: State<'_, AppState>) -> Result<u16, String> {
    let mgr = TcpPortManager::new();
    let used: std::collections::HashSet<u16> = state.repo
        .find_all().await.map_err(|e| e.to_string())?
        .iter().map(|s| s.port().value()).collect();

    let mut candidate = 25565u16;
    loop {
        if candidate > 65535 {
            return Err("No se encontró un puerto libre".into());
        }
        if !used.contains(&candidate) && mgr.is_free(candidate).await.unwrap_or(false) {
            return Ok(candidate);
        }
        candidate += 1;
    }
}

/// Construye la dirección de conexión al servidor: `<tailscale_ip>:<port>`.
/// Útil para copiar al portapapeles.
#[tauri::command]
pub async fn server_connect_address(
    server_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let uuid = Uuid::parse_str(&server_id).map_err(|e| e.to_string())?;
    let server = state.repo.find_by_id(uuid).await.map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", server_id))?;
    let ip = state.network.tailscale_ip().await.map_err(|e| e.to_string())?;
    Ok(ip.map(|addr| format!("{}:{}", addr, server.port().value())))
}

// ── Settings commands ─────────────────────────────────────────────────────────

/// DTO de configuración expuesto al frontend.
#[derive(Serialize, Clone)]
pub struct SettingsDto {
    pub servers_dir:           String,
    pub backups_dir:           String,
    pub downloads_dir:         String,
    pub default_java_path:     String,
    /// Intervalo de backup automático en segundos (0 = desactivado).
    pub backup_interval_secs:  u64,
}

#[derive(Deserialize)]
pub struct SaveSettingsCmd {
    pub servers_dir:           String,
    pub backups_dir:           String,
    pub downloads_dir:         String,
    pub default_java_path:     String,
    pub backup_interval_secs:  u64,
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<SettingsDto, String> {
    let s = state.settings.read().await;
    Ok(SettingsDto {
        servers_dir:          s.servers_dir.clone(),
        backups_dir:          s.backups_dir.clone(),
        downloads_dir:        s.downloads_dir.clone(),
        default_java_path:    s.default_java_path.clone(),
        backup_interval_secs: s.backup_interval_secs,
    })
}

#[tauri::command]
pub async fn save_settings(
    cmd: SaveSettingsCmd,
    state: State<'_, AppState>,
) -> Result<SettingsDto, String> {
    // Basic validation
    if cmd.servers_dir.trim().is_empty() {
        return Err("El directorio de servidores no puede estar vacío".into());
    }
    if cmd.backups_dir.trim().is_empty() {
        return Err("El directorio de backups no puede estar vacío".into());
    }

    {
        let mut s = state.settings.write().await;
        s.servers_dir          = cmd.servers_dir.trim().to_string();
        s.backups_dir          = cmd.backups_dir.trim().to_string();
        s.downloads_dir        = cmd.downloads_dir.trim().to_string();
        s.default_java_path    = cmd.default_java_path.trim().to_string();
        s.backup_interval_secs = cmd.backup_interval_secs;
    }

    // Reschedule automatic backup with the new interval
    {
        let s = state.settings.read().await;
        state.backup_mgr
            .restart_auto_backup(s.backup_interval_secs, s.servers_dir.clone())
            .await;
        info!(interval_secs = s.backup_interval_secs, "backup scheduler updated");
    }

    let s = state.settings.read().await;
    Ok(SettingsDto {
        servers_dir:          s.servers_dir.clone(),
        backups_dir:          s.backups_dir.clone(),
        downloads_dir:        s.downloads_dir.clone(),
        default_java_path:    s.default_java_path.clone(),
        backup_interval_secs: s.backup_interval_secs,
    })
}
