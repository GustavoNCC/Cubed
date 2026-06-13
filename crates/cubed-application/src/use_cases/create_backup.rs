use std::sync::Arc;
use uuid::Uuid;
use cubed_domain::entities::Backup;
use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::{BackupRepository, ServerRepository};

pub struct CreateBackupInput {
    pub server_id: Uuid,
    /// Ruta absoluta al archivo .tar.gz creado por la infraestructura.
    pub path: String,
    pub size_bytes: u64,
}

pub struct CreateBackup {
    servers: Arc<dyn ServerRepository>,
    backups: Arc<dyn BackupRepository>,
}

impl CreateBackup {
    pub fn new(servers: Arc<dyn ServerRepository>, backups: Arc<dyn BackupRepository>) -> Self {
        Self { servers, backups }
    }

    pub async fn execute(&self, input: CreateBackupInput) -> ApplicationResult<Backup> {
        // Verify server exists
        self.servers
            .find_by_id(input.server_id)
            .await?
            .ok_or_else(|| ApplicationError::Infrastructure(
                format!("Servidor {} no encontrado", input.server_id),
            ))?;

        let backup = Backup::new(input.server_id, input.path, input.size_bytes);
        self.backups.save(&backup).await?;
        Ok(backup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use async_trait::async_trait;
    use cubed_domain::entities::{Server, ServerSoftware};
    use cubed_domain::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};
    use crate::ports::ServerRepository;

    struct FakeServerRepo(Server);
    #[async_trait]
    impl ServerRepository for FakeServerRepo {
        async fn save(&self, _: &Server) -> ApplicationResult<()> { Ok(()) }
        async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Server>> {
            if id == self.0.id() { Ok(Some(self.0.clone())) } else { Ok(None) }
        }
        async fn find_all(&self) -> ApplicationResult<Vec<Server>> { Ok(vec![self.0.clone()]) }
        async fn delete(&self, _: Uuid) -> ApplicationResult<()> { Ok(()) }
        async fn port_in_use(&self, _: u16) -> ApplicationResult<bool> { Ok(false) }
    }

    struct FakeBackupRepo(Mutex<HashMap<Uuid, Backup>>);
    impl FakeBackupRepo { fn new() -> Arc<Self> { Arc::new(Self(Mutex::new(HashMap::new()))) } }
    #[async_trait]
    impl BackupRepository for FakeBackupRepo {
        async fn save(&self, b: &Backup) -> ApplicationResult<()> {
            self.0.lock().unwrap().insert(b.id(), b.clone()); Ok(())
        }
        async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Backup>> {
            Ok(self.0.lock().unwrap().get(&id).cloned())
        }
        async fn find_by_server(&self, sid: Uuid) -> ApplicationResult<Vec<Backup>> {
            Ok(self.0.lock().unwrap().values().filter(|b| b.server_id() == sid).cloned().collect())
        }
        async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
            self.0.lock().unwrap().remove(&id); Ok(())
        }
    }

    fn make_server() -> Server {
        Server::new(
            ServerName::new("test").unwrap(),
            ServerVersion::new("1.21").unwrap(),
            ServerSoftware::Paper,
            ServerPort::new(25565).unwrap(),
            JavaPath::new("/usr/bin/java").unwrap(),
        )
    }

    #[tokio::test]
    async fn create_backup_persists_and_returns() {
        let server = make_server();
        let sid = server.id();
        let repos = Arc::new(FakeServerRepo(server));
        let backups = FakeBackupRepo::new();
        let uc = CreateBackup::new(repos, backups.clone());
        let b = uc.execute(CreateBackupInput {
            server_id: sid,
            path: "/backups/test.tar.gz".into(),
            size_bytes: 4096,
        }).await.unwrap();
        assert_eq!(b.server_id(), sid);
        assert_eq!(b.size_bytes(), 4096);
        // Also persisted
        let found = backups.find_by_id(b.id()).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn create_backup_fails_unknown_server() {
        let server = make_server();
        let repos = Arc::new(FakeServerRepo(server));
        let backups = FakeBackupRepo::new();
        let uc = CreateBackup::new(repos, backups);
        let result = uc.execute(CreateBackupInput {
            server_id: Uuid::new_v4(),
            path: "/backups/x.tar.gz".into(),
            size_bytes: 0,
        }).await;
        assert!(result.is_err());
    }
}
