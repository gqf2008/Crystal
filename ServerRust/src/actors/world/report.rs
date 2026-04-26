/// 举报系统 — 接收玩家举报并存入 DB

use crate::db::DbPool;

/// 保存举报记录到数据库
pub async fn save_report(
    pool: &DbPool,
    reporter_name: &str,
    issue_type: u8,
    description: &str,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO report_logs (reporter_name, issue_type, description, created_at) VALUES (?, ?, ?, ?)"
    )
    .bind(reporter_name)
    .bind(issue_type as i32)
    .bind(description)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}
