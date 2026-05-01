// ClickHouse HTTP client.
//
// ClickHouse is OLAP and HTTP-based, not sqlx-compatible, so this driver
// lives outside the sqlx pool model. It speaks ClickHouse's HTTP interface
// directly (POST /?database=...&user=...&password=...) and uses the
// `FORMAT JSON` response shape, which returns:
//
//   { "meta": [{"name": ..., "type": ...}, ...],
//     "data": [{"col": value, ...}, ...],
//     "rows": N, ... }
//
// Each `ClickhouseClient` is a stateless wrapper over a `reqwest::Client`,
// the base URL, and credentials. Connection-test happens at "connect" time
// by issuing a `SELECT 1`. There is no persistent connection to close.
//
// `reqwest` is already a workspace dependency (REST mode + tauri-plugin-http
// pull it in), so adding ClickHouse here costs zero extra build weight.

use serde::Deserialize;
use std::time::Duration;

use super::client::SqlConnectionConfig;

/// Map ClickHouse error code → short user-facing message.
///
/// Codes are stable across ClickHouse versions (defined in
/// dbms/Common/ErrorCodes.cpp). For unmapped codes, the caller falls back
/// to `clean_clickhouse_error_body` which trims the verbose response body.
fn humanize_clickhouse_code(code: &str) -> Option<&'static str> {
    match code {
        "516" | "192" | "193" => Some("Authentication failed — check username and password"),
        "81" => Some("Database not found — check the database name"),
        "60" => Some("Table not found"),
        "47" => Some("Unknown column"),
        "62" => Some("SQL syntax error"),
        "159" => Some("Query timed out"),
        "164" => Some("Read-only — write operation not allowed"),
        "210" => Some("Network error — check host and port"),
        "241" => Some("Memory limit exceeded"),
        "252" => Some("Too many parts — server is overloaded"),
        "394" => Some("Query was cancelled"),
        _ => None,
    }
}

/// Fallback: trim ClickHouse's verbose error body to the salient first
/// sentence + labelled tag (e.g. `(AUTHENTICATION_FAILED)`). Used when
/// `humanize_clickhouse_code` doesn't have a mapping for the error code.
fn clean_clickhouse_error_body(body: &str) -> String {
    let after_exc = body.splitn(2, "DB::Exception:").nth(1).unwrap_or(body);
    let first = after_exc.splitn(2, ". ").next().unwrap_or(after_exc);
    let trimmed = first.trim().trim_end_matches('.').trim();
    let label = body
        .rsplit('(')
        .filter_map(|s| s.split(')').next())
        .find(|s| s.chars().all(|c| c.is_ascii_uppercase() || c == '_') && s.len() > 3);

    let mut out = trimmed.to_string();
    if out.len() > 220 {
        out.truncate(220);
        out.push('…');
    }
    if let Some(l) = label {
        if !out.contains(l) {
            out.push_str(" (");
            out.push_str(l);
            out.push(')');
        }
    }
    out
}

/// Stateless HTTP client for a single ClickHouse connection. Cloning is
/// cheap; `reqwest::Client` shares its connection pool internally.
#[derive(Debug, Clone)]
pub struct ClickhouseClient {
    http: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
    /// Default database — included in the query URL when set.
    pub database: String,
}

/// Wire shape of `FORMAT JSON` responses.
#[derive(Debug, Deserialize)]
struct ClickhouseJsonResponse {
    #[serde(default)]
    meta: Vec<ClickhouseColumnMeta>,
    #[serde(default)]
    data: Vec<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    rows: u64,
}

#[derive(Debug, Deserialize)]
struct ClickhouseColumnMeta {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
}

/// Parsed query result in the same shape `sql_execute_query` returns for
/// the other drivers (column names + JSON-typed row values).
pub struct ClickhouseQueryRows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub affected: u64,
}

impl ClickhouseClient {
    /// Build a client. `host`/`port` come from `SqlConnectionConfig`;
    /// `ssl=true` switches the URL scheme to https.
    pub fn new(config: &SqlConnectionConfig) -> Result<Self, String> {
        let scheme = if config.ssl { "https" } else { "http" };
        let host = if config.host.is_empty() { "localhost".to_string() } else { config.host.clone() };
        let port = if config.port == 0 { 8123 } else { config.port };
        let base_url = format!("{}://{}:{}", scheme, host, port);

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| format!("ClickHouse client build failed: {}", e))?;

