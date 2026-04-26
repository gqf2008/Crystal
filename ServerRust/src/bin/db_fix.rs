// DB fix tool — adds missing columns to migrate_mirdb-created tables
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = crystal_server::db::init_db_pool("sqlite:data/crystal.db").await?;
    println!("DB fix complete — init_db_pool ran with all migrations.");
    println!("Tables:");
    let tables = [
        ("map_infos", "地图"),
        ("item_infos", "物品"),
        ("monster_infos", "怪物"),
        ("npc_infos", "NPC"),
        ("magic_infos", "魔法"),
    ];
    for (table, name) in &tables {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
            .fetch_one(&pool).await?;
        println!("  {}: {} rows", name, count);
    }
    Ok(())
}
