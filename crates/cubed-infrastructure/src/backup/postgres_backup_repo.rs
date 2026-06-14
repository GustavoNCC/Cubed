use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::BackupRepository;
use cubed_domain::entities::Backup;

#[derive(Debug, FromRow)]
struct BackupRow {
    id: Uuid,
    server_id: Uuid,
    path: String,
    size_bytes: i64,
    created_at: DateTime<Utc>,
}

impl BackupRow {
    fn into_domain(self) -> Backup {
        Backup::reconstitute(
            self.id,
            self.server_id,
            self.path,
            self.size_bytes.max(0) as u64,
            self.created_at,
        )
    }
}

pub struct PostgresBackupRepo {
    pool: PgPool,
}

impl PostgresBackupRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BackupRepository for PostgresBackupRepo {
    async fn save(&self, backup: &Backup) -> ApplicationResult<()> {
        sqlx::query(
            r#"
            INSERT INTO backups (id, server_id, path, size_bytes, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                path = EXCLUDED.path,
                size_bytes = EXCLUDED.size_bytes
            "#,
        )
        .bind(backup.id())
        .bind(backup.server_id())
        .bind(backup.path())
        .bind(backup.size_bytes() as i64)
        .bind(backup.created_at())
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Backup>> {
        let row: Option<BackupRow> = sqlx::query_as(
            "SELECT id, server_id, path, size_bytes, created_at FROM backups WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(row.map(BackupRow::into_domain))
    }

    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<Backup>> {
        let rows: Vec<BackupRow> = sqlx::query_as(
            "SELECT id, server_id, path, size_bytes, created_at \
             FROM backups WHERE server_id = $1 ORDER BY created_at DESC",
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(rows.into_iter().map(BackupRow::into_domain).collect())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        sqlx::query("DELETE FROM backups WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(())
    }
}
