use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use cubed_application::CubedEvent;

/// Bus de eventos que traduce CubedEvent → evento Tauri global.
#[derive(Clone)]
pub struct EventBus {
    app: AppHandle,
}

impl EventBus {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self { app })
    }

    /// Emite el evento al frontend. Falla silenciosamente si no hay ventana.
    pub fn emit(&self, event: CubedEvent) {
        let channel = event.channel();
        // CubedEvent is Serialize, emit the whole struct as payload
        let _ = self.app.emit(&channel, &event);
    }
}
