use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use cubed_domain::entities::{ServerSoftware, ServerStatus};
use cubed_application::use_cases::{CreateServer, CreateServerInput};
use cubed_application::ports::{BackupRepository, ConsoleLine, ConsoleManager, FileSystemManager, ModRepository, ResourceMonitor, ServerRepository};
use cubed_application::use_cases::{CreateBackup, CreateBackupInput, DeleteBackup, ListBackups, ListMods, RemoveMod};
use cubed_infrastructure::{FileBackupManager, FileModManager, InMemoryBackupRepo, MinecraftConsoleManager, SysInfoResourceMonitor};

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
    pub resources:   Arc<SysInfoResourceMonitor>,
    pub backup_repo: Arc<dyn BackupRepository>,
    pub backup_mgr:  Arc<FileBackupManager>,
    pub mod_repo:    Arc<dyn ModRepository>,
    pub mod_mgr:     Arc<FileModManager>,
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
    let software = parse_software(&cmd.software)?;
    let uc = CreateServer::new(state.repo.clone(), state.fs.clone());
    let server = uc.execute(CreateServerInput {
        name: cmd.name,
        version: cmd.version,
        software,
        port: cmd.port,
        java_path: cmd.java_path,
        servers_dir: cmd.servers_dir,
    }).await.map_err(|e| e.to_string())?;
    Ok(server_to_dto(&server))
}

#[tauri::command]
pub async fn start_server(id: String, state: State<'_, AppState>) -> Result<ServerDto, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut server = state.repo.find_by_id(uuid).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;
    server.start().map_err(|e| e.to_string())?;
    state.repo.save(&server).await.map_err(|e| e.to_string())?;
    Ok(server_to_dto(&server))
}

#[tauri::command]
pub async fn stop_server(id: String, state: State<'_, AppState>) -> Result<ServerDto, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut server = state.repo.find_by_id(uuid).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;
    server.stop().map_err(|e| e.to_string())?;
    state.repo.save(&server).await.map_err(|e| e.to_string())?;
    Ok(server_to_dto(&server))
}

#[tauri::command]
pub async fn delete_server(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let server = state.repo.find_by_id(uuid).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;
    if server.is_running() {
        return Err("No se puede eliminar un servidor en ejecución".into());
    }
    state.repo.delete(uuid).await.map_err(|e| e.to_string())
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
