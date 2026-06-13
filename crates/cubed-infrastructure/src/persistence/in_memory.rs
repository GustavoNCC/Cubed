use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

use cubed_application::error::ApplicationResult;
use cubed_application::ports::ServerRepository;
use cubed_domain::entities::Server;

/// Repositorio de servidores en memoria — solo para tests y modo dev.
pub struct InMemoryServerRepo {
    store: Arc<RwLock<HashMap<Uuid, Server>>>,
}

impl InMemoryServerRepo {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { store: Arc::new(RwLock::new(HashMap::new())) })
    }
}

impl Default for InMemoryServerRepo {
    fn default() -> Self {
        Self { store: Arc::new(RwLock::new(HashMap::new())) }
    }
}

#[async_trait]
impl ServerRepository for InMemoryServerRepo {
    async fn save(&self, server: &Server) -> ApplicationResult<()> {
        self.store.write().await.insert(server.id(), server.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Server>> {
        Ok(self.store.read().await.get(&id).cloned())
    }

    async fn find_all(&self) -> ApplicationResult<Vec<Server>> {
        Ok(self.store.read().await.values().cloned().collect())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        self.store.write().await.remove(&id);
        Ok(())
    }

    async fn port_in_use(&self, port: u16) -> ApplicationResult<bool> {
        Ok(self.store.read().await.values().any(|s| s.port().value() == port))
    }
}
