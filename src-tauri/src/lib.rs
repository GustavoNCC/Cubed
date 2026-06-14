mod commands;
mod event_bus;

use commands::{AppState, SettingsStore};
use cubed_application::ports::ServerRepository;
use cubed_domain::entities::{ServerStatus, Settings};
use cubed_infrastructure::{
    check_integrity, connect, FileBackupManager, FileModManager, HttpDownloader, JsonBackupRepo,
    JsonModRepo, JsonModpackRepo, JsonServerRepository, JsonSettingsStore, LocalFileSystem,
    MinecraftConsoleManager, MinecraftProcessManager, ModpackInstaller, PostgresBackupRepo,
    PostgresModRepo, PostgresModpackRepo, PostgresServerRepository, PostgresSettingsStore,
    SysInfoResourceMonitor, SystemJavaManager, TailscaleNetworkManager,
};
use event_bus::EventBus;
use std::path::Path;
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

    let console = Arc::new(MinecraftConsoleManager::new());
    let process_mgr = Arc::new(MinecraftProcessManager::new());
    let resources = Arc::new(SysInfoResourceMonitor::new());
    let network = Arc::new(TailscaleNetworkManager::new());
    let java_mgr = Arc::new(SystemJavaManager::new());
    let downloader = Arc::new(HttpDownloader::new());

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
            let backups_path = data_dir.join("backups.json");
            let mods_path = data_dir.join("mods.json");
            let modpacks_path = data_dir.join("modpacks.json");
            let settings_path = data_dir.join("settings.json");
            let platform_defaults = default_settings_for_data_dir(&data_dir);
            tracing::info!(path = %db_path.display(), "Base de datos JSON de servidores");

            let database_url = std::env::var("DATABASE_URL").unwrap_or_default();
            let pg_pool = if database_url.trim().is_empty() {
                None
            } else {
                let pool = tauri::async_runtime::block_on(connect(&database_url))
                    .expect("No se pudo conectar a PostgreSQL");
                tracing::info!("Persistencia PostgreSQL habilitada por DATABASE_URL");
                Some(pool)
            };

            let json_settings_store = Arc::new(JsonSettingsStore::new(settings_path));
            let (settings_store, loaded_settings) = match &pg_pool {
                Some(pool) => {
                    let store = Arc::new(PostgresSettingsStore::new(pool.clone()));
                    let mut settings = tauri::async_runtime::block_on(store.load_or_default())
                        .expect("No se pudo cargar la configuración PostgreSQL");
                    if uses_builtin_storage_defaults(&settings) {
                        settings = platform_defaults.clone();
                        tauri::async_runtime::block_on(store.save(&settings))
                            .expect("No se pudo guardar la configuración PostgreSQL inicial");
                    }
                    (Arc::new(SettingsStore::Postgres(store)), settings)
                }
                None => {
                    let mut settings = json_settings_store
                        .load_or_default()
                        .expect("No se pudo cargar la configuración");
                    if uses_builtin_storage_defaults(&settings) {
                        settings = platform_defaults.clone();
                        json_settings_store
                            .save(&settings)
                            .expect("No se pudo guardar la configuración inicial");
                    }
                    (Arc::new(SettingsStore::Json(json_settings_store)), settings)
                }
            };
            let servers_dir = loaded_settings.servers_dir.clone();
            let settings = Arc::new(RwLock::new(loaded_settings.clone()));

            // Infraestructura dependiente de configuración
            let fs = Arc::new(LocalFileSystem::new(data_dir.to_string_lossy().to_string()));

            let repo: Arc<dyn ServerRepository> = match &pg_pool {
                Some(pool) => Arc::new(PostgresServerRepository::new(pool.clone())),
                None => Arc::new(JsonServerRepository::new(db_path.clone())),
            };
            let backup_repo: Arc<dyn cubed_application::ports::BackupRepository> = match &pg_pool {
                Some(pool) => Arc::new(PostgresBackupRepo::new(pool.clone())),
                None => Arc::new(JsonBackupRepo::new(backups_path)),
            };
            let mod_repo: Arc<dyn cubed_application::ports::ModRepository> = match &pg_pool {
                Some(pool) => Arc::new(PostgresModRepo::new(pool.clone())),
                None => Arc::new(JsonModRepo::new(mods_path)),
            };
            let modpack_repo: Arc<dyn cubed_application::ports::ModpackRepository> = match &pg_pool
            {
                Some(pool) => Arc::new(PostgresModpackRepo::new(pool.clone())),
                None => Arc::new(JsonModpackRepo::new(modpacks_path)),
            };

            // Verificación de integridad al iniciar
            if pg_pool.is_none() {
                check_integrity(&db_path, &servers_dir);
            } else {
                tracing::info!("check_integrity JSON omitido porque PostgreSQL está habilitado");
            }
            tauri::async_runtime::block_on(reconcile_startup_server_states(repo.clone()))
                .expect("No se pudieron reconciliar los estados de servidores");

            let backup_mgr = FileBackupManager::new(
                loaded_settings.backups_dir.clone(),
                repo.clone(),
                backup_repo.clone(),
            );
            let mod_mgr = FileModManager::new(repo.clone(), mod_repo.clone());
            let modpack_inst = ModpackInstaller::new(repo.clone(), modpack_repo.clone());
            tauri::async_runtime::block_on(backup_mgr.restart_auto_backup(
                loaded_settings.backup_interval_secs,
                loaded_settings.servers_dir.clone(),
            ));

            let event_bus = EventBus::new(app.handle().clone());
            app.manage(AppState {
                repo,
                fs,
                console,
                process_mgr,
                resources,
                backup_repo,
                backup_mgr,
                mod_repo,
                mod_mgr,
                modpack_repo,
                modpack_inst,
                network,
                java_mgr,
                downloader: downloader.clone(),
                event_bus,
                settings_store,
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
            commands::restart_server,
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

async fn reconcile_startup_server_states(
    repo: Arc<dyn ServerRepository>,
) -> Result<(), cubed_application::error::ApplicationError> {
    for mut server in repo.find_all().await? {
        if matches!(
            server.status(),
            ServerStatus::Starting | ServerStatus::Running | ServerStatus::Stopping
        ) {
            tracing::warn!(
                server_id = %server.id(),
                "Estado de servidor reconciliado a offline al iniciar Cubed"
            );
            server.recover_as_offline();
            repo.save(&server).await?;
        }
    }
    Ok(())
}

fn default_settings_for_data_dir(data_dir: &Path) -> Settings {
    Settings {
        servers_dir: data_dir.join("servers").to_string_lossy().to_string(),
        backups_dir: data_dir.join("backups").to_string_lossy().to_string(),
        downloads_dir: data_dir.join("downloads").to_string_lossy().to_string(),
        ..Settings::default()
    }
}

fn uses_builtin_storage_defaults(settings: &Settings) -> bool {
    let defaults = Settings::default();
    settings.servers_dir == defaults.servers_dir
        && settings.backups_dir == defaults.backups_dir
        && settings.downloads_dir == defaults.downloads_dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_data_defaults_replace_builtin_storage_dirs() {
        let data_dir = Path::new("/tmp/cubed-app-data");
        let settings = default_settings_for_data_dir(data_dir);

        assert_eq!(settings.servers_dir, "/tmp/cubed-app-data/servers");
        assert_eq!(settings.backups_dir, "/tmp/cubed-app-data/backups");
        assert_eq!(settings.downloads_dir, "/tmp/cubed-app-data/downloads");
        assert_eq!(settings.backup_interval_secs, 18_000);
    }

    #[test]
    fn builtin_storage_defaults_are_detected() {
        assert!(uses_builtin_storage_defaults(&Settings::default()));

        let custom = Settings {
            servers_dir: "/tmp/cubed/servers".into(),
            ..Settings::default()
        };
        assert!(!uses_builtin_storage_defaults(&custom));
    }
}
