use crate::error::ApplicationResult;
use crate::ports::{JavaInstallation, JavaManager};
use std::sync::Arc;

pub struct SelectJava {
    java: Arc<dyn JavaManager>,
}

impl SelectJava {
    pub fn new(java: Arc<dyn JavaManager>) -> Self {
        Self { java }
    }

    /// Devuelve todas las instalaciones detectadas.
    pub async fn list(&self) -> ApplicationResult<Vec<JavaInstallation>> {
        self.java.detect_installations().await
    }

    /// Selecciona el mejor binario para la versión de Minecraft indicada.
    pub async fn for_version(
        &self,
        minecraft_version: &str,
    ) -> ApplicationResult<JavaInstallation> {
        self.java.select_for_version(minecraft_version).await
    }

    /// Inspecciona una ruta concreta y valida compatibilidad.
    pub async fn inspect_and_validate(
        &self,
        path: &str,
        minecraft_version: &str,
    ) -> ApplicationResult<JavaInstallation> {
        let installation = self.java.inspect(path).await?;
        self.java
            .validate_compatibility(&installation, minecraft_version)?;
        Ok(installation)
    }
}
