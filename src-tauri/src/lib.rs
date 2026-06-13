//! Punto de ensamblaje (composition root) de Cubed.
//!
//! Aquí se conectan las capas de Clean Architecture y se registran los
//! comandos Tauri expuestos al frontend. En la Fase 0 solo se expone un
//! comando de salud para verificar el puente Frontend <-> Backend.

/// Comando de diagnóstico: confirma que el backend Rust responde.
#[tauri::command]
fn health_check() -> String {
    format!("Cubed backend OK (domain v{})", cubed_domain::DOMAIN_VERSION)
}

/// Arranca la aplicación Tauri.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![health_check])
        .run(tauri::generate_context!())
        .expect("error al iniciar la aplicación Cubed");
}
