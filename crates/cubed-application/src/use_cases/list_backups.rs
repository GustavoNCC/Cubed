use crate::error::ApplicationResult;
use crate::ports::BackupRepository;
use cubed_domain::entities::Backup;
use std::cmp::Reverse;
use std::sync::Arc;
use uuid::Uuid;

pub struct ListBackups {
    backups: Arc<dyn BackupRepository>,
}

impl ListBackups {
    pub fn new(backups: Arc<dyn BackupRepository>) -> Self {
        Self { backups }
    }

    pub async fn execute(&self, server_id: Uuid) -> ApplicationResult<Vec<Backup>> {
        let mut list = self.backups.find_by_server(server_id).await?;
        list.sort_by_key(|b| Reverse(b.created_at()));
        Ok(list)
    }
}
