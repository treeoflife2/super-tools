use sqlx::SqlitePool;
use tauri::State;

use crate::db::models::HistoryEntry;

#[tauri::command]
pub async fn list_history(
    pool: State<'_, SqlitePool>,
    limit: i32,
) -> Result<Vec<HistoryEntry>, String> {
    sqlx::query_as::<_, HistoryEntry>(
        "SELECT * FROM history ORDER BY created_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_history(pool: State<'_, SqlitePool>) -> Result<(), String> {
    sqlx::query("DELETE FROM history")
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_history_entry(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM history WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
