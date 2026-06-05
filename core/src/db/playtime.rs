use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytimeRow {
    pub game_id:       String,
    pub session_start: i64,
    pub session_end:   Option<i64>,
    pub duration_secs: Option<i64>,
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub async fn start_session(pool: &SqlitePool, game_id: &str) -> Result<i64> {
    let now = now_ts();
    sqlx::query("INSERT INTO playtime (game_id, session_start) VALUES (?1, ?2)")
        .bind(game_id).bind(now).execute(pool).await?;
    Ok(now)
}

pub async fn end_session(pool: &SqlitePool, game_id: &str, start: i64) -> Result<()> {
    let now = now_ts();
    let dur = now - start;
    sqlx::query(
        "UPDATE playtime SET session_end = ?1, duration_secs = ?2 WHERE game_id = ?3 AND session_start = ?4"
    ).bind(now).bind(dur).bind(game_id).bind(start).execute(pool).await?;
    Ok(())
}

pub async fn total_seconds(pool: &SqlitePool, game_id: &str) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(duration_secs), 0) FROM playtime WHERE game_id = ?1"
    ).bind(game_id).fetch_one(pool).await?;
    Ok(row.0)
}
