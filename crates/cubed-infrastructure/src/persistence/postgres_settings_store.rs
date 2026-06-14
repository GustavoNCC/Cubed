use sqlx::{FromRow, PgPool};

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_domain::entities::Settings;

#[derive(Debug, FromRow)]
struct SettingsRow {
    servers_dir: String,
    backups_dir: String,
    downloads_dir: String,
    default_java_path: String,
    backup_interval_secs: i64,
}

impl SettingsRow {
    fn into_domain(self) -> Settings {
        Settings {
            servers_dir: self.servers_dir,
            backups_dir: self.backups_dir,
            downloads_dir: self.downloads_dir,
            default_java_path: self.default_java_path,
            backup_interval_secs: self.backup_interval_secs.max(0) as u64,
        }
    }
}

pub struct PostgresSettingsStore {
    pool: PgPool,
}

impl PostgresSettingsStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn load_or_default(&self) -> ApplicationResult<Settings> {
        sqlx::query("INSERT INTO settings DEFAULT VALUES ON CONFLICT DO NOTHING")
            .execute(&self.pool)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        let row: SettingsRow = sqlx::query_as(
            "SELECT servers_dir, backups_dir, downloads_dir, default_java_path, backup_interval_secs \
             FROM settings WHERE id = 1",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        Ok(row.into_domain())
    }

    pub async fn save(&self, settings: &Settings) -> ApplicationResult<()> {
        sqlx::query(
            r#"
            INSERT INTO settings (
                id, servers_dir, backups_dir, downloads_dir, default_java_path, backup_interval_secs
            )
            VALUES (1, $1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                servers_dir = EXCLUDED.servers_dir,
                backups_dir = EXCLUDED.backups_dir,
                downloads_dir = EXCLUDED.downloads_dir,
                default_java_path = EXCLUDED.default_java_path,
                backup_interval_secs = EXCLUDED.backup_interval_secs
            "#,
        )
        .bind(&settings.servers_dir)
        .bind(&settings.backups_dir)
        .bind(&settings.downloads_dir)
        .bind(&settings.default_java_path)
        .bind(settings.backup_interval_secs as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(())
    }
}
