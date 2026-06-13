use crate::error::ApplicationResult;
use async_trait::async_trait;

/// Información sobre una instalación de Java encontrada en el sistema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaInstallation {
    /// Ruta absoluta al binario `java`.
    pub path: String,
    /// Versión mayor (8, 11, 17, 21, ...).
    pub major_version: u32,
    /// Cadena de versión completa tal como la reporta `java -version`.
    pub version_string: String,
}

/// Puerto para detección y validación de instalaciones de Java.
#[async_trait]
pub trait JavaManager: Send + Sync {
    /// Detecta todas las instalaciones de Java accesibles en el sistema.
    async fn detect_installations(&self) -> ApplicationResult<Vec<JavaInstallation>>;

    /// Devuelve la información de la instalación en la ruta indicada.
    async fn inspect(&self, path: &str) -> ApplicationResult<JavaInstallation>;

    /// Valida que la versión de Java es compatible con la versión de Minecraft dada.
    /// Minecraft 1.17+ requiere Java 16+; 1.18+ Java 17+; 1.20.5+ Java 21+.
    fn validate_compatibility(
        &self,
        java: &JavaInstallation,
        minecraft_version: &str,
    ) -> ApplicationResult<()>;

    /// Selecciona el binario más adecuado para la versión de Minecraft indicada.
    async fn select_for_version(
        &self,
        minecraft_version: &str,
    ) -> ApplicationResult<JavaInstallation>;
}
