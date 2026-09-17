// Check which DB has maps
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    for path in &["data/crystal.db", "Data/crystal.db"] {
        let db_url = format!("sqlite:{}", path);
        // FK 连接选项池级禁用（sqlx 默认每条新连接 FK ON，与 db::init_db_pool 对齐）
        let options = db_url
            .parse::<sqlx::sqlite::SqliteConnectOptions>()?
            .foreign_keys(false);
        let pool = sqlx::SqlitePool::connect_with(options).await?;
        let map_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM map_infos")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
        let item_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM item_infos")
            .fetch_one(&pool)
            .await
            .unwrap_or(0);
        println!("{}: maps={} items={}", path, map_count, item_count);
    }
    Ok(())
}
