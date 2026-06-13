//! # cubed-infrastructure
//!
//! Capa de Infraestructura (Clean Architecture).
//!
//! Implementa los puertos definidos en cubed-application hablando con
//! el mundo real: PostgreSQL (SQLx), sistema de archivos, procesos, etc.

pub mod persistence;

pub use persistence::{connect, PostgresServerRepository};
