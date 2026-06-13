use std::sync::Arc;
use uuid::Uuid;
use cubed_domain::entities::ServerStatus;
use crate::error::ApplicationResult;
use crate::ports::{ProcessManager, ServerRepository};

pub struct MonitorServer {
    repo: Arc<dyn ServerRepository>,
    proc: Arc<dyn ProcessManager>,
}

impl MonitorServer {
    pub fn new(repo: Arc<dyn ServerRepository>, proc: Arc<dyn ProcessManager>) -> Self {
        Self { repo, proc }
    }

    /// Sincroniza el estado del dominio con la realidad del proceso.
    /// Si el proceso murió inesperadamente, marca el servidor como Crashed.
    pub async fn sync(&self, server_id: Uuid) -> ApplicationResult<ServerStatus> {
        let mut server = match self.repo.find_by_id(server_id).await? {
            Some(s) => s,
            None => return Ok(ServerStatus::Offline),
        };

        let was_supposed_to_run = matches!(
            server.status(),
            ServerStatus::Starting | ServerStatus::Running | ServerStatus::Stopping
        );

        if was_supposed_to_run {
            let alive = self.proc.is_alive(server_id).await?;
            if !alive {
                match server.status() {
                    ServerStatus::Stopping => {
                        server.mark_offline()?;
                    }
                    _ => {
                        server.mark_crashed();
                    }
                }
                self.repo.save(&server).await?;
            }
        }

        Ok(server.status().clone())
    }

    /// Devuelve el estado actual sin modificar nada.
    pub async fn status(&self, server_id: Uuid) -> ApplicationResult<Option<ServerStatus>> {
        Ok(self.repo.find_by_id(server_id).await?.map(|s| s.status().clone()))
    }
}
