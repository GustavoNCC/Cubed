use async_trait::async_trait;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout, ChildStderr};
use tokio::sync::Mutex;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::{ConsoleLine, ConsoleCallback, ConsoleManager};

const BUFFER_LINES: usize = 500;

struct ConsoleState {
    stdin: Option<ChildStdin>,
    buffer: VecDeque<ConsoleLine>,
}

pub struct MinecraftConsoleManager {
    state: Arc<Mutex<HashMap<Uuid, ConsoleState>>>,
}

impl MinecraftConsoleManager {
    pub fn new() -> Self {
        Self { state: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Registra el stdin de un proceso recién iniciado.
    /// Debe llamarse desde el `ProcessManager` inmediatamente después de `spawn`.
    pub async fn register_stdin(&self, server_id: Uuid, stdin: ChildStdin) {
        let mut map = self.state.lock().await;
        map.entry(server_id).or_insert_with(|| ConsoleState {
            stdin: None,
            buffer: VecDeque::new(),
        }).stdin = Some(stdin);
    }

    /// Lanza las tareas de lectura de stdout y stderr.
    /// El callback es el puente hacia Tauri events o cualquier otro destino.
    pub async fn spawn_readers(
        &self,
        server_id: Uuid,
        stdout: ChildStdout,
        stderr: ChildStderr,
        callback: Arc<ConsoleCallback>,
    ) {
        let state_out = self.state.clone();
        let cb_out = callback.clone();

        // stdout reader
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let entry = ConsoleLine { server_id, is_stdout: true, text: line.clone() };
                cb_out(entry.clone());
                let mut map = state_out.lock().await;
                if let Some(cs) = map.get_mut(&server_id) {
                    cs.buffer.push_back(entry);
                    if cs.buffer.len() > BUFFER_LINES {
                        cs.buffer.pop_front();
                    }
                }
            }
        });

        let state_err = self.state.clone();
        let cb_err = callback;

        // stderr reader
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let entry = ConsoleLine { server_id, is_stdout: false, text: line.clone() };
                cb_err(entry.clone());
                let mut map = state_err.lock().await;
                if let Some(cs) = map.get_mut(&server_id) {
                    cs.buffer.push_back(entry);
                    if cs.buffer.len() > BUFFER_LINES {
                        cs.buffer.pop_front();
                    }
                }
            }
        });
    }
}

impl Default for MinecraftConsoleManager {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl ConsoleManager for MinecraftConsoleManager {
    async fn attach(
        &self,
        server_id: Uuid,
        callback: ConsoleCallback,
    ) -> ApplicationResult<()> {
        // Ensure slot exists
        let mut map = self.state.lock().await;
        map.entry(server_id).or_insert_with(|| ConsoleState {
            stdin: None,
            buffer: VecDeque::new(),
        });
        drop(map);

        // The actual stdout/stderr readers are spawned via spawn_readers(),
        // called by the composition root after process spawn. Here we just
        // replay the existing buffer to the new callback so late subscribers
        // get history.
        let map = self.state.lock().await;
        if let Some(cs) = map.get(&server_id) {
            for line in &cs.buffer {
                callback(line.clone());
            }
        }
        Ok(())
    }

    async fn send_command(&self, server_id: Uuid, command: &str) -> ApplicationResult<()> {
        let mut map = self.state.lock().await;
        let cs = map.get_mut(&server_id).ok_or_else(|| {
            ApplicationError::Infrastructure(
                format!("No hay consola activa para el servidor {}", server_id),
            )
        })?;

        let stdin = cs.stdin.as_mut().ok_or_else(|| {
            ApplicationError::Infrastructure("stdin no disponible".into())
        })?;

        let line = format!("{}\n", command);
        stdin.write_all(line.as_bytes())
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        stdin.flush()
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(())
    }

    fn tail(&self, server_id: Uuid, n: usize) -> Vec<ConsoleLine> {
        match self.state.try_lock() {
            Ok(map) => map
                .get(&server_id)
                .map(|cs| cs.buffer.iter().rev().take(n).cloned().collect::<Vec<_>>().into_iter().rev().collect())
                .unwrap_or_default(),
            Err(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn tail_empty_initially() {
        let mgr = MinecraftConsoleManager::new();
        assert!(mgr.tail(Uuid::new_v4(), 10).is_empty());
    }

    #[tokio::test]
    async fn send_command_no_process_returns_error() {
        let mgr = MinecraftConsoleManager::new();
        assert!(mgr.send_command(Uuid::new_v4(), "say hi").await.is_err());
    }

    #[tokio::test]
    async fn buffer_stores_lines_up_to_limit() {
        let mgr = MinecraftConsoleManager::new();
        let id = Uuid::new_v4();

        // Fill buffer via attach callback replay (simulate reader inserting directly)
        {
            let mut map = mgr.state.lock().await;
            let cs = map.entry(id).or_insert_with(|| ConsoleState {
                stdin: None,
                buffer: VecDeque::new(),
            });
            for i in 0..(BUFFER_LINES + 10) {
                cs.buffer.push_back(ConsoleLine {
                    server_id: id,
                    is_stdout: true,
                    text: format!("line {}", i),
                });
                if cs.buffer.len() > BUFFER_LINES {
                    cs.buffer.pop_front();
                }
            }
        }

        let tail = mgr.tail(id, BUFFER_LINES + 100);
        assert_eq!(tail.len(), BUFFER_LINES);
        // Last inserted line is the last in tail
        assert_eq!(tail.last().unwrap().text, format!("line {}", BUFFER_LINES + 9));
    }

    #[tokio::test]
    async fn attach_replays_buffer_to_callback() {
        let mgr = MinecraftConsoleManager::new();
        let id = Uuid::new_v4();

        {
            let mut map = mgr.state.lock().await;
            let cs = map.entry(id).or_insert_with(|| ConsoleState {
                stdin: None,
                buffer: VecDeque::new(),
            });
            for i in 0..5 {
                cs.buffer.push_back(ConsoleLine {
                    server_id: id,
                    is_stdout: true,
                    text: format!("msg {}", i),
                });
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        mgr.attach(id, Box::new(move |_| { c.fetch_add(1, Ordering::SeqCst); }))
            .await
            .unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }
}
