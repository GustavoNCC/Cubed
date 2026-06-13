use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use cubed_application::error::ApplicationResult;
use cubed_application::ports::BackupRepository;
use cubed_domain::entities::Backup;

pub struct InMemoryBackupRepo {
    store: Arc<RwLock<HashMap<Uuid, Backup>>>,
}

impl InMemoryBackupRepo {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

impl Default for InMemoryBackupRepo {
    fn default() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl BackupRepository for InMemoryBackupRepo {
    async fn save(&self, backup: &Backup) -> ApplicationResult<()> {
        self.store.write().await.insert(backup.id(), backup.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Backup>> {
        Ok(self.store.read().await.get(&id).cloned())
    }

    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<Backup>> {
        Ok(self
            .store
            .read()
            .await
            .values()
            .filter(|b| b.server_id() == server_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        self.store.write().await.remove(&id);
        Ok(())
    }
}
