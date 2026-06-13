use crate::error::ApplicationResult;
use async_trait::async_trait;
use cubed_domain::entities::ModEntry;
use uuid::Uuid;

#[async_trait]
pub trait ModRepository: Send + Sync {
    async fn save(&self, entry: &ModEntry) -> ApplicationResult<()>;
    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<ModEntry>>;
    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<ModEntry>>;
    async fn delete(&self, id: Uuid) -> ApplicationResult<()>;
}
