use async_trait::async_trait;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::Mutex;
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::{ConsoleCallback, ConsoleLine, ConsoleManager};

const BUFFER_LINES: usize = 500;

struct ConsoleState {
    stdin: Option<ChildStdin>,
    buffer: VecDeque<ConsoleLine>,
    /// Real-time subscribers. Each entry receives every line as it arrives.
    callbacks: Vec<ConsoleCallback>,
}

pub struct MinecraftConsoleManager {
    state: Arc<Mutex<HashMap<Uuid, ConsoleState>>>,
}

impl MinecraftConsoleManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn ensure_slot(map: &mut HashMap<Uuid, ConsoleState>, server_id: Uuid) {
        map.entry(server_id).or_insert_with(|| ConsoleState {
            stdin: None,
            buffer: VecDeque::new(),
            callbacks: Vec::new(),
        });
    }

    /// Registra el stdin de un proceso recién iniciado.
    pub async fn register_stdin(&self, server_id: Uuid, stdin: ChildStdin) {
        let mut map = self.state.lock().await;
        Self::ensure_slot(&mut map, server_id);
        map.get_mut(&server_id).unwrap().stdin = Some(stdin);
    }

    /// Lanza lectores de stdout y stderr. Cada línea se entrega a todos
    /// los callbacks suscritos y se almacena en el buffer circular.
    pub async fn spawn_readers(&self, server_id: Uuid, stdout: ChildStdout, stderr: ChildStderr) {
        let state_out = self.state.clone();
        let state_err = self.state.clone();

        // stdout reader
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let entry = ConsoleLine {
                    server_id,
                    is_stdout: true,
                    text: line,
                };
                let mut map = state_out.lock().await;
                if let Some(cs) = map.get_mut(&server_id) {
                    // deliver to all subscribers
                    for cb in &cs.callbacks {
                        cb(entry.clone());
                    }
                    // store in ring buffer
                    cs.buffer.push_back(entry);
                    if cs.buffer.len() > BUFFER_LINES {
                        cs.buffer.pop_front();
                    }
                }
            }
        });

        // stderr reader
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let entry = ConsoleLine {
                    server_id,
                    is_stdout: false,
                    text: line,
                };
                let mut map = state_err.lock().await;
                if let Some(cs) = map.get_mut(&server_id) {
                    for cb in &cs.callbacks {
                        cb(entry.clone());
                    }
                    cs.buffer.push_back(entry);
                    if cs.buffer.len() > BUFFER_LINES {
                        cs.buffer.pop_front();
                    }
                }
            }
        });
    }

    /// Elimina todos los callbacks y el stdin de un servidor (proceso terminó).
    pub async fn detach(&self, server_id: Uuid) {
        let mut map = self.state.lock().await;
        if let Some(cs) = map.get_mut(&server_id) {
            cs.stdin = None;
            cs.callbacks.clear();
        }
    }
}

impl Default for MinecraftConsoleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ConsoleManager for MinecraftConsoleManager {
    /// Suscribe un callback a líneas en tiempo real. También reproduce el buffer
    /// histórico inmediatamente para que el suscriptor no pierda el pasado.
    async fn attach(&self, server_id: Uuid, callback: ConsoleCallback) -> ApplicationResult<()> {
        let mut map = self.state.lock().await;
        Self::ensure_slot(&mut map, server_id);
        let cs = map.get_mut(&server_id).unwrap();
        // replay buffer to this subscriber
        for line in &cs.buffer {
            callback(line.clone());
        }
        // keep for future real-time delivery
        cs.callbacks.push(callback);
        Ok(())
    }

    async fn send_command(&self, server_id: Uuid, command: &str) -> ApplicationResult<()> {
        let mut map = self.state.lock().await;
        let cs = map.get_mut(&server_id).ok_or_else(|| {
            ApplicationError::Infrastructure(format!(
                "No hay consola activa para el servidor {}",
                server_id
            ))
        })?;

        let stdin = cs
            .stdin
            .as_mut()
            .ok_or_else(|| ApplicationError::Infrastructure("stdin no disponible".into()))?;

        let line = format!("{}\n", command);
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(())
    }

    fn tail(&self, server_id: Uuid, n: usize) -> Vec<ConsoleLine> {
        match self.state.try_lock() {
            Ok(map) => map
                .get(&server_id)
                .map(|cs| {
                    cs.buffer
                        .iter()
                        .rev()
                        .take(n)
                        .cloned()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect()
                })
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

        {
            let mut map = mgr.state.lock().await;
            let cs = map.entry(id).or_insert_with(|| ConsoleState {
                stdin: None,
                buffer: VecDeque::new(),
                callbacks: Vec::new(),
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
        assert_eq!(
            tail.last().unwrap().text,
            format!("line {}", BUFFER_LINES + 9)
        );
    }

    #[tokio::test]
    async fn attach_replays_buffer_and_delivers_future_lines() {
        let mgr = MinecraftConsoleManager::new();
        let id = Uuid::new_v4();

        // Pre-fill buffer
        {
            let mut map = mgr.state.lock().await;
            let cs = map.entry(id).or_insert_with(|| ConsoleState {
                stdin: None,
                buffer: VecDeque::new(),
                callbacks: Vec::new(),
            });
            for i in 0..3 {
                cs.buffer.push_back(ConsoleLine {
                    server_id: id,
                    is_stdout: true,
                    text: format!("msg {}", i),
                });
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        mgr.attach(
            id,
            Box::new(move |_| {
                c.fetch_add(1, Ordering::SeqCst);
            }),
        )
        .await
        .unwrap();

        // 3 replayed lines
        assert_eq!(counter.load(Ordering::SeqCst), 3);

        // Simulate a new line arriving
        {
            let mut map = mgr.state.lock().await;
            if let Some(cs) = map.get_mut(&id) {
                let entry = ConsoleLine {
                    server_id: id,
                    is_stdout: true,
                    text: "new".into(),
                };
                for cb in &cs.callbacks {
                    cb(entry.clone());
                }
            }
        }

        // Should now be 4
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }
}
