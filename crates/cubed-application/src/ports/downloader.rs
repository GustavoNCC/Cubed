use async_trait::async_trait;
use cubed_domain::entities::ServerSoftware;
use crate::error::ApplicationResult;

/// Resultado de una descarga.
#[derive(Debug, Clone)]
pub struct DownloadedJar {
    /// Ruta absoluta al archivo descargado.
    pub path: String,
    /// Tamaño en bytes.
    pub size_bytes: u64,
}

/// Puerto para descarga de JARs de servidor.
#[async_trait]
pub trait Downloader: Send + Sync {
    /// Descarga el JAR del software indicado para la versión de Minecraft dada.
    /// Lo deposita en `dest_dir` y devuelve la ruta resultante.
    async fn download(
        &self,
        software: &ServerSoftware,
        minecraft_version: &str,
        dest_dir: &str,
    ) -> ApplicationResult<DownloadedJar>;

    /// Construye la URL de descarga sin hacer la petición.
    fn build_url(&self, software: &ServerSoftware, minecraft_version: &str) -> ApplicationResult<String>;
}
