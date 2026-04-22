use crate::models::{ContextUsage, DiscoveredSession, SessionProfile, TokenUsage};
use crate::storage::{encode_project_path, load_profiles, save_profiles};
use std::fs;
use std::path::PathBuf;
use serde_json;
use dirs;

/// Get all profiles — no auto-discovery of session IDs
/// Session IDs are only set when explicitly captured after a session starts
#[tauri::command]
pub fn refresh_session_ids() -> Result<Vec<SessionProfile>, String> {
    Ok(load_profiles())
}

/// Update the claude session ID for a specific profile
#[tauri::command]
pub fn update_session_id(id: String, claude_session_id: String) -> Result<(), String> {
    let mut profiles = load_profiles();
    if let Some(profile) = profiles.iter_mut().find(|p| p.id == id) {
        if claude_session_id.is_empty() {
            profile.claude_session_id = None;
        } else {
            profile.claude_session_id = Some(claude_session_id);
        }
    } else {
        return Err("Profile not found".to_string());
    }
    save_profiles(&profiles)?;
    Ok(())
}

/// Count active sessions for a project path (profiles that have been used)
#[tauri::command]
pub fn count_project_sessions(project_path: String) -> Result<u32, String> {
    let profiles = load_profiles();
    let count = profiles.iter()
        .filter(|p| p.project_path == project_path)
        .count() as u32;
    Ok(count)
}

#[tauri::command]
pub fn discover_sessions(project_path: String) -> Result<Vec<DiscoveredSession>, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let encoded = encode_project_path(&project_path);
    let projects_dir = home.join(".claude").join("projects").join(&encoded);

    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    let entries = fs::read_dir(&projects_dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            let session_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let modified_at = path
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let datetime: chrono::DateTime<chrono::Utc> = t.into();
                    datetime.to_rfc3339()
                })
                .unwrap_or_default();

            // Extract first user message as preview
            let preview = fs::read_to_string(&path).ok().and_then(|content| {
                for line in content.lines().take(20) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                        if val.get("type").and_then(|t| t.as_str()) == Some("human") {
                            if let Some(msg) = val.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
                                let trimmed = msg.chars().take(80).collect::<String>();
                                return Some(trimmed);
                            }
                        }
                    }
                }
                None
            });

            sessions.push(DiscoveredSession {
                session_id,
                modified_at,
                preview,
            });
        }
    }

    sessions.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(sessions)
}

#[tauri::command]
pub fn get_session_tokens(
    project_path: String,
    session_id: Option<String>,
) -> Result<TokenUsage, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let encoded = encode_project_path(&project_path);
    let projects_dir = home.join(".claude").join("projects").join(&encoded);

    if !projects_dir.exists() {
        return Err("Project directory not found".to_string());
    }

    let file_path = if let Some(sid) = session_id {
        projects_dir.join(format!("{}.jsonl", sid))
    } else {
        // Find most recent .jsonl file
        let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    if let Ok(meta) = path.metadata() {
                        if let Ok(modified) = meta.modified() {
                            if best.as_ref().map_or(true, |(_, t)| modified > *t) {
                                best = Some((path, modified));
                            }
                        }
                    }
                }
            }
        }
        best.map(|(p, _)| p)
            .ok_or("No session files found")?
    };

    if !file_path.exists() {
        return Err("Session file not found".to_string());
    }

    let contents = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;

    let mut input_tokens: u64 = 0;
    let mut output_tokens: u64 = 0;
    let mut cache_read_tokens: u64 = 0;
    let mut cache_creation_tokens: u64 = 0;

    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            // Check both direct usage and message.usage patterns
            let usage = val.get("usage").or_else(|| {
                val.get("message").and_then(|m| m.get("usage"))
            });
            if let Some(u) = usage {
                input_tokens += u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                output_tokens += u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                cache_read_tokens += u.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                cache_creation_tokens += u.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
            }
        }
    }

    let total_tokens = input_tokens + output_tokens + cache_read_tokens + cache_creation_tokens;

    Ok(TokenUsage {
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_creation_tokens,
        total_tokens,
    })
}

/// Get the context window usage for a session by reading the last assistant entry
/// from the .jsonl file. Returns fill percentage and compaction detection.
#[tauri::command]
pub fn get_session_context_usage(
    project_path: String,
    session_id: String,
) -> Result<ContextUsage, String> {
    let home = dirs::home_dir().ok_or("Cannot determine home directory")?;
    let encoded = encode_project_path(&project_path);
    let file_path = home
        .join(".claude")
        .join("projects")
        .join(&encoded)
        .join(format!("{}.jsonl", session_id));

    if !file_path.exists() {
        return Err("Session file not found".to_string());
    }

    // Read from the end for efficiency — find last two assistant entries
    let contents = fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    let lines: Vec<&str> = contents.lines().collect();

    let mut last_usage: Option<(u64, u64, u64, String)> = None; // (input, cache_read, cache_create, model)
    let mut prev_total: Option<u64> = None;
    let mut found_last = false;

    // Iterate from the end to find the last two assistant entries
    for line in lines.iter().rev() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            let is_assistant = val.get("type").and_then(|t| t.as_str()) == Some("assistant");
            if !is_assistant {
                continue;
            }

            let usage = val.get("message").and_then(|m| m.get("usage"));
            if let Some(u) = usage {
                let input = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let cache_read = u.get("cache_read_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let cache_create = u.get("cache_creation_input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                let total = input + cache_read + cache_create;

                let model = val.get("message")
                    .and_then(|m| m.get("model"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                if !found_last {
                    last_usage = Some((input, cache_read, cache_create, model));
                    found_last = true;
                } else {
                    prev_total = Some(total);
                    break;
                }
            }
        }
    }

    let (input_tokens, cache_read_tokens, cache_creation_tokens, model) =
        last_usage.unwrap_or((0, 0, 0, "unknown".to_string()));

    let total_context_tokens = input_tokens + cache_read_tokens + cache_creation_tokens;

    // Determine context window size from model name
    let context_window: u64 = if model.contains("opus") {
        1_000_000
    } else if model.contains("sonnet") {
        200_000
    } else if model.contains("haiku") {
        200_000
    } else {
        200_000 // safe default
    };

    let fill_percent = if context_window > 0 {
        (total_context_tokens as f64 / context_window as f64) * 100.0
    } else {
        0.0
    };

    // Detect compaction: previous total was >50% higher than current
    let compacted = if let Some(prev) = prev_total {
        prev > 0 && total_context_tokens < prev / 2
    } else {
        false
    };

    Ok(ContextUsage {
        input_tokens,
        cache_read_tokens,
        cache_creation_tokens,
        total_context_tokens,
        context_window,
        fill_percent,
        model,
        compacted,
    })
}

/// Update contexts attached to a session profile
#[tauri::command]
pub fn update_session_contexts(id: String, contexts: Vec<String>) -> Result<(), String> {
    let mut profiles = load_profiles();
    if let Some(profile) = profiles.iter_mut().find(|p| p.id == id) {
        profile.contexts = contexts;
    } else {
        return Err("Profile not found".to_string());
    }
    save_profiles(&profiles)?;
    Ok(())
}
