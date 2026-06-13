use std::sync::Arc;
use uuid::Uuid;
use cubed_domain::entities::ServerStatus;
use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::{ProcessManager, ServerRepository};

/// Parámetros para arrancar un servidor.
pub struct RunServerInput {
    pub server_id: Uuid,
    /// Ruta al JAR del servidor (descargado en Fase 6).
    pub jar_path: String,
    /// Directorio de trabajo del servidor (su carpeta en servers_dir).
    pub work_dir: String,
    /// Memoria máxima en MB para la JVM.
    pub memory_mb: u32,
}

pub struct RunServer {
    repo: Arc<dyn ServerRepository>,
    proc: Arc<dyn ProcessManager>,
}

impl RunServer {
    pub fn new(repo: Arc<dyn ServerRepository>, proc: Arc<dyn ProcessManager>) -> Self {
        Self { repo, proc }
    }

    pub async fn start(&self, input: RunServerInput) -> ApplicationResult<u32> {
        let mut server = self.repo.find_by_id(input.server_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Servidor {} no encontrado", input.server_id))
        })?;

        server.start()?;
        self.repo.save(&server).await?;

        let pid = self.proc
            .spawn(
                input.server_id,
                server.java_path().as_str(),
                &input.jar_path,
                &input.work_dir,
                input.memory_mb,
            )
            .await?;

        Ok(pid)
    }

    pub async fn stop(&self, server_id: Uuid) -> ApplicationResult<()> {
        let mut server = self.repo.find_by_id(server_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Servidor {} no encontrado", server_id))
        })?;

        server.stop()?;
        self.repo.save(&server).await?;
        self.proc.stop(server_id).await
    }

    pub async fn kill(&self, server_id: Uuid) -> ApplicationResult<()> {
        let mut server = self.repo.find_by_id(server_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Servidor {} no encontrado", server_id))
        })?;

        server.mark_crashed();
        self.repo.save(&server).await?;
        self.proc.kill(server_id).await
    }

    pub async fn restart(&self, input: RunServerInput) -> ApplicationResult<u32> {
        let server_id = input.server_id;

        // Si está corriendo, para primero
        if let Ok(Some(s)) = self.repo.find_by_id(server_id).await {
            if *s.status() == ServerStatus::Running {
                self.proc.stop(server_id).await.ok();
            }
        }

        // Marca offline en dominio
        if let Ok(Some(mut s)) = self.repo.find_by_id(server_id).await {
            if matches!(s.status(), ServerStatus::Stopping | ServerStatus::Crashed) {
                s.mark_offline()?;
                self.repo.save(&s).await?;
            }
        }

        self.start(input).await
    }
}
