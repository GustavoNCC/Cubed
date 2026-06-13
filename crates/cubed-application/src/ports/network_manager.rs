use crate::error::ApplicationResult;
use async_trait::async_trait;

/// Estado de la conexión Tailscale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TailscaleStatus {
    /// No instalado en el sistema.
    NotInstalled,
    /// Instalado pero no autenticado / desconectado.
    Disconnected,
    /// Conectado y con IP asignada.
    Connected { ip: String, hostname: String },
}

#[async_trait]
pub trait NetworkManager: Send + Sync {
    /// Detecta si Tailscale está instalado en el sistema.
    async fn is_installed(&self) -> ApplicationResult<bool>;

    /// Devuelve el estado actual de Tailscale.
    async fn status(&self) -> ApplicationResult<TailscaleStatus>;

    /// Devuelve la IP de Tailscale si está conectado, o `None` si no.
    async fn tailscale_ip(&self) -> ApplicationResult<Option<String>>;
}
