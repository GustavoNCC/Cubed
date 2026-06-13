use std::sync::Arc;
use uuid::Uuid;
use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::ServerRepository;

pub struct RestartServer {
    repo: Arc<dyn ServerRepository>,
}

impl RestartServer {
    pub fn new(repo: Arc<dyn ServerRepository>) -> Self {
        Self { repo }
    }

    /// Detiene el servidor (→ Stopping) y vuelve a iniciarlo (→ Starting).
    pub async fn execute(&self, server_id: Uuid) -> ApplicationResult<()> {
        let mut server = self.repo.find_by_id(server_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Servidor {} no encontrado", server_id))
        })?;

        server.stop()?;
        server.mark_offline()?;
        server.start()?;
        self.repo.save(&server).await
    }
}
