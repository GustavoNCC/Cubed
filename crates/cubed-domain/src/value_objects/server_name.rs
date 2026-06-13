use crate::error::{DomainError, DomainResult};
use serde::{Deserialize, Serialize};

/// Nombre válido de servidor (1-64 caracteres, sin espacios).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerName(String);

impl ServerName {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.is_empty() || value.len() > 64 {
            return Err(DomainError::Validation(
                "El nombre del servidor debe tener entre 1 y 64 caracteres".into(),
            ));
        }
        if value.contains(' ') {
            return Err(DomainError::Validation(
                "El nombre del servidor no puede contener espacios".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ServerName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_name() {
        assert!(ServerName::new("survival").is_ok());
    }
    #[test]
    fn empty_name_fails() {
        assert!(ServerName::new("").is_err());
    }
    #[test]
    fn name_with_spaces_fails() {
        assert!(ServerName::new("my server").is_err());
    }
    #[test]
    fn name_too_long_fails() {
        assert!(ServerName::new("a".repeat(65)).is_err());
    }
}
