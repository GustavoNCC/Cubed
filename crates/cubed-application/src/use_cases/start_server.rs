use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::ServerRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct StartServer {
    repo: Arc<dyn ServerRepository>,
}

impl StartServer {
    pub fn new(repo: Arc<dyn ServerRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, server_id: Uuid) -> ApplicationResult<()> {
        let mut server = self.repo.find_by_id(server_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Servidor {} no encontrado", server_id))
        })?;

        server.start()?;
        self.repo.save(&server).await
    }
}
