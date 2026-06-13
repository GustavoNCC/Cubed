use async_trait::async_trait;
use uuid::Uuid;
use cubed_domain::entities::Modpack;
use crate::error::ApplicationResult;

#[async_trait]
pub trait ModpackRepository: Send + Sync {
    async fn save(&self, modpack: &Modpack) -> ApplicationResult<()>;
    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Modpack>>;
    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<Modpack>>;
    async fn delete(&self, id: Uuid) -> ApplicationResult<()>;
}
