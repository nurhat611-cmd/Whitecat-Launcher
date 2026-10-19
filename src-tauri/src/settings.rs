use crate::LauncherSettings;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn load(app: &AppHandle) -> Result<LauncherSettings, String> {
    let path = config_dir(app)?.join("settings.json");
    if !path.exists() {
        return Ok(LauncherSettings::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    // try to parse; if missing fields, fill defaults via serde
    match serde_json::from_str::<LauncherSettings>(&text) {
        Ok(v) => Ok(v),
        Err(e) => {
            // try merging with default for backward compatibility
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                let mut def = serde_json::to_value(LauncherSettings::default()).unwrap();
                if let (Some(def_obj), Some(val_obj)) = (def.as_object_mut(), value.as_object()) {
                    for (k, v) in val_obj {
                        def_obj.insert(k.clone(), v.clone());
                    }
                    if let Ok(merged) = serde_json::from_value::<LauncherSettings>(def) {
                        return Ok(merged);
                    }
                }
            }
            Err(format!("Файл настроек повреждён: {e}"))
        }
    }
}

pub fn save(app: &AppHandle, settings: &LauncherSettings) -> Result<(), String> {
    let path = config_dir(app)?.join("settings.json");
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| format!("Не удалось сохранить настройки: {e}"))
}

/// Папка игры: из настроек, иначе стандартная папка данных приложения.
pub fn game_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let settings = load(app)?;
    let dir = match settings
        .game_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(path) => PathBuf::from(path),
        None => app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("minecraft"),
    };
    fs::create_dir_all(&dir).map_err(|e| format!("Не удалось создать папку игры: {e}"))?;
    Ok(dir)
}
