//! Скачивание JDK 21 (Adoptium) с прогрессом: метаданные API → архив → распаковка в папку лаунчера.

use crate::java::detect_java;
use crate::{emit_event, get_json, JavaInfo};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const JDK_MAJOR: &str = "21";
const EVENT: &str = "jdk_progress";

pub fn download_jdk(app: &AppHandle) -> Result<JavaInfo, String> {
    let data = app.path().app_local_data_dir().map_err(|e| e.to_string())?;
    let work = data.join("jdk-download");
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| format!("Не удалось создать рабочую папку: {e}"))?;

    let (os, ext) = platform();
    let arch = if std::env::consts::ARCH == "aarch64" {
        "aarch64"
    } else {
        "x64"
    };
    let meta_url = format!(
        "https://api.adoptium.net/v3/assets/latest/{JDK_MAJOR}/hotspot?architecture={arch}&image_type=jdk&os={os}&vendor=eclipse"
    );

    emit_event(app, EVENT, "query", "Запрашиваем сведения о JDK…", 0);
    let meta = get_json(&meta_url).map_err(|e| format!("Не удалось получить данные Adoptium: {e}"))?;
    let link = meta[0]["binary"]["package"]["link"]
        .as_str()
        .ok_or("Не удалось получить ссылку на JDK")?
        .to_string();
    let package_name = meta[0]["binary"]["package"]["name"]
        .as_str()
        .unwrap_or("jdk")
        .to_string();
    let total = meta[0]["binary"]["package"]["size"].as_u64();

    emit_event(app, EVENT, "download", "Начинаю загрузку…", 1);
    let archive = work.join(format!("{package_name}.tmp"));
    let mut file = fs::File::create(&archive).map_err(|e| format!("Не удалось создать файл: {e}"))?;
    let response =
        reqwest::blocking::get(&link).map_err(|e| format!("Не удалось начать загрузку JDK: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Adoptium вернул статус {}", response.status()));
    }
    let total = total.or_else(response.content_length);
    let mut done: u64 = 0;
    while let Some(chunk) = response.chunk() {
        let chunk = chunk.map_err(|e| format!("Сбой сети при загрузке JDK: {e}"))?;
        file.write_all(&chunk).map_err(|e| format!("Ошибка записи: {e}"))?;
        done += chunk.len() as u64;
        let percent = match total {
            Some(t) if t > 0 => (1 + (done as f64 / t as f64 * 88.0)) as u32,
            _ => 1,
        };
        emit_event(
            app,
            EVENT,
            "download",
            format!("Скачивание JDK {JDK_MAJOR}: {} МБ", done / (1024 * 1024)),
            percent.min(89),
        );
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);

    emit_event(app, EVENT, "extract", "Распаковка JDK…", 92);
    let raw = work.join("extract");
    fs::create_dir_all(&raw).map_err(|e| e.to_string())?;
    if ext == "zip" {
        extract_zip(&archive, &raw)?;
    } else {
        extract_targz(&archive, &raw)?;
    }
    let inner = first_dir(&raw).ok_or("В архиве не найдена папка JDK")?;
    let dest = data.join("jdk");
    let _ = fs::remove_dir_all(&dest);
    fs::rename(&inner, &dest).map_err(|e| format!("Не удалось установить JDK: {e}"))?;
    let _ = fs::remove_dir_all(&work);
    emit_event(app, EVENT, "done", "JDK установлен", 100);

    let info = detect_java(app)?;
    if !info.found {
        return Err("JDK распакован, но исполняемый файл java не найден".into());
    }
    Ok(info)
}

fn platform() -> (&'static str, &'static str) {
    if cfg!(windows) {
        ("windows", "zip")
    } else if cfg!(target_os = "macos") {
        ("mac", "tar.gz")
    } else {
        ("linux", "tar.gz")
    }
}

fn extract_zip(path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Повреждённый zip: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let out = dest.join(&name);
        if !out.starts_with(dest) {
            continue;
        }
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out_file = fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn extract_targz(path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(gz);
    tar.unpack(dest)
        .map_err(|e| format!("Ошибка распаковки архива: {e}"))
}

/// Первая каталожная запись внутри `dir` (в JDK-архиве всегда одна корневая папка).
fn first_dir(dir: &Path) -> Option<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return None;
    };
    entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.is_dir())
}
