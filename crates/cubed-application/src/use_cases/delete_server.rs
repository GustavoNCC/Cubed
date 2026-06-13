use std::sync::Arc;
use uuid::Uuid;
use cubed_domain::entities::ServerStatus;
use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::{FileSystemManager, ServerRepository};

pub struct DeleteServer {
    repo: Arc<dyn ServerRepository>,
    fs: Arc<dyn FileSystemManager>,
}

impl DeleteServer {
    pub fn new(repo: Arc<dyn ServerRepository>, fs: Arc<dyn FileSystemManager>) -> Self {
        Self { repo, fs }
    }

    pub async fn execute(&self, server_id: Uuid, servers_dir: &str) -> ApplicationResult<()> {
        let server = self.repo.find_by_id(server_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Servidor {} no encontrado", server_id))
        })?;

        if server.is_running() || *server.status() == ServerStatus::Starting {
            return Err(ApplicationError::Infrastructure(
                "No se puede eliminar un servidor en ejecución".into(),
            ));
        }

        self.fs.delete_server_dir(servers_dir, server.name().as_str()).await?;
        self.repo.delete(server_id).await
    }
}
