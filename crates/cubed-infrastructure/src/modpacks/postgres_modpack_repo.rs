use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::ModpackRepository;
use cubed_domain::entities::{Modpack, ModpackFormat};

#[derive(Debug, FromRow)]
struct ModpackRow {
    id: Uuid,
    server_id: Uuid,
    name: String,
    format: String,
    source_path: String,
}

impl ModpackRow {
    fn into_domain(self) -> ApplicationResult<Modpack> {
        let format = match self.format.as_str() {
            ".mrpack" | "mrpack" => ModpackFormat::Mrpack,
            ".zip" | "zip" => ModpackFormat::Zip,
            other => {
                return Err(ApplicationError::Infrastructure(format!(
                    "Formato de modpack desconocido en base de datos: {}",
                    other
                )));
            }
        };
        Ok(Modpack::reconstitute(
            self.id,
            self.server_id,
            self.name,
            format,
            self.source_path,
        ))
    }
}

pub struct PostgresModpackRepo {
    pool: PgPool,
}

impl PostgresModpackRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ModpackRepository for PostgresModpackRepo {
    async fn save(&self, modpack: &Modpack) -> ApplicationResult<()> {
        sqlx::query(
            r#"
            INSERT INTO modpacks (id, server_id, name, format, source_path)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                format = EXCLUDED.format,
                source_path = EXCLUDED.source_path
            "#,
        )
        .bind(modpack.id())
        .bind(modpack.server_id())
        .bind(modpack.name())
        .bind(modpack.format().to_string())
        .bind(modpack.source_path())
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Modpack>> {
        let row: Option<ModpackRow> = sqlx::query_as(
            "SELECT id, server_id, name, format, source_path FROM modpacks WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        row.map(ModpackRow::into_domain).transpose()
    }

    async fn find_by_server(&self, server_id: Uuid) -> ApplicationResult<Vec<Modpack>> {
        let rows: Vec<ModpackRow> = sqlx::query_as(
            "SELECT id, server_id, name, format, source_path \
             FROM modpacks WHERE server_id = $1 ORDER BY name",
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        rows.into_iter().map(ModpackRow::into_domain).collect()
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        sqlx::query("DELETE FROM modpacks WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        Ok(())
    }
}
