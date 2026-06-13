use serde::{Deserialize, Serialize};
use crate::error::{DomainError, DomainResult};

/// Puerto de red válido para un servidor Minecraft (1024–65535).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerPort(u16);

impl ServerPort {
    pub fn new(value: u16) -> DomainResult<Self> {
        if value < 1024 {
            return Err(DomainError::Validation(
                "El puerto debe ser >= 1024 (puertos privilegiados no permitidos)".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn value(self) -> u16 { self.0 }
}

impl std::fmt::Display for ServerPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_port() { assert!(ServerPort::new(25565).is_ok()); }
    #[test]
    fn privileged_port_fails() { assert!(ServerPort::new(80).is_err()); }
    #[test]
    fn max_port_ok() { assert!(ServerPort::new(65535).is_ok()); }
}
