use std::path::PathBuf;

fn sanitize_branch_name(name: &str) -> String {
    let sanitized: String = name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '/' || *c == '-' || *c == '_' || *c == '.')
        .collect();
    let sanitized = sanitized.replace("..", ".").replace(".lock", "")
        .trim_matches(|c: char| c == '.' || c == '/' || c == '-').to_string();
    if sanitized.is_empty() { return "clauge/unnamed".to_string(); }
    sanitized.split('/').map(|seg| {
        if seg.starts_with('-') { format!("x{}", seg) } else { seg.to_string() }
    }).collect::<Vec<_>>().join("/")
}

#[tauri::command]
pub fn agent_is_git_repo(path: String) -> Result<bool, String> {
    let output = std::process::Command::new("git").args(["-C", &path, "rev-parse", "--is-inside-work-tree"]).output().map_err(|e| e.to_string())?;
    Ok(output.status.success())
}

#[tauri::command]
pub fn agent_create_worktree(project_path: String, branch_name: String) -> Result<String, String> {
    let branch_name = sanitize_branch_name(&branch_name);
    let worktree_dir = PathBuf::from(&project_path).join(".clauge-worktrees").join(&branch_name);
    let worktree_path = worktree_dir.to_string_lossy().to_string();
    if worktree_dir.exists() { return Ok(worktree_path); }
    let _ = std::fs::create_dir_all(worktree_dir.parent().unwrap_or(&worktree_dir));
    let _ = std::process::Command::new("git").args(["-C", &project_path, "worktree", "prune"]).output();
    let output = std::process::Command::new("git").args(["-C", &project_path, "worktree", "add", "-b", &branch_name, &worktree_path]).output().map_err(|e| format!("git worktree add failed: {}", e))?;
    if !output.status.success() {
        let output2 = std::process::Command::new("git").args(["-C", &project_path, "worktree", "add", &worktree_path, &branch_name]).output().map_err(|e| format!("git worktree add failed: {}", e))?;
        if !output2.status.success() { return Err(format!("git worktree add failed: {}", String::from_utf8_lossy(&output2.stderr))); }
    }
    let gitignore = PathBuf::from(&project_path).join(".gitignore");
    if let Ok(contents) = std::fs::read_to_string(&gitignore) {
        if !contents.contains(".clauge-worktrees") {
            let _ = std::fs::write(&gitignore, format!("{}\n.clauge-worktrees/\n", contents.trim_end()));
        }
    } else {
        let _ = std::fs::write(&gitignore, ".clauge-worktrees/\n");
    }
    Ok(worktree_path)
}

#[tauri::command]
pub fn agent_remove_worktree(project_path: String, worktree_path: String) -> Result<(), String> {
    let _ = std::process::Command::new("git").args(["-C", &project_path, "worktree", "remove", "--force", &worktree_path]).output();
    let _ = std::process::Command::new("git").args(["-C", &project_path, "worktree", "prune"]).output();
    Ok(())
}
