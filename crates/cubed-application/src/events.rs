use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Todos los eventos de dominio que Cubed puede emitir.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CubedEvent {
    ServerStarted  { server_id: Uuid },
    ServerStopped  { server_id: Uuid },
    ServerCrashed  { server_id: Uuid },
    BackupCreated  { server_id: Uuid, backup_id: Uuid },
    ResourceUpdated { server_id: Option<Uuid> },
    TailscaleUpdated { connected: bool, ip: Option<String> },
    /// Re-exportado para consistencia; el streaming real va por console-line:<id>
    ConsoleLine    { server_id: Uuid, is_stdout: bool, text: String },
}

impl CubedEvent {
    /// Nombre del evento Tauri (canal) al que se emite.
    pub fn channel(&self) -> String {
        match self {
            Self::ServerStarted  { .. }   => "cubed://server.started".into(),
            Self::ServerStopped  { .. }   => "cubed://server.stopped".into(),
            Self::ServerCrashed  { .. }   => "cubed://server.crashed".into(),
            Self::BackupCreated  { .. }   => "cubed://backup.created".into(),
            Self::ResourceUpdated { .. }  => "cubed://resource.updated".into(),
            Self::TailscaleUpdated { .. } => "cubed://tailscale.updated".into(),
            Self::ConsoleLine { server_id, .. } => format!("console-line:{}", server_id),
        }
    }
}
