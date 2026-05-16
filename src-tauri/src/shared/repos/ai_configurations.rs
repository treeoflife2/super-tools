use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AiConfiguration {
    pub id: i64,
    pub label: String,
    pub provider: String,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    pub is_default: i64,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfigurationInput {
    pub label: String,
    pub provider: String,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
}

pub async fn list_all(pool: &SqlitePool) -> Result<Vec<AiConfiguration>, sqlx::Error> {
    sqlx::query_as::<_, AiConfiguration>(
        "SELECT id, label, provider, base_url, default_model, is_default, created_at, last_used_at
           FROM ai_configurations
          ORDER BY is_default DESC, last_used_at DESC, created_at DESC",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<Option<AiConfiguration>, sqlx::Error> {
    sqlx::query_as::<_, AiConfiguration>(
        "SELECT id, label, provider, base_url, default_model, is_default, created_at, last_used_at
           FROM ai_configurations WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_default(pool: &SqlitePool) -> Result<Option<AiConfiguration>, sqlx::Error> {
    sqlx::query_as::<_, AiConfiguration>(
        "SELECT id, label, provider, base_url, default_model, is_default, created_at, last_used_at
           FROM ai_configurations WHERE is_default = 1 LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}

pub async fn create(pool: &SqlitePool, input: &AiConfigurationInput) -> Result<i64, sqlx::Error> {
    // First row becomes default automatically.
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM ai_configurations").fetch_one(pool).await?;
    let is_default = if count.0 == 0 { 1 } else { 0 };

    let row: (i64,) = sqlx::query_as(
        "INSERT INTO ai_configurations (label, provider, base_url, default_model, is_default)
         VALUES (?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(&input.label)
    .bind(&input.provider)
    .bind(&input.base_url)
    .bind(&input.default_model)
    .bind(is_default)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    input: &AiConfigurationInput,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE ai_configurations
            SET label = ?, provider = ?, base_url = ?, default_model = ?
          WHERE id = ?",
    )
    .bind(&input.label)
    .bind(&input.provider)
    .bind(&input.base_url)
    .bind(&input.default_model)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // If we're deleting the default, promote another (oldest first) to default.
    let was_default: (i64,) = sqlx::query_as(
        "SELECT COALESCE((SELECT is_default FROM ai_configurations WHERE id = ?), 0)",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM ai_configurations WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    if was_default.0 == 1 {
        sqlx::query(
            "UPDATE ai_configurations SET is_default = 1
              WHERE id = (SELECT id FROM ai_configurations ORDER BY created_at ASC LIMIT 1)",
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn set_default(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE ai_configurations SET is_default = 0").execute(&mut *tx).await?;
    sqlx::query("UPDATE ai_configurations SET is_default = 1 WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn touch_last_used(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE ai_configurations SET last_used_at = datetime('now') WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