        // ClickHouse defaults: user=default, empty password, database=default.
        let username = if config.username.is_empty() { "default".to_string() } else { config.username.clone() };
        let database = if config.database.is_empty() { "default".to_string() } else { config.database.clone() };

        Ok(Self {
            http,
            base_url,
            username,
            password: config.password.clone(),
            database,
        })
    }

    /// Verify reachability + credentials by issuing a `SELECT 1`.
    pub async fn ping(&self) -> Result<(), String> {
        let _ = self.execute_format_json("SELECT 1").await?;
        Ok(())
    }

    /// Execute an arbitrary SQL statement and return parsed rows. The
    /// statement is appended with `FORMAT JSON` when the input doesn't
    /// already specify a FORMAT clause; statements that don't return rows
    /// (e.g. INSERT, ALTER) come back with empty `meta`/`data` and that's
    /// fine.
    pub async fn query(&self, sql: &str) -> Result<ClickhouseQueryRows, String> {
        let resp = self.execute_format_json(sql).await?;
        Ok(map_response_to_rows(resp))
    }

    /// POST the SQL to ClickHouse and parse the response.
    async fn execute_format_json(&self, sql: &str) -> Result<ClickhouseJsonResponse, String> {
        // Append FORMAT JSON unless the query already has a FORMAT clause.
        let sql_for_send = if has_format_clause(sql) {
            sql.to_string()
        } else {
            format!("{}\nFORMAT JSON", sql.trim_end().trim_end_matches(';'))
        };

        let mut req = self
            .http
            .post(&self.base_url)
            .query(&[("database", self.database.as_str())])
            .header("Content-Type", "text/plain; charset=UTF-8");

        // Send credentials via headers (avoids putting password in URL).
        req = req.header("X-ClickHouse-User", &self.username);
        if !self.password.is_empty() {
            req = req.header("X-ClickHouse-Key", &self.password);
        }

        let response = req
            .body(sql_for_send)
            .send()
            .await
            .map_err(|e| format!("ClickHouse request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            // ClickHouse returns the error in the body; the
            // X-ClickHouse-Exception-Code header holds the numeric code.
            let code = response
                .headers()
                .get("X-ClickHouse-Exception-Code")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = response.text().await.unwrap_or_default();
            // Prefer our hand-written message for known codes; fall back to
            // a trimmed version of ClickHouse's verbose body otherwise.
            return Err(match code.as_deref().and_then(humanize_clickhouse_code) {
                Some(msg) => msg.to_string(),
                None => match code {
                    Some(c) => format!("ClickHouse error (code {}): {}", c, clean_clickhouse_error_body(&body)),
                    None => format!("ClickHouse error ({}): {}", status, clean_clickhouse_error_body(&body)),
                },
            });
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("ClickHouse response read failed: {}", e))?;

        if text.trim().is_empty() {
            // No-row statements (DDL/INSERT) come back empty.
            return Ok(ClickhouseJsonResponse { meta: vec![], data: vec![], rows: 0 });
        }

        serde_json::from_str::<ClickhouseJsonResponse>(&text)
            .map_err(|e| format!("ClickHouse JSON parse failed: {} (body: {})", e, text.chars().take(400).collect::<String>()))
    }
}

fn has_format_clause(sql: &str) -> bool {
    // Naive but good enough — ClickHouse's FORMAT clause is always near the
    // end and case-insensitive. Avoid false positives by requiring it as a
    // distinct word.
    let lower = sql.to_lowercase();
    lower.split_whitespace().any(|tok| tok == "format")
}

/// Convert a ClickHouse JSON response into the column/row shape the
/// existing ResultsTable expects. Each column value is mapped from its
/// declared ClickHouse type to a JSON primitive — matching how the
/// Postgres / MySQL / SQLite paths shape their output.
fn map_response_to_rows(resp: ClickhouseJsonResponse) -> ClickhouseQueryRows {
    let columns: Vec<String> = resp.meta.iter().map(|c| c.name.clone()).collect();
    let types: Vec<String> = resp.meta.iter().map(|c| c.type_name.clone()).collect();

    let rows: Vec<Vec<serde_json::Value>> = resp
        .data
        .iter()
        .map(|obj| {
            columns
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let raw = obj.get(name).cloned().unwrap_or(serde_json::Value::Null);
                    let ty = types.get(i).map(|s| s.as_str()).unwrap_or("");
                    coerce_value_for_type(raw, ty)
                })
                .collect()
        })
        .collect();

    let affected = if resp.rows > 0 { resp.rows } else { rows.len() as u64 };
    ClickhouseQueryRows { columns, rows, affected }
}

