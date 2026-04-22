use crate::storage::get_storage_dir;
use tauri::tray::TrayIconId;

/// Save session key to local storage
#[tauri::command]
pub fn save_session_key(key: String) -> Result<(), String> {
    let path = get_storage_dir().join("session_key");
    std::fs::write(&path, &key).map_err(|e| e.to_string())
}

/// Load session key from local storage
#[tauri::command]
pub fn load_session_key() -> Result<Option<String>, String> {
    let path = get_storage_dir().join("session_key");
    if path.exists() {
        let key = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let key = key.trim().to_string();
        if key.is_empty() { Ok(None) } else { Ok(Some(key)) }
    } else {
        Ok(None)
    }
}

/// Get app version from Cargo.toml
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get Claude subscription plan from keychain
#[tauri::command]
pub fn get_claude_plan() -> Result<String, String> {
    let output = std::process::Command::new("security")
        .args(["find-generic-password", "-s", "Claude Code-credentials", "-w"])
        .output()
        .map_err(|e| format!("Keychain error: {}", e))?;
    if !output.status.success() { return Ok(String::new()); }
    let json_str = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(json_str.trim()).map_err(|e| e.to_string())?;
    Ok(parsed.get("claudeAiOauth")
        .and_then(|o| o.get("subscriptionType").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string())
}

/// Update the tray title text (shown in menu bar)
#[tauri::command]
pub fn update_tray_title(app_handle: tauri::AppHandle, title: String) -> Result<(), String> {
    if let Some(tray) = app_handle.tray_by_id(&TrayIconId::new("main-tray")) {
        tray.set_title(Some(&title)).map_err(|e| format!("Tray title error: {}", e))?;
    }
    Ok(())
}
