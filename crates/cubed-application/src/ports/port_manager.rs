use crate::error::ApplicationResult;
use async_trait::async_trait;

/// Puerto para gestión de puertos de red.
#[async_trait]
pub trait PortManager: Send + Sync {
    /// Comprueba si el puerto está libre en el sistema operativo.
    async fn is_free(&self, port: u16) -> ApplicationResult<bool>;

    /// Encuentra el siguiente puerto libre a partir de `start`.
    async fn find_free_from(&self, start: u16) -> ApplicationResult<u16>;

    /// Valida que el puerto está en rango permitido y libre en el sistema.
    /// Retorna error si el puerto está ocupado o fuera de rango.
    async fn validate(&self, port: u16) -> ApplicationResult<()>;
}
