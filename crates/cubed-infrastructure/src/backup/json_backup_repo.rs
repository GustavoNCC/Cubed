use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::Mutex;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::BackupRepository;
use cubed_domain::entities::Backup;

pub struct JsonBackupRepo {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl JsonBackupRepo {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            write_lock: Mutex::new(()),
        }
    }

    fn load(&self) -> ApplicationResult<Vec<Backup>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let raw = std::fs::read_to_string(&self.path).map_err(|e| {
            ApplicationError::Infrastructure(format!("Error leyendo {}: {e}", self.path.display()))
        })?;
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&raw).map_err(|e| {
            ApplicationError::Infrastructure(format!(
                "JSON corrupto en {}: {e}",
                self.path.display()
            ))
        })
    }

    fn save_all(&self, backups: &[Backup]) -> ApplicationResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ApplicationError::Infrastructure(format!(
                    "No se pudo crear {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let json = serde_json::to_string_pretty(backups)
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, json)
            .map_err(|e| ApplicationError::Infrastructure(format!("Error escribiendo tmp: {e}")))?;
        std::fs::rename(&tmp, &self.path)
            .map_err(|e| ApplicationError::Infrastructure(format!("Error renombrando tmp: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl BackupRepository for JsonBackupRepo {
    async fn save(&self, backup: &Backup) -> ApplicationResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut backups = self.load()?;
        backups.retain(|b| b.id() != backup.id());
        backups.push(backup.clone());
        self.save_all(&backups)
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Backup>> {
        Ok(self.load()?.into_iter().find(|b| b.id() == id))
    }

    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<Backup>> {
        Ok(self
            .load()?
            .into_iter()
            .filter(|b| b.server_id() == server_id)
            .collect())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut backups = self.load()?;
        backups.retain(|b| b.id() != id);
        self.save_all(&backups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn backup_records_persist_and_delete() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("backups.json");
        let repo = JsonBackupRepo::new(path.clone());
        let server_id = Uuid::new_v4();
        let backup = Backup::reconstitute(
            Uuid::new_v4(),
            server_id,
            "/home/cubed/backups/srv.tar.gz".into(),
            42,
            Utc::now(),
        );

        repo.save(&backup).await.unwrap();
        let reloaded = JsonBackupRepo::new(path);
        assert!(reloaded.find_by_id(backup.id()).await.unwrap().is_some());
        assert_eq!(
            reloaded.find_by_server(server_id).await.unwrap()[0].size_bytes(),
            42
        );

        reloaded.delete(backup.id()).await.unwrap();
        assert!(reloaded.find_by_id(backup.id()).await.unwrap().is_none());
    }
}
