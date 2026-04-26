// Check which DB has maps
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    for path in &["data/crystal.db", "Data/crystal.db"] {
        let db_url = format!("sqlite:{}", path);
        let pool = sqlx::SqlitePool::connect(&db_url).await?;
        let map_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM map_infos").fetch_one(&pool).await.unwrap_or(0);
        let item_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_infos").fetch_one(&pool).await.unwrap_or(0);
        println!("{}: maps={} items={}", path, map_count, item_count);
    }
    Ok(())
}
