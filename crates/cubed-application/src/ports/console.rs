use async_trait::async_trait;
use uuid::Uuid;
use crate::error::ApplicationResult;

/// Una línea emitida por el proceso del servidor.
#[derive(Debug, Clone)]
pub struct ConsoleLine {
    pub server_id: Uuid,
    /// true = stdout, false = stderr
    pub is_stdout: bool,
    pub text: String,
}

/// Callback que recibe líneas de consola en tiempo real.
pub type ConsoleCallback = Box<dyn Fn(ConsoleLine) + Send + Sync + 'static>;

/// Puerto para lectura/escritura de la consola de un servidor Minecraft.
#[async_trait]
pub trait ConsoleManager: Send + Sync {
    /// Adjunta lectores de stdout y stderr al proceso ya en marcha.
    /// Las líneas se entregan al callback hasta que el proceso termina.
    async fn attach(
        &self,
        server_id: Uuid,
        callback: ConsoleCallback,
    ) -> ApplicationResult<()>;

    /// Envía un comando a stdin del proceso (p. ej. "say Hola").
    async fn send_command(&self, server_id: Uuid, command: &str) -> ApplicationResult<()>;

    /// Devuelve las últimas `n` líneas del buffer circular en memoria.
    fn tail(&self, server_id: Uuid, n: usize) -> Vec<ConsoleLine>;
}
