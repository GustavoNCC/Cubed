use crate::error::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};

/// Versión de Minecraft (e.g. "1.21.4").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerVersion(String);

impl ServerVersion {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.is_empty() {
            return Err(DomainError::Validation(
                "La versión no puede estar vacía".into(),
            ));
        }
        // Formato mínimo: dígito.dígito (e.g. "1.21" o "1.21.4")
        let parts: Vec<&str> = value.split('.').collect();
        if parts.len() < 2 || parts.iter().any(|p| p.parse::<u32>().is_err()) {
            return Err(DomainError::Validation(format!(
                "Versión inválida: '{}'. Formato esperado: X.Y o X.Y.Z",
                value
            )));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ServerVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_version() {
        assert!(ServerVersion::new("1.21.4").is_ok());
    }
    #[test]
    fn two_part_version() {
        assert!(ServerVersion::new("1.21").is_ok());
    }
    #[test]
    fn invalid_version() {
        assert!(ServerVersion::new("latest").is_err());
    }
    #[test]
    fn empty_version() {
        assert!(ServerVersion::new("").is_err());
    }
}
