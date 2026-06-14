use async_trait::async_trait;
use std::path::PathBuf;
use tokio::sync::Mutex;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::ModpackRepository;
use cubed_domain::entities::Modpack;

pub struct JsonModpackRepo {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl JsonModpackRepo {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            write_lock: Mutex::new(()),
        }
    }

    fn load(&self) -> ApplicationResult<Vec<Modpack>> {
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

    fn save_all(&self, modpacks: &[Modpack]) -> ApplicationResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ApplicationError::Infrastructure(format!(
                    "No se pudo crear {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let json = serde_json::to_string_pretty(modpacks)
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
impl ModpackRepository for JsonModpackRepo {
    async fn save(&self, modpack: &Modpack) -> ApplicationResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut modpacks = self.load()?;
        modpacks.retain(|m| m.id() != modpack.id());
        modpacks.push(modpack.clone());
        self.save_all(&modpacks)
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Modpack>> {
        Ok(self.load()?.into_iter().find(|m| m.id() == id))
    }

    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<Modpack>> {
        Ok(self
            .load()?
            .into_iter()
            .filter(|m| m.server_id() == server_id)
            .collect())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        let _guard = self.write_lock.lock().await;
        let mut modpacks = self.load()?;
        modpacks.retain(|m| m.id() != id);
        self.save_all(&modpacks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cubed_domain::entities::ModpackFormat;
    use tempfile::tempdir;

    #[tokio::test]
    async fn modpack_records_persist_and_delete() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("modpacks.json");
        let repo = JsonModpackRepo::new(path.clone());
        let server_id = Uuid::new_v4();
        let modpack = Modpack::reconstitute(
            Uuid::new_v4(),
            server_id,
            "Test Pack".into(),
            ModpackFormat::Mrpack,
            "/downloads/test.mrpack".into(),
        );

        repo.save(&modpack).await.unwrap();
        let reloaded = JsonModpackRepo::new(path);
        assert!(reloaded.find_by_id(modpack.id()).await.unwrap().is_some());
        assert_eq!(
            reloaded.find_by_server(server_id).await.unwrap()[0].name(),
            "Test Pack"
        );

        reloaded.delete(modpack.id()).await.unwrap();
        assert!(reloaded.find_by_id(modpack.id()).await.unwrap().is_none());
    }
}
