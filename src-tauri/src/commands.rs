use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

use cubed_domain::entities::{ServerSoftware, ServerStatus};
use cubed_application::use_cases::{CreateServer, CreateServerInput};
use cubed_application::ports::ServerRepository;

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

// ── App state ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub repo: Arc<dyn ServerRepository>,
    pub fs: Arc<dyn cubed_application::ports::FileSystemManager>,
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerDto>, String> {
    state
        .repo
        .find_all()
        .await
        .map_err(|e| e.to_string())
        .map(|servers| servers.iter().map(server_to_dto).collect())
}

#[tauri::command]
pub async fn create_server(
    cmd: CreateServerCmd,
    state: State<'_, AppState>,
) -> Result<ServerDto, String> {
    let software = parse_software(&cmd.software)?;

    let uc = CreateServer::new(state.repo.clone(), state.fs.clone());
    let server = uc
        .execute(CreateServerInput {
            name: cmd.name,
            version: cmd.version,
            software,
            port: cmd.port,
            java_path: cmd.java_path,
            servers_dir: cmd.servers_dir,
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(server_to_dto(&server))
}

#[tauri::command]
pub async fn start_server(id: String, state: State<'_, AppState>) -> Result<ServerDto, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let mut server = state
        .repo
        .find_by_id(uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Servidor {} no encontrado", id))?;

    server.start().map_err(|e| e.to_string())?;
    state.repo.save(&server).await.map_err(|e| e.to_string())?;
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

    server.stop().map_err(|e| e.to_string())?;
    state.repo.save(&server).await.map_err(|e| e.to_string())?;
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

    if server.is_running() {
        return Err("No se puede eliminar un servidor en ejecución".into());
    }

    state.repo.delete(uuid).await.map_err(|e| e.to_string())
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
        }
        .to_string(),
    }
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
