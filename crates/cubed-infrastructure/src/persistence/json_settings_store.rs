use std::path::PathBuf;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_domain::entities::Settings;

pub struct JsonSettingsStore {
    path: PathBuf,
}

impl JsonSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load_or_default(&self) -> ApplicationResult<Settings> {
        if !self.path.exists() {
            let settings = Settings::default();
            self.save(&settings)?;
            return Ok(settings);
        }

        let raw = std::fs::read_to_string(&self.path).map_err(|e| {
            ApplicationError::Infrastructure(format!("Error leyendo {}: {e}", self.path.display()))
        })?;
        if raw.trim().is_empty() {
            return Ok(Settings::default());
        }
        serde_json::from_str(&raw).map_err(|e| {
            ApplicationError::Infrastructure(format!(
                "JSON corrupto en {}: {e}",
                self.path.display()
            ))
        })
    }

    pub fn save(&self, settings: &Settings) -> ApplicationResult<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ApplicationError::Infrastructure(format!(
                    "No se pudo crear {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, json)
            .map_err(|e| ApplicationError::Infrastructure(format!("Error escribiendo tmp: {e}")))?;
        std::fs::rename(&tmp, &self.path)
            .map_err(|e| ApplicationError::Infrastructure(format!("Error renombrando tmp: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_or_default_creates_settings_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = JsonSettingsStore::new(path.clone());

        let settings = store.load_or_default().unwrap();

        assert_eq!(settings.backup_interval_secs, 18_000);
        assert!(path.exists());
    }

    #[test]
    fn save_persists_settings() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = JsonSettingsStore::new(path.clone());
        let settings = Settings {
            servers_dir: "/home/cubed/custom-servers".into(),
            backup_interval_secs: 3_600,
            ..Settings::default()
        };

        store.save(&settings).unwrap();
        let reloaded = JsonSettingsStore::new(path).load_or_default().unwrap();

        assert_eq!(reloaded.servers_dir, "/home/cubed/custom-servers");
        assert_eq!(reloaded.backup_interval_secs, 3_600);
    }
}
