use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use cubed_domain::entities::Server;
use cubed_application::error::{ApplicationError, ApplicationResult};
use cubed_application::ports::ServerRepository;

use super::server_row::ServerRow;

pub struct PostgresServerRepository {
    pool: PgPool,
}

impl PostgresServerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServerRepository for PostgresServerRepository {
    async fn save(&self, server: &Server) -> ApplicationResult<()> {
        sqlx::query(
            r#"
            INSERT INTO servers (id, name, version, software, port, java_path, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                name       = EXCLUDED.name,
                version    = EXCLUDED.version,
                software   = EXCLUDED.software,
                port       = EXCLUDED.port,
                java_path  = EXCLUDED.java_path,
                status     = EXCLUDED.status,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(server.id())
        .bind(server.name().as_str())
        .bind(server.version().as_str())
        .bind(server.software().to_string())
        .bind(server.port().value() as i32)
        .bind(server.java_path().as_str())
        .bind(server.status().to_string())
        .bind(server.created_at())
        .bind(server.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> ApplicationResult<Option<Server>> {
        let row: Option<ServerRow> = sqlx::query_as(
            "SELECT id, name, version, software, port, java_path, status, created_at, updated_at \
             FROM servers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        row.map(ServerRow::into_domain).transpose()
    }

    async fn find_all(&self) -> ApplicationResult<Vec<Server>> {
        let rows: Vec<ServerRow> = sqlx::query_as(
            "SELECT id, name, version, software, port, java_path, status, created_at, updated_at \
             FROM servers ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        rows.into_iter().map(ServerRow::into_domain).collect()
    }

    async fn delete(&self, id: Uuid) -> ApplicationResult<()> {
        sqlx::query("DELETE FROM servers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        Ok(())
    }

    async fn port_in_use(&self, port: u16) -> ApplicationResult<bool> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM servers WHERE port = $1")
            .bind(port as i32)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        Ok(row.0 > 0)
    }
}
