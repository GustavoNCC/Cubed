use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::ModRepository;
use cubed_domain::entities::ModEntry;

#[derive(Debug, FromRow)]
struct ModRow {
    id: Uuid,
    server_id: Uuid,
    file_name: String,
    path: String,
}

impl ModRow {
    fn into_domain(self) -> ModEntry {
        ModEntry::reconstitute(self.id, self.server_id, self.file_name, self.path)
    }
}

pub struct PostgresModRepo {
    pool: PgPool,
}

impl PostgresModRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ModRepository for PostgresModRepo {
    async fn save(&self, entry: &ModEntry) -> ApplicationResult<()> {
        sqlx::query(
            r#"
            INSERT INTO mods (id, server_id, file_name, path)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
                file_name = EXCLUDED.file_name,
                path = EXCLUDED.path
            "#,
        )
        .bind(entry.id())
        .bind(entry.server_id())
        .bind(entry.file_name())
        .bind(entry.path())
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<ModEntry>> {
        let row: Option<ModRow> =
            sqlx::query_as("SELECT id, server_id, file_name, path FROM mods WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(row.map(ModRow::into_domain))
    }

    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<ModEntry>> {
        let rows: Vec<ModRow> = sqlx::query_as(
            "SELECT id, server_id, file_name, path FROM mods WHERE server_id = $1 ORDER BY file_name",
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(rows.into_iter().map(ModRow::into_domain).collect())
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        sqlx::query("DELETE FROM mods WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(())
    }
}
