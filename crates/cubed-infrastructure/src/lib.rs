//! # cubed-infrastructure
//!
//! Capa de Infraestructura (Clean Architecture).
//!
//! Implementa los puertos definidos en cubed-application hablando con
//! el mundo real: PostgreSQL (SQLx), sistema de archivos, procesos, etc.

pub mod backup;
pub mod console;
pub mod downloader;
pub mod fs;
pub mod java;
pub mod modpacks;
pub mod mods;
pub mod network;
pub mod persistence;
pub mod port;
pub mod process;
pub mod resources;

pub use backup::{FileBackupManager, InMemoryBackupRepo, JsonBackupRepo, PostgresBackupRepo};
pub use console::MinecraftConsoleManager;
pub use downloader::HttpDownloader;
pub use fs::LocalFileSystem;
pub use java::SystemJavaManager;
pub use modpacks::{
    InMemoryModpackRepo, InstallProgress, InstallSummary, JsonModpackRepo, ModpackInstaller,
    PostgresModpackRepo,
};
pub use mods::{FileModManager, InMemoryModRepo, JsonModRepo, PostgresModRepo};
pub use network::TailscaleNetworkManager;
pub use persistence::{
    check_integrity, connect, InMemoryServerRepo, JsonServerRepository, JsonSettingsStore,
    PostgresServerRepository, PostgresSettingsStore,
};
pub use port::TcpPortManager;
pub use process::MinecraftProcessManager;
pub use resources::SysInfoResourceMonitor;
