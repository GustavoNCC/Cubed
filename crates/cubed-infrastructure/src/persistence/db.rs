use sqlx::{postgres::PgPoolOptions, PgPool};

/// Crea un pool de conexiones a PostgreSQL y ejecuta las migraciones pendientes.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
