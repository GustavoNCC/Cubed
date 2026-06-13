use std::sync::Arc;
use uuid::Uuid;
use cubed_domain::entities::Backup;
use crate::error::ApplicationResult;
use crate::ports::BackupRepository;

pub struct ListBackups {
    backups: Arc<dyn BackupRepository>,
}

impl ListBackups {
    pub fn new(backups: Arc<dyn BackupRepository>) -> Self {
        Self { backups }
    }

    pub async fn execute(&self, server_id: Uuid) -> ApplicationResult<Vec<Backup>> {
        let mut list = self.backups.find_by_server(server_id).await?;
        list.sort_by(|a, b| b.created_at().cmp(&a.created_at()));
        Ok(list)
    }
}
