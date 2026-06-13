use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use cubed_application::error::ApplicationResult;
use cubed_application::ports::ModRepository;
use cubed_domain::entities::ModEntry;

pub struct InMemoryModRepo {
    store: Arc<RwLock<HashMap<Uuid, ModEntry>>>,
}

impl InMemoryModRepo {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { store: Arc::new(RwLock::new(HashMap::new())) })
    }
}

impl Default for InMemoryModRepo {
    fn default() -> Self {
        Self { store: Arc::new(RwLock::new(HashMap::new())) }
    }
}

#[async_trait]
impl ModRepository for InMemoryModRepo {
    async fn save(&self, entry: &ModEntry) -> ApplicationResult<()> {
        self.store.write().await.insert(entry.id(), entry.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<ModEntry>> {
        Ok(self.store.read().await.get(&id).cloned())
    }

    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<ModEntry>> {
        Ok(self.store.read().await
            .values()
            .filter(|e| e.server_id() == server_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        self.store.write().await.remove(&id);
        Ok(())
    }
}
