use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use cubed_domain::entities::Server;
use cubed_application::error::ApplicationResult;
use cubed_application::ports::ServerRepository;

pub struct InMemoryServerRepo {
    data: Arc<RwLock<Vec<Server>>>,
}

impl InMemoryServerRepo {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { data: Arc::new(RwLock::new(Vec::new())) })
    }
}

#[async_trait]
impl ServerRepository for InMemoryServerRepo {
    async fn save(&self, server: &Server) -> ApplicationResult<()> {
        let mut data = self.data.write().await;
        data.retain(|s| s.id() != server.id());
        data.push(server.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Server>> {
        Ok(self.data.read().await.iter().find(|s| s.id() == id).cloned())
    }

    async fn find_all(&self) -> ApplicationResult<Vec<Server>> {
        Ok(self.data.read().await.clone())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        self.data.write().await.retain(|s| s.id() != id);
        Ok(())
    }

    async fn port_in_use(&self, port: u16) -> ApplicationResult<bool> {
        Ok(self.data.read().await.iter().any(|s| s.port().value() == port))
    }
}
