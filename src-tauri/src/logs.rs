//! Логи игры: latest.log, crash-reports, launcher.log

use crate::settings;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub name: String,
    pub path: String,
    pub content: String,
    pub modified_at: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameLogs {
    pub launcher_log: Option<String>,
    pub launcher_log_path: Option<String>,
    pub latest_log: Option<String>,
    pub latest_log_path: Option<String>,
    pub crash_reports: Vec<CrashReport>,
    pub game_dir: String,
}

fn read_text_capped(path: &Path, cap: usize) -> Option<String> {
    let data = fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&data).to_string();
    if text.len() > cap {
        // keep tail (most relevant)
        let start = text.len().saturating_sub(cap);
        // find next newline to avoid cutting mid-line
        let slice = &text[start..];
        if let Some(nl) = slice.find('\n') {
            Some(slice[nl + 1..].to_string())
        } else {
            Some(slice.to_string())
        }
    } else {
        Some(text)
    }
}

fn modified_secs(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn get_logs(app: &AppHandle) -> Result<GameLogs, String> {
    let game = settings::game_dir(app)?;
    let launcher_path = game.join("launcher.log");
    let latest_path = game.join("logs").join("latest.log");
    let crash_dir = game.join("crash-reports");

    let launcher_log = if launcher_path.exists() {
        read_text_capped(&launcher_path, 500_000)
    } else {
        None
    };
    let latest_log = if latest_path.exists() {
        read_text_capped(&latest_path, 500_000)
    } else {
        None
    };

    let mut crash_reports: Vec<CrashReport> = Vec::new();
    if crash_dir.exists() {
        if let Ok(entries) = fs::read_dir(&crash_dir) {
            let mut files: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_file())
                .collect();
            // sort by modified desc
            files.sort_by(|a, b| modified_secs(b).cmp(&modified_secs(a)));
            for path in files.into_iter().take(10) {
                if let Some(content) = read_text_capped(&path, 300_000) {
                    crash_reports.push(CrashReport {
                        name: path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "crash.txt".to_string()),
                        path: path.to_string_lossy().into_owned(),
                        content,
                        modified_at: modified_secs(&path),
                    });
                }
            }
        }
    }

    // if game crashed recently, also try to find hs_err logs
    if crash_reports.is_empty() {
        // look for hs_err_pid in game dir
        if let Ok(entries) = fs::read_dir(&game) {
            for p in entries.filter_map(|e| e.ok().map(|e| e.path())) {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("hs_err_pid") {
                        if let Some(content) = read_text_capped(&p, 300_000) {
                            crash_reports.push(CrashReport {
                                name: name.to_string(),
                                path: p.to_string_lossy().into_owned(),
                                content,
                                modified_at: modified_secs(&p),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(GameLogs {
        launcher_log,
        launcher_log_path: launcher_path
            .exists()
            .then(|| launcher_path.to_string_lossy().into_owned()),
        latest_log,
        latest_log_path: latest_path
            .exists()
            .then(|| latest_path.to_string_lossy().into_owned()),
        crash_reports,
        game_dir: game.to_string_lossy().into_owned(),
    })
}

pub fn open_folder(app: &AppHandle, kind: &str) -> Result<String, String> {
    let game = settings::game_dir(app)?;
    let target = match kind {
        "logs" => game.join("logs"),
        "crash" => game.join("crash-reports"),
        "game" => game.clone(),
        "skins" => game.join("skins"),
        _ => game.clone(),
    };
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let _ = open::that(&target);
    Ok(target.to_string_lossy().into_owned())
}

pub fn clear_logs(app: &AppHandle) -> Result<(), String> {
    let game = settings::game_dir(app)?;
    let _ = fs::remove_file(game.join("launcher.log"));
    // we don't delete latest.log, but truncate it
    let latest = game.join("logs").join("latest.log");
    if latest.exists() {
        let _ = fs::write(&latest, "");
    }
    Ok(())
}

#[tauri::command]
pub fn get_game_logs(app: AppHandle) -> Result<GameLogs, String> {
    get_logs(&app)
}

#[tauri::command]
pub fn open_logs_folder(app: AppHandle, kind: Option<String>) -> Result<String, String> {
    open_folder(&app, kind.as_deref().unwrap_or("logs"))
}

#[tauri::command]
pub fn open_game_folder(app: AppHandle) -> Result<String, String> {
    open_folder(&app, "game")
}

#[tauri::command]
pub fn get_launcher_log(app: AppHandle) -> Result<Option<String>, String> {
    let game = settings::game_dir(&app)?;
    let p = game.join("launcher.log");
    if p.exists() {
        Ok(read_text_capped(&p, 500_000))
    } else {
        Ok(None)
    }
}
