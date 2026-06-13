use crate::error::ApplicationResult;
use async_trait::async_trait;
use cubed_domain::entities::Server;
use uuid::Uuid;

/// Puerto de persistencia para servidores. La infraestructura lo implementa.
#[async_trait]
pub trait ServerRepository: Send + Sync {
    async fn save(&self, server: &Server) -> ApplicationResult<()>;
    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Server>>;
    async fn find_all(&self) -> ApplicationResult<Vec<Server>>;
    async fn delete(&self, id: Uuid) -> ApplicationResult<()>;
    async fn port_in_use(&self, port: u16) -> ApplicationResult<bool>;
}
