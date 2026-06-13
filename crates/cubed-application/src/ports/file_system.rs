use crate::error::ApplicationResult;
use async_trait::async_trait;

/// Puerto para operaciones de sistema de archivos.
/// La infraestructura lo implementa; la aplicación solo lo conoce como trait.
#[async_trait]
pub trait FileSystemManager: Send + Sync {
    /// Crea la estructura global de Cubed si no existe:
    /// /home/cubed/{servers,backups,downloads,temp,config,logs}
    async fn init_cubed_dirs(&self) -> ApplicationResult<()>;

    /// Crea la estructura de directorios de un servidor específico:
    /// <servers_dir>/<name>/{mods,world,config,logs}
    async fn init_server_dirs(&self, servers_dir: &str, server_name: &str)
        -> ApplicationResult<()>;

    /// Elimina el directorio completo de un servidor.
    async fn delete_server_dir(
        &self,
        servers_dir: &str,
        server_name: &str,
    ) -> ApplicationResult<()>;

    /// Devuelve la ruta al directorio de un servidor.
    fn server_dir(&self, servers_dir: &str, server_name: &str) -> String;

    /// Verifica que un directorio existe y es escribible.
    async fn ensure_writable(&self, path: &str) -> ApplicationResult<()>;
}
