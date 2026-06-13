mod commands;
mod event_bus;
mod in_memory_repo;

use std::sync::Arc;
use commands::AppState;
use event_bus::EventBus;
use tauri::Manager;
use tracing_subscriber::EnvFilter;
use cubed_infrastructure::{
    FileBackupManager, FileModManager, InMemoryBackupRepo, InMemoryModpackRepo,
    InMemoryModRepo, LocalFileSystem, MinecraftConsoleManager,
    ModpackInstaller, SysInfoResourceMonitor, TailscaleNetworkManager,
};

#[tauri::command]
fn health_check() -> String {
    format!("Cubed backend OK (domain v{})", cubed_domain::DOMAIN_VERSION)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(
            "cubed=debug".parse().expect("valid directive"),
        ))
        .init();
    let repo        = in_memory_repo::InMemoryServerRepo::new();
    let fs          = Arc::new(LocalFileSystem::new("/tmp/cubed-dev"));
    let console     = Arc::new(MinecraftConsoleManager::new());
    let resources   = Arc::new(SysInfoResourceMonitor::new());
    let backup_repo = InMemoryBackupRepo::new();
    let backup_mgr  = FileBackupManager::new(
        "/tmp/cubed-dev/backups",
        repo.clone(),
        backup_repo.clone(),
    );
    let mod_repo      = InMemoryModRepo::new();
    let mod_mgr       = FileModManager::new(repo.clone(), mod_repo.clone());
    let modpack_repo  = InMemoryModpackRepo::new();
    let modpack_inst  = ModpackInstaller::new(repo.clone(), modpack_repo.clone());
    let network       = Arc::new(TailscaleNetworkManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(move |app| {
            let event_bus = EventBus::new(app.handle().clone());
            app.manage(AppState {
                repo, fs, console, resources,
                backup_repo, backup_mgr,
                mod_repo, mod_mgr,
                modpack_repo, modpack_inst,
                network,
                event_bus,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            health_check,
            commands::list_servers,
            commands::create_server,
            commands::start_server,
            commands::stop_server,
            commands::delete_server,
            commands::subscribe_console,
            commands::send_console_command,
            commands::get_console_tail,
            commands::get_system_stats,
            commands::get_server_stats,
            commands::create_backup,
            commands::list_backups,
            commands::restore_backup,
            commands::delete_backup,
            commands::list_mods,
            commands::install_mod,
            commands::remove_mod,
            commands::validate_jar,
            commands::install_modpack,
            commands::list_modpacks,
            commands::delete_modpack,
            commands::suggest_free_port,
            commands::tailscale_is_installed,
            commands::tailscale_status,
            commands::tailscale_ip,
            commands::server_connect_address,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación Cubed");
}
