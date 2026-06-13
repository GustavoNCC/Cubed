use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::{ModRepository, ServerRepository};
use cubed_application::use_cases::{AddMod, AddModInput};
use cubed_domain::entities::ModEntry;

/// Gestiona la instalación física de mods (.jar) en el directorio mods/ de un servidor.
pub struct FileModManager {
    servers: Arc<dyn ServerRepository>,
    repo: Arc<dyn ModRepository>,
}

impl FileModManager {
    pub fn new(servers: Arc<dyn ServerRepository>, repo: Arc<dyn ModRepository>) -> Arc<Self> {
        Arc::new(Self { servers, repo })
    }

    /// Valida que el archivo sea un JAR (cabecera PK) sin necesidad de copiarlo.
    pub async fn validate_jar(source_path: &str) -> ApplicationResult<()> {
        let bytes = fs::read(source_path).await.map_err(|e| {
            ApplicationError::Infrastructure(format!("No se pudo leer '{}': {}", source_path, e))
        })?;

        // JAR/ZIP magic bytes: PK\x03\x04
        if bytes.len() < 4 || &bytes[0..4] != b"PK\x03\x04" {
            return Err(ApplicationError::Infrastructure(format!(
                "'{}' no es un archivo JAR válido (cabecera inválida)",
                source_path
            )));
        }
        Ok(())
    }

    /// Copia el .jar al directorio mods/ del servidor, valida su formato y registra el mod.
    pub async fn install_mod(
        &self,
        server_id: Uuid,
        source_path: &str,
        mods_dir: &str,
    ) -> ApplicationResult<ModEntry> {
        let file_name = Path::new(source_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                ApplicationError::Infrastructure(format!("Ruta inválida: '{}'", source_path))
            })?
            .to_string();

        if !file_name.ends_with(".jar") {
            return Err(ApplicationError::Infrastructure(format!(
                "'{}' no es un archivo .jar",
                file_name
            )));
        }

        // Validate JAR magic bytes
        Self::validate_jar(source_path).await?;

        // Copy to mods dir
        fs::create_dir_all(mods_dir).await.map_err(|e| {
            ApplicationError::Infrastructure(format!("No se pudo crear mods/: {}", e))
        })?;

        let dest = format!("{}/{}", mods_dir, file_name);
        fs::copy(source_path, &dest)
            .await
            .map_err(|e| ApplicationError::Infrastructure(format!("Error copiando mod: {}", e)))?;

        let uc = AddMod::new(self.servers.clone(), self.repo.clone());
        uc.execute(AddModInput {
            server_id,
            file_name,
            path: dest,
        })
        .await
    }

    /// Lista los mods registrados para un servidor.
    pub async fn list_mods(&self, server_id: Uuid) -> ApplicationResult<Vec<ModEntry>> {
        let mut list = self.repo.find_by_server(server_id).await?;
        list.sort_by(|a, b| a.file_name().cmp(b.file_name()));
        Ok(list)
    }

    /// Elimina el mod del registro y borra el .jar del disco.
    pub async fn remove_mod(&self, mod_id: Uuid) -> ApplicationResult<()> {
        let entry = self.repo.find_by_id(mod_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Mod {} no encontrado", mod_id))
        })?;

        // Best-effort file deletion
        let _ = fs::remove_file(entry.path()).await;
        self.repo.delete(mod_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::InMemoryModRepo;
    use crate::persistence::InMemoryServerRepo;
    use cubed_domain::entities::Server;
    use cubed_domain::entities::ServerSoftware;
    use cubed_domain::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};

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
    async fn validate_jar_rejects_plain_text() {
        use std::io::Write;
        use tempfile::NamedTempFile;
        let mut f = NamedTempFile::with_suffix(".jar").unwrap();
        f.write_all(b"not a jar").unwrap();
        let result = FileModManager::validate_jar(f.path().to_str().unwrap()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn install_rejects_non_jar_extension() {
        let srv = make_server();
        let sid = srv.id();
        let servers = InMemoryServerRepo::new();
        servers.save(&srv).await.unwrap();
        let mods = InMemoryModRepo::new();
        let mgr = FileModManager::new(servers, mods);
        let result = mgr.install_mod(sid, "/some/file.zip", "/tmp/mods").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_and_remove_mod() {
        let srv = make_server();
        let sid = srv.id();
        let servers = InMemoryServerRepo::new();
        servers.save(&srv).await.unwrap();
        let repo = InMemoryModRepo::new();
        // Manually add a mod entry
        let entry = ModEntry::new(sid, "test.jar".into(), "/tmp/test.jar".into());
        repo.save(&entry).await.unwrap();

        let mgr = FileModManager::new(servers, repo.clone());
        let list = mgr.list_mods(sid).await.unwrap();
        assert_eq!(list.len(), 1);

        mgr.remove_mod(entry.id()).await.unwrap();
        let list2 = mgr.list_mods(sid).await.unwrap();
        assert!(list2.is_empty());
    }
}
