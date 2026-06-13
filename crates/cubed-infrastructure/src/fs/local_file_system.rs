use async_trait::async_trait;
use std::path::Path;
use tokio::fs;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::FileSystemManager;

/// Directorios globales de Cubed bajo la raíz configurada.
const GLOBAL_DIRS: &[&str] = &[
    "servers",
    "backups",
    "downloads",
    "temp",
    "config",
    "logs",
];

/// Subdirectorios que se crean dentro de cada servidor.
const SERVER_SUBDIRS: &[&str] = &["mods", "world", "config", "logs"];

/// Implementación real sobre el sistema de archivos local.
pub struct LocalFileSystem {
    /// Directorio raíz de Cubed (e.g. "/home/cubed").
    cubed_root: String,
}

impl LocalFileSystem {
    pub fn new(cubed_root: impl Into<String>) -> Self {
        Self { cubed_root: cubed_root.into() }
    }
}

#[async_trait]
impl FileSystemManager for LocalFileSystem {
    async fn init_cubed_dirs(&self) -> ApplicationResult<()> {
        for dir in GLOBAL_DIRS {
            let path = format!("{}/{}", self.cubed_root, dir);
            create_dir_all(&path).await?;
        }
        Ok(())
    }

    async fn init_server_dirs(&self, servers_dir: &str, server_name: &str) -> ApplicationResult<()> {
        let base = format!("{}/{}", servers_dir, server_name);
        create_dir_all(&base).await?;
        for sub in SERVER_SUBDIRS {
            create_dir_all(&format!("{}/{}", base, sub)).await?;
        }
        Ok(())
    }

    async fn delete_server_dir(&self, servers_dir: &str, server_name: &str) -> ApplicationResult<()> {
        let path = format!("{}/{}", servers_dir, server_name);
        if Path::new(&path).exists() {
            fs::remove_dir_all(&path)
                .await
                .map_err(|e| ApplicationError::Infrastructure(
                    format!("No se pudo eliminar el directorio '{}': {}", path, e),
                ))?;
        }
        Ok(())
    }

    fn server_dir(&self, servers_dir: &str, server_name: &str) -> String {
        format!("{}/{}", servers_dir, server_name)
    }

    async fn ensure_writable(&self, path: &str) -> ApplicationResult<()> {
        let meta = fs::metadata(path)
            .await
            .map_err(|e| ApplicationError::Infrastructure(
                format!("No se puede acceder a '{}': {}", path, e),
            ))?;

        if meta.permissions().readonly() {
            return Err(ApplicationError::Infrastructure(
                format!("El directorio '{}' no tiene permisos de escritura", path),
            ));
        }
        Ok(())
    }
}

async fn create_dir_all(path: &str) -> ApplicationResult<()> {
    fs::create_dir_all(path)
        .await
        .map_err(|e| ApplicationError::Infrastructure(
            format!("No se pudo crear el directorio '{}': {}", path, e),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_fs() -> (LocalFileSystem, TempDir) {
        let tmp = TempDir::new().unwrap();
        let fs = LocalFileSystem::new(tmp.path().to_str().unwrap());
        (fs, tmp)
    }

    #[tokio::test]
    async fn init_cubed_dirs_creates_all_global_dirs() {
        let (fs, tmp) = temp_fs();
        fs.init_cubed_dirs().await.unwrap();
        for dir in GLOBAL_DIRS {
            assert!(tmp.path().join(dir).exists(), "falta directorio: {}", dir);
        }
    }

    #[tokio::test]
    async fn init_server_dirs_creates_subdirs() {
        let (fs, tmp) = temp_fs();
        let servers_dir = tmp.path().join("servers");
        tokio::fs::create_dir_all(&servers_dir).await.unwrap();

        fs.init_server_dirs(servers_dir.to_str().unwrap(), "survival").await.unwrap();

        for sub in SERVER_SUBDIRS {
            assert!(servers_dir.join("survival").join(sub).exists(), "falta subdir: {}", sub);
        }
    }

    #[tokio::test]
    async fn delete_server_dir_removes_tree() {
        let (fs, tmp) = temp_fs();
        let servers_dir = tmp.path().join("servers");
        let srv_dir = servers_dir.join("survival");
        tokio::fs::create_dir_all(&srv_dir).await.unwrap();

        fs.delete_server_dir(servers_dir.to_str().unwrap(), "survival").await.unwrap();

        assert!(!srv_dir.exists());
    }

    #[tokio::test]
    async fn delete_nonexistent_dir_is_ok() {
        let (fs, tmp) = temp_fs();
        let servers_dir = tmp.path().join("servers");
        tokio::fs::create_dir_all(&servers_dir).await.unwrap();

        // No debe fallar si el directorio no existe
        let result = fs.delete_server_dir(servers_dir.to_str().unwrap(), "ghost").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn server_dir_returns_correct_path() {
        let (fs, _tmp) = temp_fs();
        assert_eq!(
            fs.server_dir("/cubed/servers", "survival"),
            "/cubed/servers/survival"
        );
    }
}
