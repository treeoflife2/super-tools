use sqlx::SqlitePool;
use tauri::State;

use super::types::{AiProviderStat, AiUsageStat};

#[tauri::command]
pub async fn get_ai_usage_stats(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<AiUsageStat>, String> {
    let stats = sqlx::query_as::<_, (String, i64, i64, i64)>(
        "SELECT mode, COUNT(*) as total_calls, COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0) FROM ai_usage GROUP BY mode"
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(stats
        .into_iter()
        .map(|(mode, total_calls, input_tokens, output_tokens)| AiUsageStat {
            mode,
            total_calls,
            input_tokens,
            output_tokens,
        })
        .collect())
}

#[tauri::command]
pub async fn get_ai_provider_stats(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<AiProviderStat>, String> {
    let stats = sqlx::query_as::<_, (String, i64, i64, i64)>(
        "SELECT model, COUNT(*) as total_calls, COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0) FROM ai_usage GROUP BY model"
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(stats
        .into_iter()
        .map(|(model, total_calls, input_tokens, output_tokens)| AiProviderStat {
            model,
            total_calls,
            input_tokens,
            output_tokens,
        })
        .collect())
}

#[tauri::command]
pub async fn reset_ai_usage(pool: State<'_, SqlitePool>) -> Result<(), String> {
    sqlx::query("DELETE FROM ai_usage")
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn record_ai_usage(
    pool: State<'_, SqlitePool>,
    mode: String,
    model: String,
    input_tokens: i64,
    output_tokens: i64,
) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO ai_usage (id, mode, model, input_tokens, output_tokens) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&mode)
    .bind(&model)
    .bind(input_tokens)
    .bind(output_tokens)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
