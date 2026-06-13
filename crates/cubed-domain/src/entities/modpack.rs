use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Formato del archivo de modpack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModpackFormat {
    Mrpack,
    Zip,
}

impl std::fmt::Display for ModpackFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mrpack => write!(f, ".mrpack"),
            Self::Zip => write!(f, ".zip"),
        }
    }
}

/// Modpack importado asociado a un servidor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Modpack {
    id: Uuid,
    server_id: Uuid,
    name: String,
    format: ModpackFormat,
    /// Ruta al archivo fuente del modpack.
    source_path: String,
}

impl Modpack {
    pub fn new(
        server_id: Uuid,
        name: String,
        format: ModpackFormat,
        source_path: String,
    ) -> Self {
        Self { id: Uuid::new_v4(), server_id, name, format, source_path }
    }

    pub fn reconstitute(
        id: Uuid,
        server_id: Uuid,
        name: String,
        format: ModpackFormat,
        source_path: String,
    ) -> Self {
        Self { id, server_id, name, format, source_path }
    }

    pub fn id(&self) -> Uuid { self.id }
    pub fn server_id(&self) -> Uuid { self.server_id }
    pub fn name(&self) -> &str { &self.name }
    pub fn format(&self) -> &ModpackFormat { &self.format }
    pub fn source_path(&self) -> &str { &self.source_path }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modpack_format_display() {
        assert_eq!(ModpackFormat::Mrpack.to_string(), ".mrpack");
    }
}
