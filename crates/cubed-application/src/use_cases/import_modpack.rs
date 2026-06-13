use std::sync::Arc;
use uuid::Uuid;
use cubed_domain::entities::{Modpack, ModpackFormat};
use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::{ModpackRepository, ServerRepository};

pub struct ImportModpackInput {
    pub server_id:   Uuid,
    pub source_path: String,
}

pub struct ImportModpack {
    servers:  Arc<dyn ServerRepository>,
    modpacks: Arc<dyn ModpackRepository>,
}

impl ImportModpack {
    pub fn new(servers: Arc<dyn ServerRepository>, modpacks: Arc<dyn ModpackRepository>) -> Self {
        Self { servers, modpacks }
    }

    pub async fn execute(&self, input: ImportModpackInput) -> ApplicationResult<Modpack> {
        // Verify server
        self.servers
            .find_by_id(input.server_id)
            .await?
            .ok_or_else(|| ApplicationError::Infrastructure(
                format!("Servidor {} no encontrado", input.server_id),
            ))?;

        let format = detect_format(&input.source_path)?;
        let name = std::path::Path::new(&input.source_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("modpack")
            .to_string();

        let modpack = Modpack::new(input.server_id, name, format, input.source_path);
        self.modpacks.save(&modpack).await?;
        Ok(modpack)
    }
}

fn detect_format(path: &str) -> ApplicationResult<ModpackFormat> {
    if path.ends_with(".mrpack") { return Ok(ModpackFormat::Mrpack); }
    if path.ends_with(".zip")    { return Ok(ModpackFormat::Zip); }
    Err(ApplicationError::Infrastructure(
        format!("Formato no soportado: '{}'. Use .mrpack o .zip", path),
    ))
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

    struct FakePackRepo(Mutex<HashMap<Uuid, Modpack>>);
    impl FakePackRepo { fn new() -> Arc<Self> { Arc::new(Self(Mutex::new(HashMap::new()))) } }
    #[async_trait]
    impl ModpackRepository for FakePackRepo {
        async fn save(&self, m: &Modpack) -> ApplicationResult<()> {
            self.0.lock().unwrap().insert(m.id(), m.clone()); Ok(())
        }
        async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Modpack>> {
            Ok(self.0.lock().unwrap().get(&id).cloned())
        }
        async fn find_by_server(&self, sid: Uuid) -> ApplicationResult<Vec<Modpack>> {
            Ok(self.0.lock().unwrap().values().filter(|m| m.server_id() == sid).cloned().collect())
        }
        async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
            self.0.lock().unwrap().remove(&id); Ok(())
        }
    }

    fn make_server() -> Server {
        Server::new(
            ServerName::new("srv").unwrap(),
            ServerVersion::new("1.21").unwrap(),
            ServerSoftware::Paper,
            ServerPort::new(25565).unwrap(),
            JavaPath::new("/usr/bin/java").unwrap(),
        )
    }

    #[tokio::test]
    async fn import_mrpack_detected() {
        let srv = make_server();
        let sid = srv.id();
        let uc = ImportModpack::new(Arc::new(FakeSrvRepo(srv)), FakePackRepo::new());
        let mp = uc.execute(ImportModpackInput {
            server_id: sid,
            source_path: "/packs/fabric-1.21.mrpack".into(),
        }).await.unwrap();
        assert_eq!(mp.format(), &ModpackFormat::Mrpack);
    }

    #[tokio::test]
    async fn import_zip_detected() {
        let srv = make_server();
        let sid = srv.id();
        let uc = ImportModpack::new(Arc::new(FakeSrvRepo(srv)), FakePackRepo::new());
        let mp = uc.execute(ImportModpackInput {
            server_id: sid,
            source_path: "/packs/pack.zip".into(),
        }).await.unwrap();
        assert_eq!(mp.format(), &ModpackFormat::Zip);
    }

    #[tokio::test]
    async fn import_unsupported_format_fails() {
        let srv = make_server();
        let sid = srv.id();
        let uc = ImportModpack::new(Arc::new(FakeSrvRepo(srv)), FakePackRepo::new());
        let result = uc.execute(ImportModpackInput {
            server_id: sid,
            source_path: "/packs/pack.tar.gz".into(),
        }).await;
        assert!(result.is_err());
    }
}
