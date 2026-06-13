//! # cubed-infrastructure
//!
//! Capa de Infraestructura (Clean Architecture).
//!
//! Implementa los puertos definidos en cubed-application hablando con
//! el mundo real: PostgreSQL (SQLx), sistema de archivos, procesos, etc.

pub mod backup;
pub mod console;
pub mod mods;
pub mod modpacks;
pub mod network;
pub mod downloader;
pub mod fs;
pub mod java;
pub mod persistence;
pub mod port;
pub mod process;
pub mod resources;

pub use backup::{FileBackupManager, InMemoryBackupRepo};
pub use mods::{FileModManager, InMemoryModRepo};
pub use modpacks::{InMemoryModpackRepo, InstallProgress, InstallSummary, ModpackInstaller};
pub use network::TailscaleNetworkManager;
pub use console::MinecraftConsoleManager;
pub use downloader::HttpDownloader;
pub use fs::LocalFileSystem;
pub use java::SystemJavaManager;
pub use persistence::{connect, check_integrity, InMemoryServerRepo, JsonServerRepository, PostgresServerRepository};
pub use port::TcpPortManager;
pub use process::MinecraftProcessManager;
pub use resources::SysInfoResourceMonitor;
