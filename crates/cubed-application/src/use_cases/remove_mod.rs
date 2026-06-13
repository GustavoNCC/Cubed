use crate::error::{ApplicationError, ApplicationResult};
use crate::ports::ModRepository;
use std::sync::Arc;
use uuid::Uuid;

pub struct RemoveMod {
    mods: Arc<dyn ModRepository>,
}

impl RemoveMod {
    pub fn new(mods: Arc<dyn ModRepository>) -> Self {
        Self { mods }
    }

    /// Elimina el mod del repositorio y devuelve la ruta para borrar el archivo.
    pub async fn execute(&self, mod_id: Uuid) -> ApplicationResult<String> {
        let entry = self.mods.find_by_id(mod_id).await?.ok_or_else(|| {
            ApplicationError::Infrastructure(format!("Mod {} no encontrado", mod_id))
        })?;
        let path = entry.path().to_string();
        self.mods.delete(mod_id).await?;
        Ok(path)
    }
}
