use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::ServerRepository;
use cubed_domain::entities::{Server, ServerSoftware, ServerStatus};
use cubed_domain::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};

/// Registro serializable — espejo plano de la entidad Server.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerRecord {
    id: Uuid,
    name: String,
    version: String,
    software: String,
    port: u16,
    java_path: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<ServerRecord> for Server {
    type Error = ApplicationError;

    fn try_from(r: ServerRecord) -> Result<Self, Self::Error> {
        let name = ServerName::new(&r.name)?;
        let version = ServerVersion::new(&r.version)?;
        let port = ServerPort::new(r.port)?;
        let java = JavaPath::new(&r.java_path)?;
        let software = parse_software(&r.software)?;
        let status = parse_status(&r.status)?;
        Ok(Server::reconstitute(
            r.id,
            name,
            version,
            software,
            port,
            java,
            status,
            r.created_at,
            r.updated_at,
        ))
    }
}

impl From<&Server> for ServerRecord {
    fn from(s: &Server) -> Self {
        Self {
            id: s.id(),
            name: s.name().as_str().to_owned(),
            version: s.version().as_str().to_owned(),
            software: s.software().to_string(),
            port: s.port().value(),
            java_path: s.java_path().as_str().to_owned(),
            status: s.status().to_string(),
            created_at: s.created_at(),
            updated_at: s.updated_at(),
        }
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
            "Software desconocido: '{other}'"
        ))),
    }
}

fn parse_status(s: &str) -> ApplicationResult<ServerStatus> {
    match s {
        "offline" | "starting" | "running" | "stopping" => Ok(ServerStatus::Offline),
        "crashed" => Ok(ServerStatus::Crashed),
        other => Err(ApplicationError::Infrastructure(format!(
            "Estado desconocido: '{other}'"
        ))),
    }
}

/// Repositorio JSON — persiste los servidores en un archivo JSON local.
/// Las escrituras se hacen atómicamente vía archivo temporal + rename.
pub struct JsonServerRepository {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl JsonServerRepository {
    pub fn new(path: PathBuf) -> Self {
        info!(path = %path.display(), "JsonServerRepository iniciado");
        Self {
            path,
            write_lock: Mutex::new(()),
        }
    }

    fn load_records(&self) -> ApplicationResult<Vec<ServerRecord>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let raw = std::fs::read_to_string(&self.path).map_err(|e| {
            ApplicationError::Infrastructure(format!("Error leyendo {}: {e}", self.path.display()))
        })?;
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str::<Vec<ServerRecord>>(&raw).map_err(|e| {
            ApplicationError::Infrastructure(format!(
                "JSON corrupto en {}: {e}",
                self.path.display()
            ))
        })
    }

    fn save_records(&self, records: &[ServerRecord]) -> ApplicationResult<()> {
        let json = serde_json::to_string_pretty(records)
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        // Escritura atómica: temp file + rename
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, &json)
            .map_err(|e| ApplicationError::Infrastructure(format!("Error escribiendo tmp: {e}")))?;
        std::fs::rename(&tmp, &self.path)
            .map_err(|e| ApplicationError::Infrastructure(format!("Error renombrando tmp: {e}")))?;

        Ok(())
    }
}

#[async_trait]
impl ServerRepository for JsonServerRepository {
    async fn save(&self, server: &Server) -> ApplicationResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut records = self.load_records()?;
        let record = ServerRecord::from(server);
        records.retain(|r| r.id != record.id);
        records.push(record);
        self.save_records(&records)?;
        info!(server_id = %server.id(), name = %server.name(), "Servidor guardado en JSON");
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Server>> {
        let records = self.load_records()?;
        let found = records.into_iter().find(|r| r.id == id);
        match found {
            None => Ok(None),
            Some(r) => Ok(Some(Server::try_from(r)?)),
        }
    }

    async fn find_all(&self) -> ApplicationResult<Vec<Server>> {
        let records = self.load_records()?;
        let mut servers = Vec::with_capacity(records.len());
        for r in records {
            match Server::try_from(r) {
                Ok(s) => servers.push(s),
                Err(e) => error!("Error deserializando servidor desde JSON: {e}"),
            }
        }
        info!("Cargados {} servidores desde JSON", servers.len());
        Ok(servers)
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut records = self.load_records()?;
        records.retain(|r| r.id != id);
        self.save_records(&records)?;
        info!(server_id = %id, "Servidor eliminado del JSON");
        Ok(())
    }

