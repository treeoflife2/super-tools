use portable_pty::MasterPty;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use parking_lot::Mutex;

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProfile {
    pub id: String,
    pub title: String,
    pub purpose: String,
    pub project_path: String,
    pub project_name: String,
    pub claude_session_id: Option<String>,
    pub context_prompt: String,
    pub created_at: String,
    pub last_used_at: String,
    #[serde(default)]
    pub worktree_path: Option<String>,
    #[serde(default)]
    pub worktree_branch: Option<String>,
    #[serde(default)]
    pub skip_permissions: bool,
    #[serde(default)]
    pub git_name: Option<String>,
    #[serde(default)]
    pub git_email: Option<String>,
    #[serde(default)]
    pub contexts: Vec<String>, // attached context snippet names
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    pub profiles: Vec<SessionProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredSession {
    pub session_id: String,
    pub modified_at: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsage {
    pub input_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_context_tokens: u64,
    pub context_window: u64,
    pub fill_percent: f64,
    pub model: String,
    pub compacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePlugin {
    pub name: String,
    pub marketplace: String,
    pub enabled: bool,
    pub version: Option<String>,
    pub install_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplacePlugin {
    pub name: String,
    pub description: String,
    pub marketplace: String,
    pub category: Option<String>,
    pub installed: bool,
    pub installs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputPayload {
    pub terminal_id: String,
    pub data: String, // base64 encoded
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileChange {
    pub path: String,
    pub status: String, // "M" modified, "A" added, "D" deleted, "?" untracked, "R" renamed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageAnalytics {
    pub total_cost: f64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_write_tokens: u64,
    pub total_sessions: u32,
    pub total_api_calls: u32,
    pub cache_hit_percent: f64,
    pub daily: Vec<DailyUsage>,
    pub by_model: Vec<ModelUsage>,
    pub by_project: Vec<ProjectUsage>,
    pub top_sessions: Vec<SessionCost>,
    pub tools: Vec<ToolCount>,
    pub shell_commands: Vec<ToolCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub cost: f64,
    pub calls: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    pub model: String,
    pub cost: f64,
    pub calls: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_hit_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUsage {
    pub project: String,
    pub cost: f64,
    pub sessions: u32,
    pub calls: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCost {
    pub session_id: String,
    pub project: String,
    pub cost: f64,
    pub calls: u32,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCount {
    pub name: String,
    pub count: u32,
}

// ---------------------------------------------------------------------------
// Terminal state
// ---------------------------------------------------------------------------

pub(crate) struct TerminalEntry {
    pub(crate) master: Box<dyn MasterPty + Send>,
    pub(crate) writer: Box<dyn Write + Send>,
    #[allow(dead_code)]
    pub(crate) child: Box<dyn portable_pty::Child + Send>,
}

pub struct TerminalState {
    pub(crate) terminals: Arc<Mutex<HashMap<String, TerminalEntry>>>,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