/// Map a raw ClickHouse JSON cell into the JSON shape the frontend expects.
///
/// ClickHouse's `FORMAT JSON` already produces sensible JSON (numbers stay
/// numbers, strings stay strings, nulls become null). For very large
/// integer types (`UInt64` / `Int64` / 128-bit ints) ClickHouse emits them
/// as JSON strings to avoid f64 precision loss — we keep them as strings,
/// matching how Postgres NUMERIC is surfaced.
fn coerce_value_for_type(raw: serde_json::Value, ty: &str) -> serde_json::Value {
    if matches!(raw, serde_json::Value::Null) {
        return serde_json::Value::Null;
    }
    // Strip Nullable(T) wrapper for type-name dispatch.
    let inner = strip_nullable(ty);

    // Pass-through for the cases ClickHouse already shapes correctly.
    // Arrays/Tuples/Maps come back as JSON arrays/objects; leave them.
    if inner.starts_with("Array(") || inner.starts_with("Tuple(") || inner.starts_with("Map(") {
        return raw;
    }

    // 64-bit ints + Decimal: ClickHouse stringifies these in JSON.
    // Booleans: ClickHouse Bool type → already JSON bool.
    // Everything else we accept as-is.
    let _ = inner; // reserved for future per-type coercion (Date, UUID, etc.)
    raw
}

fn strip_nullable(ty: &str) -> &str {
    if let Some(rest) = ty.strip_prefix("Nullable(") {
        rest.strip_suffix(')').unwrap_or(rest)
    } else {
        ty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_basic_response_to_rows() {
        let resp = ClickhouseJsonResponse {
            meta: vec![
                ClickhouseColumnMeta { name: "id".to_string(), type_name: "UInt32".to_string() },
                ClickhouseColumnMeta { name: "name".to_string(), type_name: "String".to_string() },
                ClickhouseColumnMeta { name: "amount".to_string(), type_name: "Decimal(10, 2)".to_string() },
            ],
            data: vec![
                {
                    let mut m = serde_json::Map::new();
                    m.insert("id".to_string(), serde_json::json!(1));
                    m.insert("name".to_string(), serde_json::json!("alice"));
                    m.insert("amount".to_string(), serde_json::json!("12.50"));
                    m
                },
                {
                    let mut m = serde_json::Map::new();
                    m.insert("id".to_string(), serde_json::json!(2));
                    m.insert("name".to_string(), serde_json::Value::Null);
                    m.insert("amount".to_string(), serde_json::json!("0.00"));
                    m
                },
            ],
            rows: 2,
        };

        let mapped = map_response_to_rows(resp);
        assert_eq!(mapped.columns, vec!["id", "name", "amount"]);
        assert_eq!(mapped.rows.len(), 2);
        assert_eq!(mapped.rows[0][0], serde_json::json!(1));
        assert_eq!(mapped.rows[0][1], serde_json::json!("alice"));
        assert_eq!(mapped.rows[0][2], serde_json::json!("12.50"));
        assert_eq!(mapped.rows[1][1], serde_json::Value::Null);
        assert_eq!(mapped.affected, 2);
    }

    #[test]
    fn nullable_wrapping_is_stripped() {
        assert_eq!(strip_nullable("Nullable(String)"), "String");
        assert_eq!(strip_nullable("String"), "String");
        assert_eq!(strip_nullable("Nullable(DateTime64(3))"), "DateTime64(3)");
    }

    #[test]
    fn detects_format_clause() {
        assert!(has_format_clause("SELECT 1 FORMAT JSON"));
        assert!(has_format_clause("select 1 format TSV"));
        assert!(!has_format_clause("SELECT 1"));
        assert!(!has_format_clause("SELECT 'format' FROM t"));
    }
}
