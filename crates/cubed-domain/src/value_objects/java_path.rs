use serde::{Deserialize, Serialize};
use crate::error::{DomainError, DomainResult};

/// Ruta absoluta al binario de Java.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaPath(String);

impl JavaPath {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.is_empty() {
            return Err(DomainError::Validation("La ruta de Java no puede estar vacía".into()));
        }
        if !value.starts_with('/') {
            return Err(DomainError::Validation(
                "La ruta de Java debe ser absoluta (comenzar con '/')".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for JavaPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_path() { assert!(JavaPath::new("/usr/bin/java").is_ok()); }
    #[test]
    fn relative_path_fails() { assert!(JavaPath::new("usr/bin/java").is_err()); }
    #[test]
    fn empty_fails() { assert!(JavaPath::new("").is_err()); }
}
