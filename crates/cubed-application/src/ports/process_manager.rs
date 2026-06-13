use async_trait::async_trait;
use uuid::Uuid;
use crate::error::ApplicationResult;

/// Información sobre un proceso de servidor en ejecución.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub server_id: Uuid,
    pub pid: u32,
}

/// Puerto para gestión de procesos de servidor Minecraft.
#[async_trait]
pub trait ProcessManager: Send + Sync {
    /// Lanza el proceso Java con el JAR indicado. Devuelve el PID asignado.
    async fn spawn(
        &self,
        server_id: Uuid,
        java_path: &str,
        jar_path: &str,
        work_dir: &str,
        memory_mb: u32,
    ) -> ApplicationResult<u32>;

    /// Envía el comando "stop" a stdin del proceso (parada limpia).
    async fn stop(&self, server_id: Uuid) -> ApplicationResult<()>;

    /// Envía SIGKILL al proceso (parada forzada).
    async fn kill(&self, server_id: Uuid) -> ApplicationResult<()>;

    /// Devuelve true si el proceso sigue corriendo.
    async fn is_alive(&self, server_id: Uuid) -> ApplicationResult<bool>;

    /// Devuelve información de todos los procesos activos.
    fn list_active(&self) -> Vec<ProcessInfo>;
}
