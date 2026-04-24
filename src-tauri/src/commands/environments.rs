use sqlx::SqlitePool;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

use crate::db::models::{EnvVariable, Environment};

#[tauri::command]
pub async fn list_environments(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<Environment>, String> {
    sqlx::query_as::<_, Environment>(
        "SELECT * FROM environments ORDER BY sort_order ASC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_environment(
    pool: State<'_, SqlitePool>,
    name: String,
    color: String,
) -> Result<Environment, String> {
    let id = Uuid::new_v4().to_string();

    let max_order: (i32,) =
        sqlx::query_as("SELECT COALESCE(MAX(sort_order), -1) FROM environments")
            .fetch_one(pool.inner())
            .await
            .map_err(|e| e.to_string())?;

    // Auto-set as default if no environments exist yet
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM environments")
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    let is_default = if count.0 == 0 { 1 } else { 0 };

    sqlx::query(
        "INSERT INTO environments (id, name, color, is_default, sort_order) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&color)
    .bind(is_default)
    .bind(max_order.0 + 1)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, Environment>("SELECT * FROM environments WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_environment(
    pool: State<'_, SqlitePool>,
    id: String,
    name: String,
    color: String,
) -> Result<Environment, String> {
    sqlx::query(
        "UPDATE environments SET name = ?, color = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&name)
    .bind(&color)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, Environment>("SELECT * FROM environments WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_environment(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    let env = sqlx::query_as::<_, Environment>("SELECT * FROM environments WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Delete the environment
    sqlx::query("DELETE FROM environments WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // If it was the default, promote another env to default (if any remain)
    if env.is_default == 1 {
        sqlx::query(
            "UPDATE environments SET is_default = 1 WHERE id = (SELECT id FROM environments ORDER BY sort_order ASC LIMIT 1)"
        )
        .execute(pool.inner())
        .await
        .ok(); // No-op if no environments remain
    }

    Ok(())
}

#[tauri::command]
pub async fn set_default_environment(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    // Set is_default=0 on all environments
    sqlx::query("UPDATE environments SET is_default = 0")
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Set is_default=1 on the specified one
    sqlx::query("UPDATE environments SET is_default = 1 WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn list_env_variables(
    pool: State<'_, SqlitePool>,
    environment_id: String,
) -> Result<Vec<EnvVariable>, String> {
    sqlx::query_as::<_, EnvVariable>(
        "SELECT * FROM env_variables WHERE environment_id = ? ORDER BY sort_order ASC",
    )
    .bind(&environment_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_env_variable(
    pool: State<'_, SqlitePool>,
    environment_id: String,
    key: String,
    value: String,
    is_secret: i32,
) -> Result<EnvVariable, String> {
    let id = Uuid::new_v4().to_string();

    let max_order: (i32,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order), -1) FROM env_variables WHERE environment_id = ?",
    )
    .bind(&environment_id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Upsert: try to find existing variable with same environment_id + key
    let existing: Option<EnvVariable> = sqlx::query_as::<_, EnvVariable>(
        "SELECT * FROM env_variables WHERE environment_id = ? AND key = ?",
    )
    .bind(&environment_id)
    .bind(&key)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let final_id = if let Some(existing) = existing {
        sqlx::query(
            "UPDATE env_variables SET value = ?, is_secret = ? WHERE id = ?",
        )
        .bind(&value)
        .bind(is_secret)
        .bind(&existing.id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
        existing.id
    } else {
        sqlx::query(
            "INSERT INTO env_variables (id, environment_id, key, value, is_secret, sort_order) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&environment_id)
        .bind(&key)
        .bind(&value)
        .bind(is_secret)
        .bind(max_order.0 + 1)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
        id
    };

    sqlx::query_as::<_, EnvVariable>("SELECT * FROM env_variables WHERE id = ?")
        .bind(&final_id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_env_variable(
    pool: State<'_, SqlitePool>,
    id: String,
    key: String,
    value: String,
    is_secret: i32,
) -> Result<EnvVariable, String> {
    sqlx::query(
        "UPDATE env_variables SET key = ?, value = ?, is_secret = ? WHERE id = ?",
    )
    .bind(&key)
    .bind(&value)
    .bind(is_secret)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, EnvVariable>("SELECT * FROM env_variables WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_env_variable(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM env_variables WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_env_variables_for_resolution(
    pool: State<'_, SqlitePool>,
    environment_id: String,
) -> Result<HashMap<String, String>, String> {
    let vars = sqlx::query_as::<_, EnvVariable>(
        "SELECT * FROM env_variables WHERE environment_id = ?",
    )
    .bind(&environment_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let mut map = HashMap::new();
    for var in vars {
        map.insert(var.key, var.value);
    }
    Ok(map)
}

