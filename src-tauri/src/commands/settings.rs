use sqlx::SqlitePool;
use std::collections::HashMap;
use tauri::State;

use crate::db::models::Setting;

#[tauri::command]
pub async fn get_setting(
    pool: State<'_, SqlitePool>,
    key: String,
) -> Result<Option<String>, String> {
    let setting = sqlx::query_as::<_, Setting>("SELECT * FROM settings WHERE key = ?")
        .bind(&key)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(setting.map(|s| s.value))
}

#[tauri::command]
pub async fn set_setting(
    pool: State<'_, SqlitePool>,
    key: String,
    value: String,
) -> Result<(), String> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(&key)
        .bind(&value)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_all_settings(
    pool: State<'_, SqlitePool>,
) -> Result<HashMap<String, String>, String> {
    let settings = sqlx::query_as::<_, Setting>("SELECT * FROM settings")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let mut map = HashMap::new();
    for s in settings {
        map.insert(s.key, s.value);
    }
    Ok(map)
}
