use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::DomainResult;
use crate::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};

/// Estado del ciclo de vida de un servidor Minecraft.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerStatus {
    #[default]
    Offline,
    Starting,
    Running,
    Stopping,
    Crashed,
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Offline => write!(f, "offline"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Stopping => write!(f, "stopping"),
            Self::Crashed => write!(f, "crashed"),
        }
    }
}

/// Tipo de software del servidor Minecraft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerSoftware {
    Paper,
    Purpur,
    Fabric,
    Forge,
    NeoForge,
}

impl std::fmt::Display for ServerSoftware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Paper => write!(f, "Paper"),
            Self::Purpur => write!(f, "Purpur"),
            Self::Fabric => write!(f, "Fabric"),
            Self::Forge => write!(f, "Forge"),
            Self::NeoForge => write!(f, "NeoForge"),
        }
    }
}

/// Entidad raíz de agregado: Servidor Minecraft.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    id: Uuid,
    name: ServerName,
    version: ServerVersion,
    software: ServerSoftware,
    port: ServerPort,
    java_path: JavaPath,
    status: ServerStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Server {
    /// Crea un nuevo servidor en estado Offline.
    pub fn new(
        name: ServerName,
        version: ServerVersion,
        software: ServerSoftware,
        port: ServerPort,
        java_path: JavaPath,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            version,
            software,
            port,
            java_path,
            status: ServerStatus::Offline,
            created_at: now,
            updated_at: now,
        }
    }

    /// Reconstruye desde persistencia (con id y timestamps conocidos).
    #[allow(clippy::too_many_arguments)]
    pub fn reconstitute(
        id: Uuid,
        name: ServerName,
        version: ServerVersion,
        software: ServerSoftware,
        port: ServerPort,
        java_path: JavaPath,
        status: ServerStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            version,
            software,
            port,
            java_path,
            status,
            created_at,
            updated_at,
        }
    }

    // --- Transitions ---

    pub fn start(&mut self) -> DomainResult<()> {
        use crate::error::DomainError;
        match self.status {
            ServerStatus::Offline | ServerStatus::Crashed => {
                self.transition(ServerStatus::Starting);
                Ok(())
            }
            ref s => Err(DomainError::InvalidTransition {
                from: s.to_string(),
                to: "Starting".into(),
            }),
        }
    }

    pub fn mark_running(&mut self) -> DomainResult<()> {
        use crate::error::DomainError;
        if self.status == ServerStatus::Starting {
            self.transition(ServerStatus::Running);
            Ok(())
        } else {
            Err(DomainError::InvalidTransition {
                from: self.status.to_string(),
                to: "Running".into(),
            })
        }
    }

    pub fn stop(&mut self) -> DomainResult<()> {
        use crate::error::DomainError;
        if self.status == ServerStatus::Running {
            self.transition(ServerStatus::Stopping);
            Ok(())
        } else {
            Err(DomainError::InvalidTransition {
                from: self.status.to_string(),
                to: "Stopping".into(),
            })
        }
    }

    pub fn mark_offline(&mut self) -> DomainResult<()> {
        use crate::error::DomainError;
        match self.status {
            ServerStatus::Stopping | ServerStatus::Crashed => {
                self.transition(ServerStatus::Offline);
                Ok(())
            }
            ref s => Err(DomainError::InvalidTransition {
                from: s.to_string(),
                to: "Offline".into(),
            }),
        }
    }

    pub fn mark_crashed(&mut self) {
        self.transition(ServerStatus::Crashed);
    }

    /// Reconciliación al arrancar Cubed sin un proceso administrado en memoria.
    pub fn recover_as_offline(&mut self) {
        self.transition(ServerStatus::Offline);
    }

    // --- Getters ---

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn name(&self) -> &ServerName {
        &self.name
    }
    pub fn version(&self) -> &ServerVersion {
        &self.version
    }
    pub fn software(&self) -> &ServerSoftware {
        &self.software
    }
    pub fn port(&self) -> &ServerPort {
        &self.port
    }
    pub fn java_path(&self) -> &JavaPath {
        &self.java_path
    }
    pub fn status(&self) -> &ServerStatus {
        &self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn is_running(&self) -> bool {
        self.status == ServerStatus::Running
    }

    fn transition(&mut self, new: ServerStatus) {
        self.status = new;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};

    fn make_server() -> Server {
        Server::new(
            ServerName::new("survival").unwrap(),
            ServerVersion::new("1.21.4").unwrap(),
            ServerSoftware::Paper,
            ServerPort::new(25565).unwrap(),
            JavaPath::new("/usr/bin/java").unwrap(),
        )
    }

    #[test]
    fn new_server_is_offline() {
        assert_eq!(make_server().status(), &ServerStatus::Offline);
    }

    #[test]
    fn start_from_offline_succeeds() {
        let mut s = make_server();
        s.start().unwrap();
        assert_eq!(s.status(), &ServerStatus::Starting);
    }

    #[test]
    fn start_from_running_fails() {
        let mut s = make_server();
        s.start().unwrap();
        s.mark_running().unwrap();
        assert!(s.start().is_err());
    }

    #[test]
    fn full_lifecycle() {
        let mut s = make_server();
        s.start().unwrap();
        s.mark_running().unwrap();
        s.stop().unwrap();
        s.mark_offline().unwrap();
        assert_eq!(s.status(), &ServerStatus::Offline);
    }

    #[test]
    fn crash_then_restart() {
        let mut s = make_server();
        s.start().unwrap();
        s.mark_crashed();
        assert_eq!(s.status(), &ServerStatus::Crashed);
        s.start().unwrap();
        assert_eq!(s.status(), &ServerStatus::Starting);
    }

    #[test]
    fn recover_as_offline_resets_running_state() {
        let mut s = make_server();
        s.start().unwrap();
        s.mark_running().unwrap();
        s.recover_as_offline();
        assert_eq!(s.status(), &ServerStatus::Offline);
    }
}
