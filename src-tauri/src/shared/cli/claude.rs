// Claude CLI implementation of [`CliRunner`].
//
// The literals embedded here used to be scattered across
// `modes/agent/{terminal,plugins,usage}.rs`. Centralising them means a future
// Codex / Gemini / Aider implementation is one new file alongside this one.

use std::path::{Path, PathBuf};

use super::runner::{CliRunner, SpawnOpts};
use crate::shared::platform::shell::default_user_shell;

pub struct ClaudeRunner;

/// The Claude binary name on `$PATH`.
const BINARY: &str = "claude";

/// Sub-directory under `$HOME` that holds Claude's state.
const HOME_SUBDIR: &str = ".claude";

/// Sub-directory under `<home>` that holds installed plugins.
const PLUGINS_SUBDIR: &str = "plugins";

/// Sub-directory under `<home>` that holds per-project session logs.
const SESSIONS_SUBDIR: &str = "projects";

/// Session log file extension (without the dot).
const SESSION_EXT: &str = "jsonl";

impl ClaudeRunner {
    fn dot_claude(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(HOME_SUBDIR))
    }
}

impl CliRunner for ClaudeRunner {
    fn id(&self) -> &'static str {
        "claude"
    }

    fn binary_name(&self) -> &'static str {
        BINARY
    }

    fn resolve_binary_path(&self) -> String {
        // On Unix: spawn the user's shell with `-l -i -c "which claude"` so PATH
        // adjustments from rc files (nvm, fnm, asdf) are visible.
        // On Windows: shell out directly to `where.exe claude` — no shell rc to
        // worry about, and `which` does not exist there.
        let (shell_path, shell_kind) = default_user_shell();
        let (binary, args) = if cfg!(target_os = "windows") {
            ("where.exe".to_string(), vec![BINARY.to_string()])
        } else {
            (
                shell_path.clone(),
                shell_kind.exec_command_argv(&format!("which {}", BINARY)),
            )
        };

        if let Ok(output) = std::process::Command::new(&binary).args(&args).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // `where.exe` may print multiple matches (one per line). Take the first.
                let path = stdout.lines().next().unwrap_or("").trim().to_string();
                if !path.is_empty() {
                    return path;
                }
            }
        }
        BINARY.to_string()
    }

    fn build_spawn_command(&self, opts: &SpawnOpts) -> String {
        let mut cmd = String::from(BINARY);
        if let Some(ref sid) = opts.resume_session_id {
            cmd.push_str(&format!(" --resume \"{}\"", sid));
        }
        if opts.skip_permissions {
            cmd.push_str(" --dangerously-skip-permissions");
        }
        if let Some(ref prompt) = opts.system_prompt {
            if !prompt.is_empty() {
                // Single quotes prevent ALL shell interpretation (< > $ ` etc.).
                // Escape any single quotes in the prompt: ' -> '\''
                let escaped = prompt.replace('\'', "'\\''");
                cmd.push_str(&format!(" --append-system-prompt '{}'", escaped));
            }
        }
        cmd
    }

    fn home_dir(&self) -> Option<PathBuf> {
        self.dot_claude()
    }

    fn plugins_dir(&self) -> Option<PathBuf> {
        self.dot_claude().map(|p| p.join(PLUGINS_SUBDIR))
    }

    fn settings_file(&self) -> Option<PathBuf> {
        self.dot_claude().map(|p| p.join("settings.json"))
    }

    fn installed_plugins_file(&self) -> Option<PathBuf> {
        self.plugins_dir().map(|p| p.join("installed_plugins.json"))
    }

    fn plugin_marketplaces_dir(&self) -> Option<PathBuf> {
        self.plugins_dir().map(|p| p.join("marketplaces"))
    }

    fn plugin_install_counts_file(&self) -> Option<PathBuf> {
        self.plugins_dir().map(|p| p.join("install-counts-cache.json"))
    }

    fn run_plugin_subcommand(&self, args: &[&str]) -> Result<(bool, String), String> {
        let mut full: Vec<&str> = vec!["plugins"];
        full.extend_from_slice(args);
        let output = std::process::Command::new(BINARY)
            .args(&full)
            .output()
            .map_err(|e| format!("Failed to run {} plugins: {}", BINARY, e))?;
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Ok((output.status.success(), stderr))
    }

    fn sessions_root(&self) -> Option<PathBuf> {
        self.dot_claude().map(|p| p.join(SESSIONS_SUBDIR))
    }

    fn session_dir_for_project(&self, project_path: &str) -> Option<PathBuf> {
        let encoded = project_path.replace('/', "-").replace('.', "-");
        self.sessions_root().map(|r| r.join(encoded))
    }

    fn session_file_extension(&self) -> &'static str {
        SESSION_EXT
    }

    fn extract_resume_id_from_output(&self, buffer: &str) -> Option<String> {
        // Mirror of the frontend regex: /claude --resume ([a-f0-9-]+)/
        // Walk the buffer manually so we don't pull in the `regex` crate just
        // for a single hex-uuid extraction.
        let needle = "claude --resume ";
        let start = buffer.find(needle)? + needle.len();
        let rest = &buffer[start..];
        let id: String = rest
            .chars()
            .take_while(|c| c.is_ascii_hexdigit() || *c == '-')
            .collect();
        if id.is_empty() {
            None
        } else {
            Some(id)
        }
    }

    fn usage_api_orgs_url(&self) -> Option<String> {
        Some("https://claude.ai/api/organizations".to_string())
    }

    fn usage_api_url_for(&self, org_id: &str) -> Option<String> {
        Some(format!(
            "https://claude.ai/api/organizations/{}/usage",
            org_id
        ))
    }

    fn is_session_file(&self, path: &Path) -> bool {
        path.extension().and_then(|e| e.to_str()) == Some(SESSION_EXT)
    }
}

/// Process-wide stateless instance.
pub static CLAUDE: ClaudeRunner = ClaudeRunner;
