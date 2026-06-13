use async_trait::async_trait;
use std::net::TcpListener;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::PortManager;

const MIN_PORT: u16 = 1024;
const MAX_PORT: u16 = 65535;

/// Comprueba si un puerto TCP está libre intentando hacer bind en 0.0.0.0:<port>.
fn port_is_free(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).is_ok()
}

pub struct TcpPortManager;

impl TcpPortManager {
    pub fn new() -> Self { Self }
}

impl Default for TcpPortManager {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl PortManager for TcpPortManager {
    async fn is_free(&self, port: u16) -> ApplicationResult<bool> {
        Ok(port_is_free(port))
    }

    async fn find_free_from(&self, start: u16) -> ApplicationResult<u16> {
        let start = start.max(MIN_PORT);
        for port in start..=MAX_PORT {
            if port_is_free(port) {
                return Ok(port);
            }
        }
        Err(ApplicationError::Infrastructure(
            format!("No hay puertos libres en el rango {}–{}", start, MAX_PORT),
        ))
    }

    async fn validate(&self, port: u16) -> ApplicationResult<()> {
        if port < MIN_PORT {
            return Err(ApplicationError::Infrastructure(
                format!("El puerto {} está reservado (mínimo {})", port, MIN_PORT),
            ));
        }
        if !port_is_free(port) {
            return Err(ApplicationError::Infrastructure(
                format!("El puerto {} está ocupado por otro proceso", port),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn find_free_returns_a_port() {
        let mgr = TcpPortManager::new();
        let port = mgr.find_free_from(25565).await.unwrap();
        assert!(port >= 25565);
        // Port may be taken between find_free and assertion; just verify range.
    }

    #[tokio::test]
    async fn validate_privileged_port_fails() {
        let mgr = TcpPortManager::new();
        assert!(mgr.validate(80).await.is_err());
    }

    #[tokio::test]
    async fn is_free_on_bound_port_returns_false_2() {
        // Bind a port ourselves, then verify is_free correctly returns false.
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mgr = TcpPortManager::new();
        assert!(!mgr.is_free(port).await.unwrap(), "bound port should not be free");
        drop(listener);
    }

    #[tokio::test]
    async fn is_free_on_bound_port_returns_false() {
        // Ocupa un puerto y comprueba que is_free lo detecta
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mgr = TcpPortManager::new();
        assert!(!mgr.is_free(port).await.unwrap());
        drop(listener);
    }

    #[tokio::test]
    async fn validate_bound_port_fails() {
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mgr = TcpPortManager::new();
        assert!(mgr.validate(port).await.is_err());
        drop(listener);
    }

    #[tokio::test]
    async fn find_free_skips_bound_port() {
        let listener = TcpListener::bind("0.0.0.0:0").unwrap();
        let occupied = listener.local_addr().unwrap().port();
        let mgr = TcpPortManager::new();
        let free = mgr.find_free_from(occupied).await.unwrap();
        assert_ne!(free, occupied);
        drop(listener);
    }
}
