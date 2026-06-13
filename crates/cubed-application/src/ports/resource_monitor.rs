use async_trait::async_trait;
use uuid::Uuid;
use crate::error::ApplicationResult;

/// Estadísticas globales del sistema anfitrión.
#[derive(Debug, Clone)]
pub struct SystemStats {
    /// Uso de CPU total en porcentaje (0–100).
    pub cpu_percent: f32,
    /// RAM usada en bytes.
    pub ram_used_bytes: u64,
    /// RAM total en bytes.
    pub ram_total_bytes: u64,
    /// Disco usado en bytes (raíz o partición principal).
    pub disk_used_bytes: u64,
    /// Disco total en bytes.
    pub disk_total_bytes: u64,
    /// Bytes recibidos desde el inicio del proceso (Δ acumulado).
    pub net_rx_bytes: u64,
    /// Bytes enviados desde el inicio del proceso (Δ acumulado).
    pub net_tx_bytes: u64,
}

/// Estadísticas de un proceso de servidor Minecraft concreto.
#[derive(Debug, Clone)]
pub struct ServerStats {
    pub server_id: Uuid,
    /// CPU del proceso en porcentaje.
    pub cpu_percent: f32,
    /// RAM del proceso (RSS) en bytes.
    pub ram_bytes: u64,
    /// Segundos que lleva el proceso vivo.
    pub uptime_secs: u64,
}

#[async_trait]
pub trait ResourceMonitor: Send + Sync {
    /// Estadísticas actuales del sistema.
    async fn system_stats(&self) -> ApplicationResult<SystemStats>;
    /// Estadísticas del proceso asociado a un servidor.
    /// Devuelve `None` si el proceso no está vivo.
    async fn server_stats(&self, server_id: Uuid, pid: u32) -> ApplicationResult<Option<ServerStats>>;
}
