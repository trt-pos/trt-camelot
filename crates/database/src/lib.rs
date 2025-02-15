use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

pub mod models;

pub async fn create_conn_pool(db: &str) -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .connect(db)
        .await
        .expect("Failed to create pool")
}

#[cfg(test)]
mod test {
    use std::env;
    use super::*;

    #[tokio::test]
    async fn test_connection_pool() {
        dotenvy::dotenv().ok();
        let pool = create_conn_pool(&env::var("DATABASE_URL").unwrap()).await;

        sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
            .execute(&pool)
            .await
            .expect("Failed to create table");
        
        pool.close().await;
    }
}