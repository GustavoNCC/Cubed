use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Un mod instalado en un servidor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    id: Uuid,
    server_id: Uuid,
    /// Nombre del archivo .jar.
    file_name: String,
    /// Ruta absoluta al .jar dentro de la carpeta mods/.
    path: String,
}

impl ModEntry {
    pub fn new(server_id: Uuid, file_name: String, path: String) -> Self {
        Self { id: Uuid::new_v4(), server_id, file_name, path }
    }

    pub fn reconstitute(id: Uuid, server_id: Uuid, file_name: String, path: String) -> Self {
        Self { id, server_id, file_name, path }
    }

    pub fn id(&self) -> Uuid { self.id }
    pub fn server_id(&self) -> Uuid { self.server_id }
    pub fn file_name(&self) -> &str { &self.file_name }
    pub fn path(&self) -> &str { &self.path }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_entry_fields() {
        let srv = Uuid::new_v4();
        let m = ModEntry::new(srv, "lithium.jar".into(), "/mods/lithium.jar".into());
        assert_eq!(m.file_name(), "lithium.jar");
        assert_eq!(m.server_id(), srv);
    }
}
