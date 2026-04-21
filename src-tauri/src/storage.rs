use std::path::PathBuf;
use crate::models::{SessionProfile, SessionStore};

pub fn get_storage_dir() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        let new_dir = home.join(".clauge");
        let old_dir = home.join(".ctx-mgr");
        // Migrate from old name
        if old_dir.exists() && !new_dir.exists() {
            let _ = std::fs::rename(&old_dir, &new_dir);
        }
        let _ = std::fs::create_dir_all(&new_dir);
        new_dir
    } else {
        PathBuf::from(".")
    }
}

pub fn get_storage_path() -> PathBuf {
    get_storage_dir().join("sessions.json")
}

pub fn load_profiles() -> Vec<SessionProfile> {
    let path = get_storage_path();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            let store: SessionStore = serde_json::from_str(&contents).unwrap_or(SessionStore {
                profiles: Vec::new(),
            });
            store.profiles
        }
        Err(_) => Vec::new(),
    }
}

pub fn save_profiles(profiles: &[SessionProfile]) -> Result<(), String> {
    let path = get_storage_path();
    let store = SessionStore {
        profiles: profiles.to_vec(),
    };
    let json = serde_json::to_string_pretty(&store).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn encode_project_path(project_path: &str) -> String {
    // Claude encodes project paths by replacing '/' and '.' with '-'
    // e.g. /Users/me/.clauge-worktrees -> -Users-me--clauge-worktrees
    project_path.replace('/', "-").replace('.', "-")
}

pub fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn project_name_from_path(project_path: &str) -> String {
    std::path::Path::new(project_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown")
        .to_string()
}
