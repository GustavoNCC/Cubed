use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("violación de regla de negocio: {0}")]
    BusinessRule(String),

    #[error("validación fallida: {0}")]
    Validation(String),

    #[error("transición de estado inválida: {from} → {to}")]
    InvalidTransition { from: String, to: String },

    #[error("servidor no encontrado: {0}")]
    ServerNotFound(uuid::Uuid),
}

pub type DomainResult<T> = Result<T, DomainError>;
