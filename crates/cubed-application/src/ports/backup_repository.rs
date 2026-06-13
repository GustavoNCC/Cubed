use async_trait::async_trait;
use uuid::Uuid;
use cubed_domain::entities::Backup;
use crate::error::ApplicationResult;

#[async_trait]
pub trait BackupRepository: Send + Sync {
    async fn save(&self, backup: &Backup) -> ApplicationResult<()>;
    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Backup>>;
    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<Backup>>;
    async fn delete(&self, id: Uuid) -> ApplicationResult<()>;
}
