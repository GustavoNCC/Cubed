use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::BackupRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct DeleteBackup {
    backups: Arc<dyn BackupRepository>,
}

impl DeleteBackup {
    pub fn new(backups: Arc<dyn BackupRepository>) -> Self {
        Self { backups }
    }

    pub async fn execute(&self, backup_id: Uuid) -> ApplicationResult<String> {
        let backup = self.backups.find_by_id(backup_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Backup {} no encontrado", backup_id))
        })?;
        let path = backup.path().to_string();
        self.backups.delete(backup_id).await?;
        Ok(path)
    }
}
