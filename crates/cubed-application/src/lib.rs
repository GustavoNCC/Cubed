//! # cubed-application
//!
//! Capa de Aplicación (Clean Architecture).
//!
//! Orquesta los casos de uso de Cubed y define los puertos (traits) que la
//! infraestructura debe implementar. Depende SOLO de la capa de dominio.

pub mod error;
pub mod events;
pub mod ports;
pub mod use_cases;

pub use events::CubedEvent;

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use uuid::Uuid;
    use async_trait::async_trait;
    use cubed_domain::entities::{Server, ServerSoftware};
    use crate::error::ApplicationResult;
    use crate::ports::{FileSystemManager, ServerRepository};
    use crate::use_cases::{CreateServer, CreateServerInput};

    // --- Stubs ---

    struct InMemoryServerRepo {
        inner: std::sync::Mutex<Vec<Server>>,
    }

    impl InMemoryServerRepo {
        fn new() -> Arc<Self> {
            Arc::new(Self { inner: std::sync::Mutex::new(Vec::new()) })
        }
    }

    #[async_trait]
    impl ServerRepository for InMemoryServerRepo {
        async fn save(&self, server: &Server) -> ApplicationResult<()> {
            let mut data = self.inner.lock().unwrap();
            data.retain(|s| s.id() != server.id());
            data.push(server.clone());
            Ok(())
        }
        async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Server>> {
            Ok(self.inner.lock().unwrap().iter().find(|s| s.id() == id).cloned())
        }
        async fn find_all(&self) -> ApplicationResult<Vec<Server>> {
            Ok(self.inner.lock().unwrap().clone())
        }
        async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
            self.inner.lock().unwrap().retain(|s| s.id() != id);
            Ok(())
        }
        async fn port_in_use(&self, port: u16) -> ApplicationResult<bool> {
            Ok(self.inner.lock().unwrap().iter().any(|s| s.port().value() == port))
        }
    }

    struct NoopFs;

    #[async_trait]
    impl FileSystemManager for NoopFs {
        async fn init_cubed_dirs(&self) -> ApplicationResult<()> { Ok(()) }
        async fn init_server_dirs(&self, _: &str, _: &str) -> ApplicationResult<()> { Ok(()) }
        async fn delete_server_dir(&self, _: &str, _: &str) -> ApplicationResult<()> { Ok(()) }
        fn server_dir(&self, servers_dir: &str, name: &str) -> String {
            format!("{}/{}", servers_dir, name)
        }
        async fn ensure_writable(&self, _: &str) -> ApplicationResult<()> { Ok(()) }
    }

    fn make_fs() -> Arc<NoopFs> { Arc::new(NoopFs) }

    // --- Tests ---

    #[tokio::test]
    async fn create_server_persists() {
        let repo = InMemoryServerRepo::new();
        let uc = CreateServer::new(repo.clone(), make_fs());

        let server = uc.execute(CreateServerInput {
            name: "survival".into(),
            version: "1.21.4".into(),
            software: ServerSoftware::Paper,
            port: 25565,
            java_path: "/usr/bin/java".into(),
            servers_dir: "/cubed/servers".into(),
        }).await.unwrap();

        assert!(repo.find_by_id(server.id()).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn duplicate_port_rejected() {
        let repo = InMemoryServerRepo::new();
        let uc = CreateServer::new(repo.clone(), make_fs());

        uc.execute(CreateServerInput {
            name: "srv1".into(),
            version: "1.21.4".into(),
            software: ServerSoftware::Paper,
            port: 25565,
            java_path: "/usr/bin/java".into(),
            servers_dir: "/cubed/servers".into(),
        }).await.unwrap();

        let result = uc.execute(CreateServerInput {
            name: "srv2".into(),
            version: "1.21.4".into(),
            software: ServerSoftware::Purpur,
            port: 25565,
            java_path: "/usr/bin/java".into(),
            servers_dir: "/cubed/servers".into(),
        }).await;

        assert!(result.is_err());
    }
}
