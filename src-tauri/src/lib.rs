mod commands;
mod in_memory_repo;

use std::sync::Arc;
use commands::AppState;
use cubed_infrastructure::LocalFileSystem;

#[tauri::command]
fn health_check() -> String {
    format!("Cubed backend OK (domain v{})", cubed_domain::DOMAIN_VERSION)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let repo = in_memory_repo::InMemoryServerRepo::new();
    let fs   = Arc::new(LocalFileSystem::new("/tmp/cubed-dev"));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState { repo, fs })
        .invoke_handler(tauri::generate_handler![
            health_check,
            commands::list_servers,
            commands::create_server,
            commands::start_server,
            commands::stop_server,
            commands::delete_server,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación Cubed");
}
