//! Скины: загрузка по URL / из файла, превью и хранение.

use crate::settings;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkinInfo {
    pub exists: bool,
    pub path: Option<String>,
    pub model: String,
    pub has_custom: bool,
    pub preview_base64: Option<String>,
}

fn skins_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let game = settings::game_dir(app)?;
    let dir = game.join("skins");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn skin_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(skins_dir(app)?.join("skin.png"))
}

fn skin_meta_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(skins_dir(app)?.join("skin.json"))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SkinMeta {
    model: String,
    url: Option<String>,
    updated_at: u64,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_meta(app: &AppHandle) -> SkinMeta {
    let path = skin_meta_path(app).unwrap_or_else(|_| PathBuf::from("skin.json"));
    if path.exists() {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(meta) = serde_json::from_str::<SkinMeta>(&text) {
                return meta;
            }
        }
    }
    // fallback to settings
    let settings_val = settings::load(app).unwrap_or_default();
    SkinMeta {
        model: if settings_val.skin_model.is_empty() {
            "steve".to_string()
        } else {
            settings_val.skin_model.clone()
        },
        url: settings_val.skin_url.clone(),
        updated_at: 0,
    }
}

fn write_meta(app: &AppHandle, meta: &SkinMeta) -> Result<(), String> {
    let path = skin_meta_path(app)?;
    let text = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())?;
    // also sync to settings
    let mut s = settings::load(app).unwrap_or_default();
    s.skin_model = meta.model.clone();
    s.skin_url = meta.url.clone();
    if let Some(p) = skin_path(app).ok() {
        if p.exists() {
            s.skin_path = Some(p.to_string_lossy().into_owned());
        }
    }
    let _ = settings::save(app, &s);
    Ok(())
}

fn validate_png(data: &[u8]) -> Result<(), String> {
    if data.len() < 8 {
        return Err("Файл слишком маленький — это не PNG скина".into());
    }
    // PNG signature
    if data[0..8] != [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return Err("Файл не является PNG изображением (скин должен быть PNG 64×64 или 64×32)".into());
    }
    if data.len() > 10 * 1024 * 1024 {
        return Err("Скин слишком большой (>10 МБ)".into());
    }
    Ok(())
}

pub fn get_skin(app: &AppHandle) -> Result<SkinInfo, String> {
    let path = skin_path(app)?;
    let exists = path.exists();
    let meta = read_meta(app);
    let preview = if exists {
        match fs::read(&path) {
            Ok(data) => Some(BASE64.encode(&data)),
            Err(_) => None,
        }
    } else {
        None
    };
    Ok(SkinInfo {
        exists,
        path: if exists {
            Some(path.to_string_lossy().into_owned())
        } else {
            None
        },
        model: meta.model,
        has_custom: exists,
        preview_base64: preview,
    })
}

pub fn save_skin_from_url(app: &AppHandle, url: String, model: String) -> Result<SkinInfo, String> {
    let model = normalize_model(&model);
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("Укажите URL скина".into());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("URL должен начинаться с http:// или https://".into());
    }

    let client = crate::installer::http_client()?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| format!("Не удалось скачать скин: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Сервер вернул статус {} для скина", resp.status()));
    }
    let bytes = resp.bytes().map_err(|e| format!("Ошибка загрузки скина: {e}"))?;
    validate_png(bytes.as_ref())?;

    let path = skin_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, bytes.as_ref()).map_err(|e| format!("Не удалось сохранить скин: {e}"))?;

    let meta = SkinMeta {
        model: model.clone(),
        url: Some(url),
        updated_at: now_secs(),
    };
    write_meta(app, &meta)?;

    get_skin(app)
}

pub fn save_skin_from_file(app: &AppHandle, base64_data: String, filename: Option<String>, model: String) -> Result<SkinInfo, String> {
    let model = normalize_model(&model);
    // strip data URL prefix if present
    let b64 = if let Some(idx) = base64_data.find(";base64,") {
        base64_data[idx + 8..].to_string()
    } else if base64_data.starts_with("data:") {
        return Err("Некорректный формат файла".into());
    } else {
        base64_data
    };
    if b64.trim().is_empty() {
        return Err("Файл пустой".into());
    }
    let bytes = BASE64
        .decode(b64.trim())
        .map_err(|e| format!("Не удалось декодировать файл: {e}"))?;

    // optional filename check
    if let Some(name) = filename {
        let lower = name.to_lowercase();
        if !lower.ends_with(".png") {
            return Err("Скин должен быть в формате PNG".into());
        }
    }

    validate_png(&bytes)?;

    let path = skin_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, &bytes).map_err(|e| format!("Не удалось сохранить скин: {e}"))?;

    let meta = SkinMeta {
        model: model.clone(),
        url: None,
        updated_at: now_secs(),
    };
    write_meta(app, &meta)?;

    get_skin(app)
}

pub fn delete_skin(app: &AppHandle) -> Result<SkinInfo, String> {
    let path = skin_path(app)?;
    let _ = fs::remove_file(&path);
    let meta_path = skin_meta_path(app)?;
    let _ = fs::remove_file(&meta_path);
    // clear settings skin fields
    if let Ok(mut s) = settings::load(app) {
        s.skin_url = None;
        s.skin_path = None;
        s.skin_model = "steve".to_string();
        let _ = settings::save(app, &s);
    }
    get_skin(app)
}

pub fn open_skins_folder(app: &AppHandle) -> Result<String, String> {
    let dir = skins_dir(app)?;
    let path_str = dir.to_string_lossy().into_owned();
    // try to open via OS
    let _ = open::that(&dir);
    Ok(path_str)
}

fn normalize_model(m: &str) -> String {
    match m.to_lowercase().as_str() {
        "alex" | "slim" | "3px" => "alex".to_string(),
        _ => "steve".to_string(),
    }
}

#[tauri::command]
pub fn get_skin_info(app: AppHandle) -> Result<SkinInfo, String> {
    get_skin(&app)
}

#[tauri::command]
pub fn save_skin_url(app: AppHandle, url: String, model: String) -> Result<SkinInfo, String> {
    save_skin_from_url(&app, url, model)
}

#[tauri::command]
pub fn save_skin_file(
    app: AppHandle,
    base64_data: String,
    filename: Option<String>,
    model: String,
) -> Result<SkinInfo, String> {
    save_skin_from_file(&app, base64_data, filename, model)
}

#[tauri::command]
pub fn delete_skin_cmd(app: AppHandle) -> Result<SkinInfo, String> {
    delete_skin(&app)
}

#[tauri::command]
pub fn open_skins_folder_cmd(app: AppHandle) -> Result<String, String> {
    open_skins_folder(&app)
}
