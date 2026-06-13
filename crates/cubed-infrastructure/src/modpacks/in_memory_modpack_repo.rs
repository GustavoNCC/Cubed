use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use cubed_application::error::ApplicationResult;
use cubed_application::ports::ModpackRepository;
use cubed_domain::entities::Modpack;

pub struct InMemoryModpackRepo {
    store: Arc<RwLock<HashMap<Uuid, Modpack>>>,
}

impl InMemoryModpackRepo {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { store: Arc::new(RwLock::new(HashMap::new())) })
    }
}

impl Default for InMemoryModpackRepo {
    fn default() -> Self {
        Self { store: Arc::new(RwLock::new(HashMap::new())) }
    }
}

#[async_trait]
impl ModpackRepository for InMemoryModpackRepo {
    async fn save(&self, modpack: &Modpack) -> ApplicationResult<()> {
        self.store.write().await.insert(modpack.id(), modpack.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Modpack>> {
        Ok(self.store.read().await.get(&id).cloned())
    }

    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<Modpack>> {
        Ok(self.store.read().await
            .values()
            .filter(|m| m.server_id() == server_id)
            .cloned()
            .collect())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        self.store.write().await.remove(&id);
        Ok(())
    }
}
