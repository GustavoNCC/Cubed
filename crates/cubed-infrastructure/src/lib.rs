//! # cubed-infrastructure
//!
//! Capa de Infraestructura (Clean Architecture).
//!
//! Implementa los puertos definidos en cubed-application hablando con
//! el mundo real: PostgreSQL (SQLx), sistema de archivos, procesos, etc.

pub mod downloader;
pub mod fs;
pub mod java;
pub mod persistence;
pub mod port;

pub use downloader::HttpDownloader;
pub use fs::LocalFileSystem;
pub use java::SystemJavaManager;
pub use persistence::{connect, PostgresServerRepository};
pub use port::TcpPortManager;
