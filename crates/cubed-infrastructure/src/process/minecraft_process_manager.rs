use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::{ProcessInfo, ProcessManager};

struct ManagedProcess {
    pid: u32,
    child: Child,
}

pub struct MinecraftProcessManager {
    // server_id → proceso
    processes: Arc<Mutex<HashMap<Uuid, ManagedProcess>>>,
}

impl MinecraftProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for MinecraftProcessManager {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl ProcessManager for MinecraftProcessManager {
    async fn spawn(
        &self,
        server_id: Uuid,
        java_path: &str,
        jar_path: &str,
        work_dir: &str,
        memory_mb: u32,
    ) -> ApplicationResult<u32> {
        let child = Command::new(java_path)
            .arg(format!("-Xms{}M", memory_mb / 2))
            .arg(format!("-Xmx{}M", memory_mb))
            .arg("-jar")
            .arg(jar_path)
            .arg("--nogui")
            .current_dir(work_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| ApplicationError::Infrastructure(
                format!("No se pudo iniciar el proceso Java: {}", e),
            ))?;

        let pid = child.id().ok_or_else(|| {
            ApplicationError::Infrastructure("No se pudo obtener el PID del proceso".into())
        })?;

        self.processes.lock().await.insert(server_id, ManagedProcess { pid, child });

        Ok(pid)
    }

    async fn stop(&self, server_id: Uuid) -> ApplicationResult<()> {
        let mut procs = self.processes.lock().await;
        let proc = procs.get_mut(&server_id).ok_or_else(|| {
            ApplicationError::Infrastructure(
                format!("No hay proceso activo para el servidor {}", server_id),
            )
        })?;

        if let Some(stdin) = proc.child.stdin.as_mut() {
            stdin.write_all(b"stop\n").await.ok();
            stdin.flush().await.ok();
        }

        Ok(())
    }

    async fn kill(&self, server_id: Uuid) -> ApplicationResult<()> {
        let mut procs = self.processes.lock().await;
        if let Some(proc) = procs.get_mut(&server_id) {
            proc.child.kill().await.map_err(|e| {
                ApplicationError::Infrastructure(format!("No se pudo matar el proceso: {}", e))
            })?;
            procs.remove(&server_id);
        }
        Ok(())
    }

    async fn is_alive(&self, server_id: Uuid) -> ApplicationResult<bool> {
        let mut procs = self.processes.lock().await;
        let proc = match procs.get_mut(&server_id) {
            Some(p) => p,
            None => return Ok(false),
        };

        // try_wait: None = sigue corriendo, Some(_) = terminó
        match proc.child.try_wait() {
            Ok(None) => Ok(true),
            Ok(Some(_)) => {
                procs.remove(&server_id);
                Ok(false)
            }
            Err(e) => Err(ApplicationError::Infrastructure(e.to_string())),
        }
    }

    fn list_active(&self) -> Vec<ProcessInfo> {
        // list_active es sync → usamos try_lock para no bloquear
        match self.processes.try_lock() {
            Ok(procs) => procs
                .iter()
                .map(|(id, p)| ProcessInfo { server_id: *id, pid: p.pid })
                .collect(),
            Err(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn is_alive_unknown_server_returns_false() {
        let mgr = MinecraftProcessManager::new();
        let result = mgr.is_alive(Uuid::new_v4()).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn kill_unknown_server_is_ok() {
        let mgr = MinecraftProcessManager::new();
        assert!(mgr.kill(Uuid::new_v4()).await.is_ok());
    }

    #[tokio::test]
    async fn list_active_empty_initially() {
        let mgr = MinecraftProcessManager::new();
        assert!(mgr.list_active().is_empty());
    }

    #[tokio::test]
    async fn spawn_short_lived_process_and_detect_exit() {
        let mgr = MinecraftProcessManager::new();
        let id = Uuid::new_v4();

        // Usa `true` (unix) o `cmd /c exit` — procesos que terminan inmediatamente
        #[cfg(unix)]
        let (bin, args): (&str, &[&str]) = ("true", &[]);
        #[cfg(windows)]
        let (bin, args): (&str, &[&str]) = ("cmd", &["/c", "exit"]);

        let child = tokio::process::Command::new(bin)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let pid = child.id().unwrap();
        mgr.processes.lock().await.insert(id, ManagedProcess { pid, child });

        // Espera a que termine
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        assert!(!mgr.is_alive(id).await.unwrap());
        // Tras is_alive=false el proceso se elimina del mapa
        assert!(mgr.list_active().is_empty());
    }

    #[tokio::test]
    async fn spawn_long_lived_process_is_alive() {
        let mgr = MinecraftProcessManager::new();
        let id = Uuid::new_v4();

        #[cfg(unix)]
        let (bin, args): (&str, &[&str]) = ("sleep", &["5"]);
        #[cfg(windows)]
        let (bin, args): (&str, &[&str]) = ("ping", &["-n", "10", "127.0.0.1"]);

        let child = tokio::process::Command::new(bin)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let pid = child.id().unwrap();
        mgr.processes.lock().await.insert(id, ManagedProcess { pid, child });

        assert!(mgr.is_alive(id).await.unwrap());
        mgr.kill(id).await.unwrap();
    }
}
