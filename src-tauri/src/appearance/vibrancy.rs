use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceConfig {
    pub theme: String,
    pub accent_color: String,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "dark-glass".to_string(),
            accent_color: "#6366f1".to_string(),
        }
    }
}

#[cfg(target_os = "macos")]
pub fn apply_vibrancy(window: &tauri::WebviewWindow, material: &str) -> Result<(), String> {
    use window_vibrancy::{apply_vibrancy as wv_apply, NSVisualEffectMaterial};

    let mat = match material {
        "titlebar" => NSVisualEffectMaterial::Titlebar,
        "sidebar" => NSVisualEffectMaterial::Sidebar,
        "under-window" => NSVisualEffectMaterial::UnderWindowBackground,
        "hud" => NSVisualEffectMaterial::HudWindow,
        "content" => NSVisualEffectMaterial::ContentBackground,
        "header" => NSVisualEffectMaterial::HeaderView,
        "window" => NSVisualEffectMaterial::WindowBackground,
        "menu" => NSVisualEffectMaterial::Menu,
        "popover" => NSVisualEffectMaterial::Popover,
        "selection" => NSVisualEffectMaterial::Selection,
        #[allow(deprecated)]
        "dark" => NSVisualEffectMaterial::Dark,
        #[allow(deprecated)]
        "ultra-dark" => NSVisualEffectMaterial::UltraDark,
        "none" => return Ok(()), // no vibrancy
        _ => NSVisualEffectMaterial::Sidebar, // default to sidebar
    };

    wv_apply(window, mat, None, None).map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
pub fn apply_vibrancy(_window: &tauri::WebviewWindow, _material: &str) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn set_vibrancy(window: tauri::WebviewWindow, material: String) -> Result<(), String> {
    apply_vibrancy(&window, &material)
}

#[tauri::command]
pub async fn get_appearance(pool: State<'_, SqlitePool>) -> Result<AppearanceConfig, String> {
    let mut config = AppearanceConfig::default();

    if let Ok(Some(row)) =
        sqlx::query_as::<_, (String,)>("SELECT value FROM settings WHERE key = 'theme'")
            .fetch_optional(pool.inner())
            .await
    {
        config.theme = row.0;
    }

    if let Ok(Some(row)) =
        sqlx::query_as::<_, (String,)>("SELECT value FROM settings WHERE key = 'accent_color'")
            .fetch_optional(pool.inner())
            .await
    {
        config.accent_color = row.0;
    }

    Ok(config)
}

#[tauri::command]
pub async fn set_appearance(
    pool: State<'_, SqlitePool>,
    window: tauri::WebviewWindow,
    config: AppearanceConfig,
) -> Result<(), String> {
    let is_glass = config.theme == "dark-glass";
    let vibrancy_material = if is_glass { "sidebar" } else { "none" };

    let settings = [
        ("theme", config.theme.as_str()),
        ("accent_color", config.accent_color.as_str()),
    ];

    for (key, value) in &settings {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }

    apply_vibrancy(&window, vibrancy_material)?;

    Ok(())
}

/// Get list of available themes
#[tauri::command]
pub async fn get_available_themes() -> Result<Vec<ThemeInfo>, String> {
    Ok(vec![
        ThemeInfo {
            id: "dark-glass".to_string(),
            name: "Dark Glass".to_string(),
            description: "Translucent dark theme with macOS vibrancy".to_string(),
            preview_bg: "#07070f".to_string(),
            preview_accent: "#7c5cf8".to_string(),
        },
        ThemeInfo {
            id: "dark-solid".to_string(),
            name: "Dark Solid".to_string(),
            description: "Solid dark surfaces, no transparency".to_string(),
            preview_bg: "#0d0d18".to_string(),
            preview_accent: "#7c5cf8".to_string(),
        },
        ThemeInfo {
            id: "midnight".to_string(),
            name: "Midnight".to_string(),
            description: "Deep blacks, high contrast, OLED-friendly".to_string(),
            preview_bg: "#000000".to_string(),
            preview_accent: "#4f94d4".to_string(),
        },
        ThemeInfo {
            id: "nord".to_string(),
            name: "Nord".to_string(),
            description: "Arctic blue-gray palette".to_string(),
            preview_bg: "#2e3440".to_string(),
            preview_accent: "#88c0d0".to_string(),
        },
    ])
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub preview_bg: String,
    pub preview_accent: String,
}
