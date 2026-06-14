use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::Mutex;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::ModRepository;
use cubed_domain::entities::ModEntry;

pub struct JsonModRepo {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl JsonModRepo {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            write_lock: Mutex::new(()),
        }
    }

    fn load(&self) -> ApplicationResult<Vec<ModEntry>> {
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

    fn save_all(&self, entries: &[ModEntry]) -> ApplicationResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ApplicationError::Infrastructure(format!(
                    "No se pudo crear {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let json = serde_json::to_string_pretty(entries)
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
impl ModRepository for JsonModRepo {
    async fn save(&self, entry: &ModEntry) -> ApplicationResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut entries = self.load()?;
        entries.retain(|e| e.id() != entry.id());
        entries.push(entry.clone());
        self.save_all(&entries)
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<ModEntry>> {
        Ok(self.load()?.into_iter().find(|e| e.id() == id))
    }

    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<ModEntry>> {
        Ok(self
            .load()?
            .into_iter()
            .filter(|e| e.server_id() == server_id)
            .collect())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut entries = self.load()?;
        entries.retain(|e| e.id() != id);
        self.save_all(&entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn mod_records_persist_and_delete() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("mods.json");
        let repo = JsonModRepo::new(path.clone());
        let server_id = Uuid::new_v4();
        let entry = ModEntry::reconstitute(
            Uuid::new_v4(),
            server_id,
            "lithium.jar".into(),
            "/home/cubed/servers/srv/mods/lithium.jar".into(),
        );

        repo.save(&entry).await.unwrap();
        let reloaded = JsonModRepo::new(path);
        assert!(reloaded.find_by_id(entry.id()).await.unwrap().is_some());
        assert_eq!(
            reloaded.find_by_server(server_id).await.unwrap()[0].file_name(),
            "lithium.jar"
        );

        reloaded.delete(entry.id()).await.unwrap();
        assert!(reloaded.find_by_id(entry.id()).await.unwrap().is_none());
    }
}
