use serde::{Deserialize, Serialize};

pub const MEMORY_MB_MIN: u32 = 4_096;
pub const MEMORY_MB_MAX: u32 = 12_288;

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
    /// RAM asignada a los servidores jar (Paper, Purpur, Fabric sin run.sh).
    /// Rango válido: 4096–12288 MB. Los servidores Forge/NeoForge usan user_jvm_args.txt.
    #[serde(default = "Settings::default_memory_mb")]
    pub memory_mb: u32,
}

impl Settings {
    pub fn default_memory_mb() -> u32 {
        4_096
    }

    pub fn validate_memory_mb(mb: u32) -> Result<(), String> {
        if mb < MEMORY_MB_MIN || mb > MEMORY_MB_MAX {
            return Err(format!(
                "La RAM debe estar entre {} MB (4 GB) y {} MB (12 GB), se recibió {} MB",
                MEMORY_MB_MIN, MEMORY_MB_MAX, mb
            ));
        }
        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            servers_dir: "/home/cubed/servers".into(),
            backups_dir: "/home/cubed/backups".into(),
            downloads_dir: "/home/cubed/downloads".into(),
            default_java_path: "/usr/bin/java".into(),
            backup_interval_secs: 18_000,
            memory_mb: Self::default_memory_mb(),
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
