use std::sync::Arc;
use uuid::Uuid;
use cubed_domain::entities::ModEntry;
use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::{ModRepository, ServerRepository};

pub struct AddModInput {
    pub server_id: Uuid,
    pub file_name: String,
    /// Ruta absoluta al .jar ya copiado en mods/.
    pub path: String,
}

pub struct AddMod {
    servers: Arc<dyn ServerRepository>,
    mods:    Arc<dyn ModRepository>,
}

impl AddMod {
    pub fn new(servers: Arc<dyn ServerRepository>, mods: Arc<dyn ModRepository>) -> Self {
        Self { servers, mods }
    }

    pub async fn execute(&self, input: AddModInput) -> ApplicationResult<ModEntry> {
        // Verify .jar extension
        if !input.file_name.ends_with(".jar") {
            return Err(ApplicationError::Infrastructure(
                format!("'{}' no es un archivo .jar válido", input.file_name),
            ));
        }

        // Verify server exists
        self.servers
            .find_by_id(input.server_id)
            .await?
            .ok_or_else(|| ApplicationError::Infrastructure(
                format!("Servidor {} no encontrado", input.server_id),
            ))?;

        let entry = ModEntry::new(input.server_id, input.file_name, input.path);
        self.mods.save(&entry).await?;
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use async_trait::async_trait;
    use cubed_domain::entities::{Server, ServerSoftware};
    use cubed_domain::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};
    use crate::ports::ServerRepository;

    struct FakeSrvRepo(Server);
    #[async_trait]
    impl ServerRepository for FakeSrvRepo {
        async fn save(&self, _: &Server) -> ApplicationResult<()> { Ok(()) }
        async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Server>> {
            if id == self.0.id() { Ok(Some(self.0.clone())) } else { Ok(None) }
        }
        async fn find_all(&self) -> ApplicationResult<Vec<Server>> { Ok(vec![self.0.clone()]) }
        async fn delete(&self, _: Uuid) -> ApplicationResult<()> { Ok(()) }
        async fn port_in_use(&self, _: u16) -> ApplicationResult<bool> { Ok(false) }
    }

    struct FakeModRepo(Mutex<HashMap<Uuid, ModEntry>>);
    impl FakeModRepo { fn new() -> Arc<Self> { Arc::new(Self(Mutex::new(HashMap::new()))) } }
    #[async_trait]
    impl ModRepository for FakeModRepo {
        async fn save(&self, e: &ModEntry) -> ApplicationResult<()> {
            self.0.lock().unwrap().insert(e.id(), e.clone()); Ok(())
        }
        async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<ModEntry>> {
            Ok(self.0.lock().unwrap().get(&id).cloned())
        }
        async fn find_by_server(&self, sid: Uuid) -> ApplicationResult<Vec<ModEntry>> {
            Ok(self.0.lock().unwrap().values().filter(|e| e.server_id() == sid).cloned().collect())
        }
        async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
            self.0.lock().unwrap().remove(&id); Ok(())
        }
    }

    fn make_server() -> Server {
        Server::new(
            ServerName::new("test").unwrap(),
            ServerVersion::new("1.21").unwrap(),
            ServerSoftware::Paper,
            ServerPort::new(25565).unwrap(),
            JavaPath::new("/usr/bin/java").unwrap(),
        )
    }

    #[tokio::test]
    async fn add_valid_jar() {
        let srv = make_server();
        let sid = srv.id();
        let uc = AddMod::new(Arc::new(FakeSrvRepo(srv)), FakeModRepo::new());
        let entry = uc.execute(AddModInput {
            server_id: sid,
            file_name: "lithium.jar".into(),
            path: "/srv/mods/lithium.jar".into(),
        }).await.unwrap();
        assert_eq!(entry.file_name(), "lithium.jar");
    }

    #[tokio::test]
    async fn rejects_non_jar() {
        let srv = make_server();
        let sid = srv.id();
        let uc = AddMod::new(Arc::new(FakeSrvRepo(srv)), FakeModRepo::new());
        let result = uc.execute(AddModInput {
            server_id: sid,
            file_name: "lithium.zip".into(),
            path: "/srv/mods/lithium.zip".into(),
        }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_unknown_server() {
        let srv = make_server();
        let uc = AddMod::new(Arc::new(FakeSrvRepo(srv)), FakeModRepo::new());
        let result = uc.execute(AddModInput {
            server_id: Uuid::new_v4(),
            file_name: "lithium.jar".into(),
            path: "/srv/mods/lithium.jar".into(),
        }).await;
        assert!(result.is_err());
    }
}
