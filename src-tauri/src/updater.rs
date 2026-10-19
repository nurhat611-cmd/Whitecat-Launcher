//! Проверка обновлений лаунчера (заглушка: GitHub releases).

use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub checked_at: u64,
    pub changelog: Option<String>,
    pub download_url: Option<String>,
}

fn cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("update_check.json"))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    let pa: Vec<u64> = a.split('.').filter_map(|x| x.parse().ok()).collect();
    let pb: Vec<u64> = b.split('.').filter_map(|x| x.parse().ok()).collect();
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => {}
            o => return o,
        }
    }
    std::cmp::Ordering::Equal
}

pub fn check_update(app: &AppHandle, force: bool) -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();

    // Try cache unless force
    if !force {
        if let Ok(path) = cache_path(app) {
            if let Ok(text) = fs::read_to_string(&path) {
                if let Ok(cached) = serde_json::from_str::<UpdateInfo>(&text) {
                    if now_secs().saturating_sub(cached.checked_at) < 3600 {
                        return Ok(cached);
                    }
                }
            }
        }
    }

    // Attempt fetch GitHub releases
    let latest = fetch_latest().unwrap_or(current.clone());
    let has_update = compare(&latest, &current) == std::cmp::Ordering::Greater;

    let info = UpdateInfo {
        current_version: current.clone(),
        latest_version: Some(latest.clone()),
        has_update,
        checked_at: now_secs(),
        changelog: if has_update {
            Some(format!("Доступна новая версия {latest}. Текущая: {current}."))
        } else {
            Some("У вас последняя версия.".to_string())
        },
        download_url: Some("https://github.com/nurhat611-cmd/Whitecat-Launcher/releases".to_string()),
    };

    if let Ok(path) = cache_path(app) {
        let _ = fs::write(path, serde_json::to_string(&info).unwrap_or_default());
    }

    Ok(info)
}

fn fetch_latest() -> Option<String> {
    let client = crate::installer::http_client().ok()?;
    let resp = client
        .get("https://api.github.com/repos/nurhat611-cmd/Whitecat-Launcher/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().ok()?;
    let tag = json.get("tag_name").and_then(|v| v.as_str())?;
    Some(tag.trim_start_matches('v').to_string())
}

#[tauri::command]
pub fn check_for_updates(app: AppHandle, force: Option<bool>) -> Result<UpdateInfo, String> {
    check_update(&app, force.unwrap_or(false))
}
