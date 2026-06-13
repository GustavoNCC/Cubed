use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_domain::entities::{Server, ServerSoftware, ServerStatus};
use cubed_domain::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};

/// Fila tal como la devuelve PostgreSQL.
#[derive(Debug, FromRow)]
pub struct ServerRow {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub software: String,
    pub port: i32,
    pub java_path: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ServerRow {
    pub fn into_domain(self) -> ApplicationResult<Server> {
        let name = ServerName::new(&self.name)?;
        let version = ServerVersion::new(&self.version)?;
        let port = ServerPort::new(self.port as u16)?;
        let java_path = JavaPath::new(&self.java_path)?;

        let software = parse_software(&self.software)?;
        let status = parse_status(&self.status)?;

        Ok(Server::reconstitute(
            self.id,
            name,
            version,
            software,
            port,
            java_path,
            status,
            self.created_at,
            self.updated_at,
        ))
    }
}

fn parse_software(s: &str) -> ApplicationResult<ServerSoftware> {
    match s {
        "Paper" => Ok(ServerSoftware::Paper),
        "Purpur" => Ok(ServerSoftware::Purpur),
        "Fabric" => Ok(ServerSoftware::Fabric),
        "Forge" => Ok(ServerSoftware::Forge),
        "NeoForge" => Ok(ServerSoftware::NeoForge),
        other => Err(ApplicationError::Infrastructure(format!(
            "Software desconocido en base de datos: '{}'",
            other
        ))),
    }
}

fn parse_status(s: &str) -> ApplicationResult<ServerStatus> {
    match s {
        "offline" => Ok(ServerStatus::Offline),
        "starting" => Ok(ServerStatus::Starting),
        "running" => Ok(ServerStatus::Running),
        "stopping" => Ok(ServerStatus::Stopping),
        "crashed" => Ok(ServerStatus::Crashed),
        other => Err(ApplicationError::Infrastructure(format!(
            "Estado desconocido en base de datos: '{}'",
            other
        ))),
    }
}
