//! # cubed-infrastructure
//!
//! Capa de Infraestructura (Clean Architecture).
//!
//! Implementa los puertos definidos en la capa de aplicación hablando con el
//! mundo real: PostgreSQL (SQLx), sistema de archivos, procesos Java, APIs de
//! Paper/Purpur/Fabric/Forge/NeoForge, Tailscale CLI, etc.
//!
//! Puede depender de dominio y aplicación. Nadie depende de ella salvo el
//! ensamblador final (src-tauri).
//!
//! Las implementaciones reales (PostgreSQL en Fase 2, FS en Fase 3, ...) se
//! añaden según el Roadmap.

#[cfg(test)]
mod tests {
    #[test]
    fn infrastructure_layer_links() {
        assert!(true);
    }
}
