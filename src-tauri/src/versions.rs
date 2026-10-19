//! Список версий Minecraft из официального API Mojang (piston-meta):
//! кеш на диске, фоновое обновление раз в час и ручное обновление из интерфейса.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
/// Кеш считается свежим 30 минут — интерфейс открывается мгновенно.
pub const CACHE_TTL: u64 = 30 * 60;
/// Фоновое обновление манифеста раз в час.
pub const REFRESH_INTERVAL: u64 = 60 * 60;
pub const EVENT: &str = "versions_updated";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub release_time: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    pub versions: Vec<VersionSummary>,
    pub latest_release: Option<String>,
    pub latest_snapshot: Option<String>,
    pub fetched_at: u64,
    pub from_cache: bool,
    pub stale: bool,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Не удалось определить папку конфигурации: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("version_manifest.json"))
}

fn read_cache(app: &AppHandle) -> Option<(Value, u64)> {
    let path = cache_path(app).ok()?;
    let text = fs::read_to_string(path).ok()?;
    let parsed: Value = serde_json::from_str(&text).ok()?;
    let fetched_at = parsed.get("fetchedAt")?.as_u64()?;
    let manifest = parsed.get("manifest")?.clone();
    if manifest.get("versions").is_none() {
        return None;
    }
    Some((manifest, fetched_at))
}

fn write_cache(app: &AppHandle, manifest: &Value, fetched_at: u64) {
    let Ok(path) = cache_path(app) else {
        return;
    };
    let payload = serde_json::json!({ "fetchedAt": fetched_at, "manifest": manifest });
    if let Ok(text) = serde_json::to_string(&payload) {
        let _ = fs::write(path, text);
    }
}

/// Манифест версий: из кеша, если он моложе `max_age` секунд, иначе из сети.
pub fn manifest(app: &AppHandle, max_age: u64) -> Result<(Value, u64), String> {
    if let Some((manifest, fetched_at)) = read_cache(app) {
        if now_secs().saturating_sub(fetched_at) < max_age {
            return Ok((manifest, fetched_at));
        }
    }
    let manifest = crate::http_json(MANIFEST_URL)
        .map_err(|e| format!("Не удалось загрузить список версий Mojang: {e}"))?;
    if manifest.get("versions").and_then(Value::as_array).is_none() {
        return Err("Mojang вернул некорректный манифест версий".into());
    }
    let fetched_at = now_secs();
    write_cache(app, &manifest, fetched_at);
    Ok((manifest, fetched_at))
}

/// Преобразует сырой манифест в список для интерфейса.
pub fn to_list(manifest: &Value, fetched_at: u64, from_cache: bool, stale: bool) -> VersionList {
    let mut versions: Vec<VersionSummary> = manifest
        .get("versions")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|item| {
                    let id = item.get("id").and_then(Value::as_str)?.to_string();
                    let kind = item
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("release")
                        .to_string();
                    let release_time = item
                        .get("releaseTime")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let url = item
                        .get("url")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    Some(VersionSummary {
                        id,
                        kind,
                        release_time,
                        url,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // Свежие версии — первыми (манифест и так отсортирован, но кеш может быть старым).
    versions.sort_by(|a, b| b.release_time.cmp(&a.release_time));

    let latest = manifest.get("latest");
    VersionList {
        versions,
        latest_release: latest
            .and_then(|l| l.get("release"))
            .and_then(Value::as_str)
            .map(str::to_string),
        latest_snapshot: latest
            .and_then(|l| l.get("snapshot"))
            .and_then(Value::as_str)
            .map(str::to_string),
        fetched_at,
        from_cache,
        stale,
    }
}

#[tauri::command]
pub async fn get_versions(app: AppHandle, refresh: bool) -> Result<VersionList, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if !refresh {
            if let Some((manifest, fetched_at)) = read_cache(&app) {
                if now_secs().saturating_sub(fetched_at) < CACHE_TTL {
                    return Ok(to_list(&manifest, fetched_at, true, false));
                }
            }
        }
        match manifest(&app, 0) {
            Ok((manifest, fetched_at)) => Ok(to_list(&manifest, fetched_at, false, false)),
            Err(error) => match read_cache(&app) {
                Some((manifest, fetched_at)) => Ok(to_list(&manifest, fetched_at, true, true)),
                None => Err(error),
            },
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Фоновое обслуживание кеша: при старте и далее раз в 10 минут проверяет,
/// не устарел ли манифест, и молча обновляет его с уведомлением интерфейса.
pub fn spawn_background_refresh(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            let stale = match read_cache(&app) {
                Some((_, fetched_at)) => {
                    now_secs().saturating_sub(fetched_at) >= REFRESH_INTERVAL
                }
                None => true,
            };
            if stale {
                if let Ok((manifest, fetched_at)) = manifest(&app, REFRESH_INTERVAL) {
                    let _ = app.emit(EVENT, to_list(&manifest, fetched_at, false, false));
                }
            }
            std::thread::sleep(Duration::from_secs(600));
        }
    });
}
