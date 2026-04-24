use sqlx::SqlitePool;
use tauri::State;
use uuid::Uuid;

use crate::db::models::Collection;

#[tauri::command]
pub async fn list_collections(pool: State<'_, SqlitePool>) -> Result<Vec<Collection>, String> {
    sqlx::query_as::<_, Collection>("SELECT * FROM collections ORDER BY sort_order ASC")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_collection(
    pool: State<'_, SqlitePool>,
    name: String,
) -> Result<Collection, String> {
    let id = Uuid::new_v4().to_string();

    let max_order: (i32,) =
        sqlx::query_as("SELECT COALESCE(MAX(sort_order), -1) FROM collections")
            .fetch_one(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

    sqlx::query("INSERT INTO collections (id, name, sort_order) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&name)
        .bind(max_order.0 + 1)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, Collection>("SELECT * FROM collections WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_collection(
    pool: State<'_, SqlitePool>,
    id: String,
    name: String,
    env_id: Option<String>,
) -> Result<Collection, String> {
    sqlx::query(
        "UPDATE collections SET name = ?, env_id = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&name)
    .bind(&env_id)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, Collection>("SELECT * FROM collections WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_collection(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn reorder_collections(
    pool: State<'_, SqlitePool>,
    ids: Vec<String>,
) -> Result<(), String> {
    for (i, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE collections SET sort_order = ? WHERE id = ?")
            .bind(i as i32)
            .bind(id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
