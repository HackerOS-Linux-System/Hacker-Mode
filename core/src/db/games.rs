use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use sqlx::Row;
use uuid::Uuid;

use crate::store::StoreSource;
use crate::runner::RunnerType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRow {
    pub id:             String,
    pub store_id:       String,
    pub source:         String,
    pub title:          String,
    pub developer:      Option<String>,
    pub publisher:      Option<String>,
    pub install_path:   Option<String>,
    pub executable:     Option<String>,
    pub cover_path:     Option<String>,
    pub hero_path:      Option<String>,
    pub size_bytes:     Option<i64>,
    pub runner:         String,
    pub proton_version: Option<String>,
    pub wine_prefix:    Option<String>,
    pub is_installed:   bool,
    pub is_favourite:   bool,
    pub last_played:    Option<i64>,
    pub added_at:       i64,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for GameRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        Ok(Self {
            id:             row.try_get("id")?,
            store_id:       row.try_get("store_id")?,
            source:         row.try_get("source")?,
            title:          row.try_get("title")?,
            developer:      row.try_get("developer")?,
            publisher:      row.try_get("publisher")?,
            install_path:   row.try_get("install_path")?,
            executable:     row.try_get("executable")?,
            cover_path:     row.try_get("cover_path")?,
            hero_path:      row.try_get("hero_path")?,
            size_bytes:     row.try_get("size_bytes")?,
            runner:         row.try_get("runner")?,
            proton_version: row.try_get("proton_version")?,
            wine_prefix:    row.try_get("wine_prefix")?,
            is_installed:   row.try_get::<i64, _>("is_installed")? != 0,
            is_favourite:   row.try_get::<i64, _>("is_favourite")? != 0,
            last_played:    row.try_get("last_played")?,
            added_at:       row.try_get("added_at")?,
        })
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl GameRow {
    pub fn new(
        store_id: impl Into<String>,
        source: StoreSource,
        title: impl Into<String>,
        runner: &RunnerType,
    ) -> Self {
        Self {
            id:             Uuid::new_v4().to_string(),
            store_id:       store_id.into(),
            source:         source.as_str().to_owned(),
            title:          title.into(),
            developer:      None,
            publisher:      None,
            install_path:   None,
            executable:     None,
            cover_path:     None,
            hero_path:      None,
            size_bytes:     None,
            runner:         serde_json::to_string(runner).unwrap_or_default(),
            proton_version: None,
            wine_prefix:    None,
            is_installed:   false,
            is_favourite:   false,
            last_played:    None,
            added_at:       now_ts(),
        }
    }

    pub fn runner_type(&self) -> Option<RunnerType> {
        serde_json::from_str(&self.runner).ok()
    }
}

pub async fn upsert(pool: &SqlitePool, g: &GameRow) -> Result<()> {
    sqlx::query(
        r#"INSERT INTO games (
            id, store_id, source, title, developer, publisher,
            install_path, executable, cover_path, hero_path, size_bytes,
            runner, proton_version, wine_prefix,
            is_installed, is_favourite, last_played, added_at
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)
        ON CONFLICT(store_id, source) DO UPDATE SET
            title          = excluded.title,
            developer      = excluded.developer,
            publisher      = excluded.publisher,
            install_path   = excluded.install_path,
            executable     = excluded.executable,
            cover_path     = COALESCE(excluded.cover_path, cover_path),
            hero_path      = COALESCE(excluded.hero_path,  hero_path),
            size_bytes     = excluded.size_bytes,
            runner         = excluded.runner,
            proton_version = excluded.proton_version,
            wine_prefix    = excluded.wine_prefix,
            is_installed   = excluded.is_installed"#,
    )
    .bind(&g.id).bind(&g.store_id).bind(&g.source).bind(&g.title)
    .bind(&g.developer).bind(&g.publisher)
    .bind(&g.install_path).bind(&g.executable)
    .bind(&g.cover_path).bind(&g.hero_path).bind(g.size_bytes)
    .bind(&g.runner).bind(&g.proton_version).bind(&g.wine_prefix)
    .bind(g.is_installed as i64).bind(g.is_favourite as i64)
    .bind(g.last_played).bind(g.added_at)
    .execute(pool)
    .await?;
    Ok(())
}

const SELECT_ALL: &str = r#"SELECT id, store_id, source, title, developer, publisher,
    install_path, executable, cover_path, hero_path, size_bytes,
    runner, proton_version, wine_prefix,
    is_installed, is_favourite, last_played, added_at FROM games"#;

pub async fn all(pool: &SqlitePool) -> Result<Vec<GameRow>> {
    Ok(sqlx::query_as::<_, GameRow>(
        &format!("{SELECT_ALL} ORDER BY last_played DESC NULLS LAST, title ASC")
    ).fetch_all(pool).await?)
}

pub async fn by_source(pool: &SqlitePool, source: StoreSource) -> Result<Vec<GameRow>> {
    Ok(sqlx::query_as::<_, GameRow>(
        &format!("{SELECT_ALL} WHERE source = ?1 ORDER BY title ASC")
    ).bind(source.as_str()).fetch_all(pool).await?)
}

pub async fn by_id(pool: &SqlitePool, id: &str) -> Result<Option<GameRow>> {
    Ok(sqlx::query_as::<_, GameRow>(
        &format!("{SELECT_ALL} WHERE id = ?1")
    ).bind(id).fetch_optional(pool).await?)
}

pub async fn set_last_played(pool: &SqlitePool, id: &str) -> Result<()> {
    let now = now_ts();
    sqlx::query("UPDATE games SET last_played = ?1 WHERE id = ?2")
        .bind(now).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn set_favourite(pool: &SqlitePool, id: &str, fav: bool) -> Result<()> {
    sqlx::query("UPDATE games SET is_favourite = ?1 WHERE id = ?2")
        .bind(fav as i64).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM games WHERE id = ?1")
        .bind(id).execute(pool).await?;
    Ok(())
}
