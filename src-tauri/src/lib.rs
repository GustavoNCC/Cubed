mod commands;
mod event_bus;

use commands::AppState;
use cubed_infrastructure::{
    check_integrity, FileBackupManager, FileModManager, InMemoryBackupRepo, InMemoryModRepo,
    InMemoryModpackRepo, JsonServerRepository, LocalFileSystem, MinecraftConsoleManager,
    MinecraftProcessManager, ModpackInstaller, SysInfoResourceMonitor, SystemJavaManager,
    TailscaleNetworkManager,
};
use event_bus::EventBus;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[tauri::command]
fn health_check() -> String {
    format!(
        "Cubed backend OK (domain v{})",
        cubed_domain::DOMAIN_VERSION
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("cubed=debug".parse().expect("valid directive")),
        )
        .init();

    let servers_dir = "/tmp/cubed-dev/servers".to_string();

    // Infraestructura sin dependencia del app handle
    let fs = Arc::new(LocalFileSystem::new("/tmp/cubed-dev"));
    let console = Arc::new(MinecraftConsoleManager::new());
    let process_mgr = Arc::new(MinecraftProcessManager::new());
    let resources = Arc::new(SysInfoResourceMonitor::new());
    let backup_repo = InMemoryBackupRepo::new();
    let mod_repo = InMemoryModRepo::new();
    let modpack_repo = InMemoryModpackRepo::new();
    let network = Arc::new(TailscaleNetworkManager::new());
    let java_mgr = Arc::new(SystemJavaManager::new());
    let settings = Arc::new(RwLock::new(cubed_domain::entities::Settings {
        servers_dir: servers_dir.clone(),
        backups_dir: "/tmp/cubed-dev/backups".into(),
        downloads_dir: "/tmp/cubed-dev/downloads".into(),
        default_java_path: "/usr/bin/java".into(),
        backup_interval_secs: 18_000,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            // Directorio de datos persistentes de la aplicación
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("No se puede determinar el directorio de datos de la aplicación");
            std::fs::create_dir_all(&data_dir).expect("No se puede crear el directorio de datos");

            let db_path = data_dir.join("servers.json");
            tracing::info!(path = %db_path.display(), "Base de datos JSON de servidores");

            let repo = Arc::new(JsonServerRepository::new(db_path.clone()));

            // Verificación de integridad al iniciar
            check_integrity(&db_path, &servers_dir);

            let backup_mgr =
                FileBackupManager::new("/tmp/cubed-dev/backups", repo.clone(), backup_repo.clone());
            let mod_mgr = FileModManager::new(repo.clone(), mod_repo.clone());
            let modpack_inst = ModpackInstaller::new(repo.clone(), modpack_repo.clone());

            let event_bus = EventBus::new(app.handle().clone());
            app.manage(AppState {
                repo,
                fs,
                console,
                process_mgr,
                servers_dir,
                resources,
                backup_repo,
                backup_mgr,
                mod_repo,
                mod_mgr,
                modpack_repo,
                modpack_inst,
                network,
                java_mgr,
                event_bus,
                settings,
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
            commands::detect_java,
            commands::select_java_for_version,
            commands::get_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación Cubed");
}
