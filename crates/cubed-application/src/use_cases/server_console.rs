use crate::error::ApplicationResult;
use crate::ports::{ConsoleLine, ConsoleManager};
use std::sync::Arc;
use uuid::Uuid;

pub struct ServerConsole {
    console: Arc<dyn ConsoleManager>,
}

impl ServerConsole {
    pub fn new(console: Arc<dyn ConsoleManager>) -> Self {
        Self { console }
    }

    /// Adjunta el lector al proceso e invoca el callback por cada línea.
    pub async fn attach(
        &self,
        server_id: Uuid,
        callback: impl Fn(ConsoleLine) + Send + Sync + 'static,
    ) -> ApplicationResult<()> {
        self.console.attach(server_id, Box::new(callback)).await
    }

    /// Envía un comando a stdin.
    pub async fn send(&self, server_id: Uuid, command: &str) -> ApplicationResult<()> {
        self.console.send_command(server_id, command).await
    }

    /// Últimas `n` líneas del buffer.
    pub fn tail(&self, server_id: Uuid, n: usize) -> Vec<ConsoleLine> {
        self.console.tail(server_id, n)
    }
}
