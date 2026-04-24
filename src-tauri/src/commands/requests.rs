use serde::Deserialize;
use sqlx::SqlitePool;
use tauri::State;
use uuid::Uuid;

use crate::db::models::{Request, RequestHeader, RequestParam};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestUpdate {
    pub name: Option<String>,
    pub method: Option<String>,
    pub url: Option<String>,
    pub body: Option<String>,
    pub body_type: Option<String>,
    pub auth_type: Option<String>,
    pub auth_data: Option<String>,
    pub pre_script: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct KVInput {
    pub key: String,
    pub value: String,
    pub enabled: i32,
}

#[derive(Debug, serde::Serialize)]
pub struct RequestWithDetails {
    #[serde(flatten)]
    pub request: Request,
    pub headers: Vec<RequestHeader>,
    pub params: Vec<RequestParam>,
}

#[tauri::command]
pub async fn list_requests(
    pool: State<'_, SqlitePool>,
    collection_id: String,
) -> Result<Vec<Request>, String> {
    sqlx::query_as::<_, Request>(
        "SELECT * FROM requests WHERE collection_id = ? ORDER BY sort_order ASC",
    )
    .bind(&collection_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_request(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<RequestWithDetails, String> {
    let request = sqlx::query_as::<_, Request>("SELECT * FROM requests WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let headers = sqlx::query_as::<_, RequestHeader>(
        "SELECT * FROM request_headers WHERE request_id = ? ORDER BY sort_order ASC",
    )
    .bind(&id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let params = sqlx::query_as::<_, RequestParam>(
        "SELECT * FROM request_params WHERE request_id = ? ORDER BY sort_order ASC",
    )
    .bind(&id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(RequestWithDetails {
        request,
        headers,
        params,
    })
}

#[tauri::command]
pub async fn create_request(
    pool: State<'_, SqlitePool>,
    collection_id: String,
    name: String,
    method: String,
) -> Result<Request, String> {
    let id = Uuid::new_v4().to_string();

    let max_order: (i32,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order), -1) FROM requests WHERE collection_id = ?",
    )
    .bind(&collection_id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO requests (id, collection_id, name, method, sort_order) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&collection_id)
    .bind(&name)
    .bind(&method)
    .bind(max_order.0 + 1)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, Request>("SELECT * FROM requests WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_request(
    pool: State<'_, SqlitePool>,
    id: String,
    data: RequestUpdate,
) -> Result<Request, String> {
    // Build dynamic update query
    let mut sets: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref name) = data.name {
        sets.push("name = ?".to_string());
        values.push(name.clone());
    }
    if let Some(ref method) = data.method {
        sets.push("method = ?".to_string());
        values.push(method.clone());
    }
    if let Some(ref url) = data.url {
        sets.push("url = ?".to_string());
        values.push(url.clone());
    }
    if let Some(ref body) = data.body {
        sets.push("body = ?".to_string());
        values.push(body.clone());
    }
    if let Some(ref body_type) = data.body_type {
        sets.push("body_type = ?".to_string());
        values.push(body_type.clone());
    }
    if let Some(ref auth_type) = data.auth_type {
        sets.push("auth_type = ?".to_string());
        values.push(auth_type.clone());
    }
    if let Some(ref auth_data) = data.auth_data {
        sets.push("auth_data = ?".to_string());
        values.push(auth_data.clone());
    }
    if let Some(ref pre_script) = data.pre_script {
        sets.push("pre_script = ?".to_string());
        values.push(pre_script.clone());
    }

    if !sets.is_empty() {
        sets.push("updated_at = datetime('now')".to_string());
        let sql = format!("UPDATE requests SET {} WHERE id = ?", sets.join(", "));

        let mut query = sqlx::query(&sql);
        for v in &values {
            query = query.bind(v);
        }
        query = query.bind(&id);

        query
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }

    sqlx::query_as::<_, Request>("SELECT * FROM requests WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_request(pool: State<'_, SqlitePool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM requests WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn duplicate_request(
    pool: State<'_, SqlitePool>,
    id: String,
) -> Result<Request, String> {
    let original = sqlx::query_as::<_, Request>("SELECT * FROM requests WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let new_id = Uuid::new_v4().to_string();

    let max_order: (i32,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order), -1) FROM requests WHERE collection_id = ?",
    )
    .bind(&original.collection_id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO requests (id, collection_id, name, description, method, url, body, body_type, auth_type, auth_data, pre_script, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&new_id)
    .bind(&original.collection_id)
    .bind(format!("{} (copy)", &original.name))
    .bind(&original.description)
    .bind(&original.method)
    .bind(&original.url)
    .bind(&original.body)
    .bind(&original.body_type)
    .bind(&original.auth_type)
    .bind(&original.auth_data)
    .bind(&original.pre_script)
    .bind(max_order.0 + 1)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Duplicate headers
    let headers = sqlx::query_as::<_, RequestHeader>(
        "SELECT * FROM request_headers WHERE request_id = ?",
    )
    .bind(&id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    for h in &headers {
        let hid = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO request_headers (id, request_id, key, value, enabled, sort_order) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&hid)
        .bind(&new_id)
        .bind(&h.key)
        .bind(&h.value)
        .bind(h.enabled)
        .bind(h.sort_order)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    }

    // Duplicate params
    let params = sqlx::query_as::<_, RequestParam>(
        "SELECT * FROM request_params WHERE request_id = ?",
    )
    .bind(&id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    for p in &params {
        let pid = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO request_params (id, request_id, key, value, enabled, sort_order) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&pid)
        .bind(&new_id)
        .bind(&p.key)
        .bind(&p.value)
        .bind(p.enabled)
        .bind(p.sort_order)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query_as::<_, Request>("SELECT * FROM requests WHERE id = ?")
        .bind(&new_id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn move_request(
    pool: State<'_, SqlitePool>,
    id: String,
    target_collection_id: String,
) -> Result<Request, String> {
    let max_order: (i32,) = sqlx::query_as(
        "SELECT COALESCE(MAX(sort_order), -1) FROM requests WHERE collection_id = ?",
    )
    .bind(&target_collection_id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "UPDATE requests SET collection_id = ?, sort_order = ?, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(&target_collection_id)
    .bind(max_order.0 + 1)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query_as::<_, Request>("SELECT * FROM requests WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_request_headers(
    pool: State<'_, SqlitePool>,
    request_id: String,
    headers: Vec<KVInput>,
) -> Result<Vec<RequestHeader>, String> {
    // Delete existing headers
    sqlx::query("DELETE FROM request_headers WHERE request_id = ?")
        .bind(&request_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Insert new headers
    for (i, h) in headers.iter().enumerate() {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO request_headers (id, request_id, key, value, enabled, sort_order) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&request_id)
        .bind(&h.key)
        .bind(&h.value)
        .bind(h.enabled)
        .bind(i as i32)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query_as::<_, RequestHeader>(
        "SELECT * FROM request_headers WHERE request_id = ? ORDER BY sort_order ASC",
    )
    .bind(&request_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_request_params(
    pool: State<'_, SqlitePool>,
    request_id: String,
    params: Vec<KVInput>,
) -> Result<Vec<RequestParam>, String> {
    // Delete existing params
    sqlx::query("DELETE FROM request_params WHERE request_id = ?")
        .bind(&request_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Insert new params
    for (i, p) in params.iter().enumerate() {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO request_params (id, request_id, key, value, enabled, sort_order) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&request_id)
        .bind(&p.key)
        .bind(&p.value)
        .bind(p.enabled)
        .bind(i as i32)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query_as::<_, RequestParam>(
        "SELECT * FROM request_params WHERE request_id = ? ORDER BY sort_order ASC",
    )
    .bind(&request_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())
}
