//! # cubed-domain
//!
//! Capa de Dominio (Clean Architecture / DDD).
//!
//! Contiene las reglas de negocio centrales de Cubed: entidades, value objects
//! y errores de dominio. Esta capa NO conoce React, Tauri, PostgreSQL ni el
//! sistema de archivos. No depende de ninguna otra capa de Cubed.

pub mod entities;
pub mod error;
pub mod value_objects;

pub use entities::{
    Backup, ModEntry, Modpack, ModpackFormat, Server, ServerSoftware, ServerStatus, Settings,
};
pub use value_objects::{JavaPath, ServerName, ServerPort, ServerVersion};

pub const DOMAIN_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_version_is_exposed() {
        assert!(!DOMAIN_VERSION.is_empty());
    }
}
