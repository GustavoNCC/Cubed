//! Errores de la capa de aplicación.

use cubed_domain::error::DomainError;
use thiserror::Error;

/// Error de un caso de uso.
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// Error propagado desde el dominio.
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// Falla en un puerto de infraestructura.
    #[error("error de infraestructura: {0}")]
    Infrastructure(String),
}

/// Alias de resultado para casos de uso.
pub type ApplicationResult<T> = Result<T, ApplicationError>;
