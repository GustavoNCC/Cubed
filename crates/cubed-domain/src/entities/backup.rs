use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Snapshot de un servidor en un instante dado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backup {
    id: Uuid,
    server_id: Uuid,
    /// Ruta absoluta al archivo de backup (tar.gz).
    path: String,
    /// Tamaño en bytes del archivo.
    size_bytes: u64,
    created_at: DateTime<Utc>,
}

impl Backup {
    pub fn new(server_id: Uuid, path: String, size_bytes: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            server_id,
            path,
            size_bytes,
            created_at: Utc::now(),
        }
    }

    pub fn reconstitute(
        id: Uuid,
        server_id: Uuid,
        path: String,
        size_bytes: u64,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self { id, server_id, path, size_bytes, created_at }
    }

    pub fn id(&self) -> Uuid { self.id }
    pub fn server_id(&self) -> Uuid { self.server_id }
    pub fn path(&self) -> &str { &self.path }
    pub fn size_bytes(&self) -> u64 { self.size_bytes }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_fields_accessible() {
        let srv = Uuid::new_v4();
        let b = Backup::new(srv, "/backups/srv.tar.gz".into(), 1024);
        assert_eq!(b.server_id(), srv);
        assert_eq!(b.size_bytes(), 1024);
    }
}
