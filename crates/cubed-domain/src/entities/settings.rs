use serde::{Deserialize, Serialize};

/// Configuración global de Cubed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Directorio raíz donde se almacenan los servidores.
    pub servers_dir: String,
    /// Directorio raíz para backups.
    pub backups_dir: String,
    /// Directorio para descargas temporales.
    pub downloads_dir: String,
    /// Java por defecto si no se especifica uno por servidor.
    pub default_java_path: String,
    /// Intervalo de backup automático en segundos (0 = desactivado).
    pub backup_interval_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            servers_dir: "/home/cubed/servers".into(),
            backups_dir: "/home/cubed/backups".into(),
            downloads_dir: "/home/cubed/downloads".into(),
            default_java_path: "/usr/bin/java".into(),
            backup_interval_secs: 18_000, // 5 horas
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_backup_interval_is_five_hours() {
        assert_eq!(Settings::default().backup_interval_secs, 18_000);
    }
}
