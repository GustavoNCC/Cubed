mod commands;
mod in_memory_repo;

use std::sync::Arc;
use commands::AppState;
use cubed_infrastructure::{
    FileBackupManager, FileModManager, InMemoryBackupRepo, InMemoryModRepo,
    LocalFileSystem, MinecraftConsoleManager, SysInfoResourceMonitor,
};

#[tauri::command]
fn health_check() -> String {
    format!("Cubed backend OK (domain v{})", cubed_domain::DOMAIN_VERSION)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
    let mod_repo = InMemoryModRepo::new();
    let mod_mgr  = FileModManager::new(repo.clone(), mod_repo.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState { repo, fs, console, resources, backup_repo, backup_mgr, mod_repo, mod_mgr })
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
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación Cubed");
}
