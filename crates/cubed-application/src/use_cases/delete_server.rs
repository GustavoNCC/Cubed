use std::sync::Arc;
use uuid::Uuid;
use cubed_domain::entities::ServerStatus;
use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::ServerRepository;

pub struct DeleteServer {
    repo: Arc<dyn ServerRepository>,
}

impl DeleteServer {
    pub fn new(repo: Arc<dyn ServerRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, server_id: Uuid) -> ApplicationResult<()> {
        let server = self.repo.find_by_id(server_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Servidor {} no encontrado", server_id))
        })?;

        if server.is_running() || *server.status() == ServerStatus::Starting {
            return Err(ApplicationError::Infrastructure(
                "No se puede eliminar un servidor en ejecución".into(),
            ));
        }

        self.repo.delete(server_id).await
    }
}
