use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::{PortManager, ServerRepository};
use std::sync::Arc;

pub struct ReservePort {
    port_mgr: Arc<dyn PortManager>,
    repo: Arc<dyn ServerRepository>,
}

impl ReservePort {
    pub fn new(port_mgr: Arc<dyn PortManager>, repo: Arc<dyn ServerRepository>) -> Self {
        Self { port_mgr, repo }
    }

    /// Valida que el puerto está libre tanto en el SO como en la BD.
    pub async fn validate(&self, port: u16) -> ApplicationResult<()> {
        self.port_mgr.validate(port).await?;
        if self.repo.port_in_use(port).await? {
            return Err(ApplicationError::Infrastructure(format!(
                "El puerto {} ya está asignado a otro servidor en Cubed",
                port
            )));
        }
        Ok(())
    }

    /// Sugiere el siguiente puerto libre comenzando desde `start`.
    pub async fn suggest_free(&self, start: u16) -> ApplicationResult<u16> {
        let mut candidate = self.port_mgr.find_free_from(start).await?;
        // Si el puerto sugerido ya está en BD, busca el siguiente
        while self.repo.port_in_use(candidate).await? {
            candidate = self.port_mgr.find_free_from(candidate + 1).await?;
        }
        Ok(candidate)
    }
}
