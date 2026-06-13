use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::BackupRepository;
use cubed_application::ports::ServerRepository;
use cubed_application::use_cases::{CreateBackup, CreateBackupInput};
use cubed_domain::entities::Backup;

/// Gestiona la creación y restauración de backups en disco (tar.gz).
/// También puede iniciarse un scheduler automático cada N segundos.
pub struct FileBackupManager {
    backups_dir: String,
    servers: Arc<dyn ServerRepository>,
    repo: Arc<dyn BackupRepository>,
    scheduler: Mutex<Option<JoinHandle<()>>>,
}

impl FileBackupManager {
    pub fn new(
        backups_dir: impl Into<String>,
        servers: Arc<dyn ServerRepository>,
        repo: Arc<dyn BackupRepository>,
    ) -> Arc<Self> {
        Arc::new(Self {
            backups_dir: backups_dir.into(),
            servers,
            repo,
            scheduler: Mutex::new(None),
        })
    }

    /// Crea un backup del directorio del servidor como archivo tar.gz.
    /// Devuelve el `Backup` persistido.
    pub async fn backup_server(
        &self,
        server_id: Uuid,
        server_name: &str,
        server_dir: &str,
    ) -> ApplicationResult<Backup> {
        debug!(server_id = %server_id, server_dir, "starting backup");
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.tar.gz", server_name, ts);
        let dest = format!("{}/{}", self.backups_dir, filename);

        // Ensure backups dir exists
        fs::create_dir_all(&self.backups_dir).await.map_err(|e| {
            ApplicationError::Infrastructure(format!(
                "No se pudo crear directorio de backups: {}",
                e
            ))
        })?;

        // tar -czf <dest> -C <parent> <server_name>
        let parent = Path::new(server_dir)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or(server_dir);
        let dir_name = Path::new(server_dir)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(server_dir);

        let status = Command::new("tar")
            .args(["-czf", &dest, "-C", parent, dir_name])
            .status()
            .await
            .map_err(|e| ApplicationError::Infrastructure(format!("tar falló: {}", e)))?;

        if !status.success() {
            return Err(ApplicationError::Infrastructure(format!(
                "tar terminó con código {:?}",
                status.code()
            )));
        }

        let meta = fs::metadata(&dest).await.map_err(|e| {
            ApplicationError::Infrastructure(format!("No se pudo leer metadata del backup: {}", e))
        })?;

        info!(server_id = %server_id, dest, size_bytes = meta.len(), "backup created");
        let uc = CreateBackup::new(self.servers.clone(), self.repo.clone());
        uc.execute(CreateBackupInput {
            server_id,
            path: dest,
            size_bytes: meta.len(),
        })
        .await
    }

    /// Restaura un backup (tar.gz) extrayendo en `restore_dir`.
    pub async fn restore_backup(
        &self,
        backup_id: Uuid,
        restore_dir: &str,
    ) -> ApplicationResult<()> {
        let backup = self.repo.find_by_id(backup_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Backup {} no encontrado", backup_id))
        })?;

        fs::create_dir_all(restore_dir).await.map_err(|e| {
            ApplicationError::Infrastructure(format!(
                "No se pudo crear directorio de restauración: {}",
                e
            ))
        })?;

        let status = Command::new("tar")
            .args(["-xzf", backup.path(), "-C", restore_dir])
            .status()
            .await
            .map_err(|e| ApplicationError::Infrastructure(format!("tar restore falló: {}", e)))?;

        if !status.success() {
            warn!(backup_id = %backup_id, "tar restore failed");
            return Err(ApplicationError::Infrastructure(format!(
                "tar restore terminó con código {:?}",
                status.code()
            )));
        }
        info!(backup_id = %backup_id, restore_dir, "backup restored");
        Ok(())
    }

    /// Inicia el scheduler automático.
    /// Ejecuta `callback` cada `interval_secs` segundos mientras el handle esté vivo.
    /// El callback recibe `(server_id, server_name, server_dir)`.
    pub async fn start_scheduler<F, Fut>(
        self: &Arc<Self>,
        interval_secs: u64,
        servers_snapshot: Vec<(Uuid, String, String)>,
        callback: F,
    ) where
        F: Fn(Uuid, String, String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let cb = Arc::new(callback);
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
                for (id, name, dir) in &servers_snapshot {
                    cb(*id, name.clone(), dir.clone()).await;
                }
            }
        });
        *self.scheduler.lock().await = Some(handle);
    }

    /// Detiene el scheduler automático.
    pub async fn stop_scheduler(&self) {
        if let Some(h) = self.scheduler.lock().await.take() {
            h.abort();
        }
    }

    /// Detiene el scheduler anterior (si existe) y arranca uno nuevo que
    /// hace backup de todos los servidores conocidos cada `interval_secs`.
    /// Pasa `servers_dir` para construir la ruta de cada servidor.
    /// Si `interval_secs == 0` solo detiene el scheduler.
    pub async fn restart_auto_backup(self: &Arc<Self>, interval_secs: u64, servers_dir: String) {
        // Cancel previous task
        self.stop_scheduler().await;

        if interval_secs == 0 {
            return;
        }

        let this = self.clone();
        let sdir = servers_dir;
        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval_secs)).await;
                let servers = match this.servers.find_all().await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("auto-backup: failed to list servers: {}", e);
                        continue;
                    }
                };
                for srv in servers {
                    let server_dir = format!("{}/{}", sdir, srv.name());
                    if let Err(e) = this
                        .backup_server(srv.id(), srv.name().as_str(), &server_dir)
                        .await
                    {
                        tracing::warn!(server_id = %srv.id(), "auto-backup failed: {}", e);
                    }
                }
            }
        });
        *self.scheduler.lock().await = Some(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::InMemoryBackupRepo;
    use crate::persistence::in_memory::InMemoryServerRepo;
    use cubed_domain::entities::{Server, ServerSoftware};
    use cubed_domain::value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};

    fn make_server() -> Server {
        Server::new(
            ServerName::new("test-server").unwrap(),
            ServerVersion::new("1.21").unwrap(),
            ServerSoftware::Paper,
            ServerPort::new(25565).unwrap(),
            JavaPath::new("/usr/bin/java").unwrap(),
        )
    }

    #[tokio::test]
    async fn backup_nonexistent_dir_returns_error() {
        let server = make_server();
        let sid = server.id();
        let servers = InMemoryServerRepo::new();
        servers.save(&server).await.unwrap();
        let repo = InMemoryBackupRepo::new();
        let mgr = FileBackupManager::new("/tmp/cubed-test-backups", servers, repo);

        // Backup a non-existent source dir — tar should fail (unless /no/such exists)
        let result = mgr
            .backup_server(sid, "test-server", "/no/such/directory")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn backup_real_dir_creates_file() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let src = dir.path().to_str().unwrap();
        // Create a dummy file so tar has something
        tokio::fs::write(format!("{}/world.dat", src), b"dummy")
            .await
            .unwrap();

        let server = make_server();
        let sid = server.id();
        let servers = InMemoryServerRepo::new();
        servers.save(&server).await.unwrap();
        let repo = InMemoryBackupRepo::new();

        let bk_dir = tempdir().unwrap();
        let mgr = FileBackupManager::new(bk_dir.path().to_str().unwrap(), servers, repo.clone());

        let backup = mgr.backup_server(sid, "test-server", src).await.unwrap();
        assert!(backup.size_bytes() > 0);
        assert!(std::path::Path::new(backup.path()).exists());

        // Verify persisted
        let found = repo.find_by_id(backup.id()).await.unwrap();
        assert!(found.is_some());
    }
}