    async fn port_in_use(&self, port: u16) -> ApplicationResult<bool> {
        Ok(self.load_records()?.iter().any(|r| r.port == port))
    }
}

/// Verifica integridad entre la base de datos JSON y el sistema de archivos.
/// Registra advertencias para cada inconsistencia encontrada.
pub fn check_integrity(repo_path: &std::path::Path, servers_dir: &str) {
    let servers_path = std::path::Path::new(servers_dir);

    // Leer registros del JSON
    let raw = match std::fs::read_to_string(repo_path) {
        Ok(r) => r,
        Err(_) => {
            info!("check_integrity: no existe archivo JSON todavía, nada que verificar");
            return;
        }
    };
    let records: Vec<ServerRecord> = match serde_json::from_str(&raw) {
        Ok(r) => r,
        Err(e) => {
            error!("check_integrity: JSON corrupto: {e}");
            return;
        }
    };

    // Nombres registrados en DB
    let db_names: std::collections::HashSet<String> =
        records.iter().map(|r| r.name.clone()).collect();

    // Directorios en disco
    let disk_names: std::collections::HashSet<String> = match std::fs::read_dir(servers_path) {
        Err(_) => {
            info!("check_integrity: directorio de servidores no existe aún");
            return;
        }
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect(),
    };

    // Registros DB sin carpeta en disco
    for name in &db_names {
        if !disk_names.contains(name) {
            warn!(
                server_name = %name,
                "INTEGRIDAD: servidor registrado en DB pero sin directorio en disco"
            );
        }
    }

    // Carpetas en disco sin registro en DB
    for name in &disk_names {
        if !db_names.contains(name) {
            warn!(
                dir_name = %name,
                "INTEGRIDAD: directorio en disco sin registro en DB"
            );
        }
    }

    info!(
        db_count = db_names.len(),
        disk_count = disk_names.len(),
        "check_integrity completado"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use cubed_domain::entities::{Server, ServerSoftware};
    use cubed_domain::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};
    use tempfile::tempdir;

    fn make_server(name: &str, port: u16) -> Server {
        Server::new(
            ServerName::new(name).unwrap(),
            ServerVersion::new("1.21.4").unwrap(),
            ServerSoftware::Paper,
            ServerPort::new(port).unwrap(),
            JavaPath::new("/usr/bin/java").unwrap(),
        )
    }

    #[tokio::test]
    async fn persist_and_reload() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("servers.json");

        {
            let repo = JsonServerRepository::new(path.clone());
            let s = make_server("survival", 25565);
            repo.save(&s).await.unwrap();
            assert_eq!(repo.find_all().await.unwrap().len(), 1);
        }

        // New instance — simulates app restart
        let repo2 = JsonServerRepository::new(path.clone());
        let servers = repo2.find_all().await.unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name().as_str(), "survival");
    }

    #[tokio::test]
    async fn multiple_servers_persist() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("servers.json");

        let repo = JsonServerRepository::new(path.clone());
        repo.save(&make_server("s1", 25565)).await.unwrap();
        repo.save(&make_server("s2", 25566)).await.unwrap();
        repo.save(&make_server("s3", 25567)).await.unwrap();

        let repo2 = JsonServerRepository::new(path);
        assert_eq!(repo2.find_all().await.unwrap().len(), 3);
    }

    #[tokio::test]
    async fn delete_persists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("servers.json");

        let repo = JsonServerRepository::new(path.clone());
        let s = make_server("to-delete", 25570);
        let id = s.id();
        repo.save(&s).await.unwrap();
        repo.delete(id).await.unwrap();

        let repo2 = JsonServerRepository::new(path);
        assert_eq!(repo2.find_all().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn status_normalised_to_offline_on_reload() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("servers.json");

        let repo = JsonServerRepository::new(path.clone());
        let mut s = make_server("running-srv", 25575);
        s.start().unwrap();
        s.mark_running().unwrap();
        repo.save(&s).await.unwrap();

        // On reload, Running must be normalised to Offline
        let repo2 = JsonServerRepository::new(path);
        let reloaded = repo2.find_all().await.unwrap();
        use cubed_domain::entities::ServerStatus;
        assert_eq!(reloaded[0].status(), &ServerStatus::Offline);
    }
}
