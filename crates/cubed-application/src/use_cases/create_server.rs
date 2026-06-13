use std::sync::Arc;
use cubed_domain::entities::{Server, ServerSoftware};
use cubed_domain::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};
use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::{FileSystemManager, ServerRepository};

pub struct CreateServerInput {
    pub name: String,
    pub version: String,
    pub software: ServerSoftware,
    pub port: u16,
    pub java_path: String,
    /// Directorio raíz donde se almacenan los servidores (de Settings).
    pub servers_dir: String,
}

pub struct CreateServer {
    repo: Arc<dyn ServerRepository>,
    fs: Arc<dyn FileSystemManager>,
}

impl CreateServer {
    pub fn new(repo: Arc<dyn ServerRepository>, fs: Arc<dyn FileSystemManager>) -> Self {
        Self { repo, fs }
    }

    pub async fn execute(&self, input: CreateServerInput) -> ApplicationResult<Server> {
        let name = ServerName::new(&input.name)?;
        let version = ServerVersion::new(&input.version)?;
        let port = ServerPort::new(input.port)?;
        let java_path = JavaPath::new(&input.java_path)?;

        if self.repo.port_in_use(port.value()).await? {
            return Err(ApplicationError::Infrastructure(
                format!("El puerto {} ya está en uso", port),
            ));
        }

        let server = Server::new(name, version, input.software, port, java_path);
        self.repo.save(&server).await?;
        self.fs.init_server_dirs(&input.servers_dir, server.name().as_str()).await?;
        Ok(server)
    }
}
