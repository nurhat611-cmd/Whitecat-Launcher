//! Клиент Modrinth API: поиск контента с фильтрами, версии проектов и установка
//! модов / ресурспаков / шейдеров / сборок (.mrpack) в правильные папки.

use crate::installer::{self, Job, Progress};
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::io::Read;
use std::path::{Component, Path};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentHit {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub project_type: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub versions: Vec<String>,
    pub date_modified: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub hits: Vec<ContentHit>,
    pub total_hits: u64,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentFile {
    pub url: String,
    pub filename: String,
    pub size: u64,
    pub sha1: Option<String>,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub downloads: u64,
    pub date_published: String,
    pub version_type: String,
    pub files: Vec<ContentFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfo {
    pub name: String,
    pub slug: String,
    pub path: String,
    pub mc_version: String,
    pub loader: String,
    pub loader_version: String,
    pub mod_count: u32,
    pub installed_at: u64,
    pub icon_url: Option<String>,
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn http_json_query(url: &str, params: &[(String, String)]) -> Result<Value, String> {
    let client = installer::http_client()?;
    let response = client
        .get(url)
        .query(params)
        .send()
        .map_err(|e| format!("Modrinth недоступен: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        let brief: String = body.chars().take(240).collect();
        return Err(if brief.is_empty() {
            format!("Modrinth вернул статус {status}")
        } else {
            format!("Modrinth вернул статус {status}: {brief}")
        });
    }
    response
        .json::<Value>()
        .map_err(|e| format!("Некорректный ответ Modrinth: {e}"))
}

fn parse_hit(value: &Value) -> Option<ContentHit> {
    Some(ContentHit {
        id: value
            .get("project_id")
            .and_then(Value::as_str)
            .or_else(|| value.get("id").and_then(Value::as_str))?
            .to_string(),
        slug: value
            .get("slug")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Без названия")
            .to_string(),
        description: value
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        author: value
            .get("author")
            .and_then(Value::as_str)
            .unwrap_or("неизвестный автор")
            .to_string(),
        project_type: value
            .get("project_type")
            .and_then(Value::as_str)
            .unwrap_or("mod")
            .to_string(),
        icon_url: value
            .get("icon_url")
            .and_then(Value::as_str)
            .map(str::to_string),
        downloads: value.get("downloads").and_then(Value::as_u64).unwrap_or(0),
        follows: value.get("follows").and_then(Value::as_u64).unwrap_or(0),
        categories: string_list(value.get("display_categories"))
            .into_iter()
            .chain(string_list(value.get("categories")))
            .collect(),
        versions: string_list(value.get("versions")),
        date_modified: value
            .get("date_modified")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

fn parse_version(value: &Value) -> Option<ContentVersion> {
    let files = value
        .get("files")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|file| {
                    let url = file.get("url").and_then(Value::as_str)?.to_string();
                    let filename = file.get("filename").and_then(Value::as_str)?.to_string();
                    Some(ContentFile {
                        url,
                        filename,
                        size: file.get("size").and_then(Value::as_u64).unwrap_or(0),
                        sha1: file
                            .get("hashes")
                            .and_then(|h| h.get("sha1"))
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        primary: file
                            .get("primary")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(ContentVersion {
        id: value
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        project_id: value
            .get("project_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        version_number: value
            .get("version_number")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        game_versions: string_list(value.get("game_versions")),
        loaders: string_list(value.get("loaders")),
        downloads: value.get("downloads").and_then(Value::as_u64).unwrap_or(0),
        date_published: value
            .get("date_published")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        version_type: value
            .get("version_type")
            .and_then(Value::as_str)
            .unwrap_or("release")
            .to_string(),
        files,
    })
}

/// Поиск контента: `project_type` = mod | resourcepack | shader | modpack.
pub fn search(
    query: &str,
    project_type: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
    sort: &str,
    offset: u32,
    limit: u32,
) -> Result<SearchResult, String> {
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{project_type}")]];
    if let Some(version) = game_version.map(str::trim).filter(|v| !v.is_empty()) {
        facets.push(vec![format!("versions:{version}")]);
    }
    if let Some(loader) = loader.map(str::trim).filter(|v| !v.is_empty()) {
        facets.push(vec![format!("categories:{loader}")]);
    }
    let facets_json =
        serde_json::to_string(&facets).map_err(|e| format!("Не удалось собрать фильтры: {e}"))?;

    let sort = match sort {
        "downloads" | "follows" | "newest" | "updated" | "relevance" => sort,
        _ => "relevance",
    };

    let value = http_json_query(
        &format!("{API}/search"),
        &[
            ("query".to_string(), query.to_string()),
            ("facets".to_string(), facets_json),
            ("index".to_string(), sort.to_string()),
            ("offset".to_string(), offset.to_string()),
            ("limit".to_string(), limit.clamp(1, 50).to_string()),
        ],
    )?;

    let hits = value
        .get("hits")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(parse_hit).collect::<Vec<_>>())
        .unwrap_or_default();

    Ok(SearchResult {
        hits,
        total_hits: value
            .get("total_hits")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        offset,
        limit: limit.clamp(1, 50),
    })
}

/// Версии проекта (с фильтрами по игре и загрузчику), свежие — первыми.
pub fn project_versions(
    project_id: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<ContentVersion>, String> {
    let mut params: Vec<(String, String)> = Vec::new();
    if let Some(version) = game_version.map(str::trim).filter(|v| !v.is_empty()) {
        params.push(("game_versions".to_string(), json!([version]).to_string()));
    }
    if let Some(loader) = loader.map(str::trim).filter(|v| !v.is_empty()) {
        params.push(("loaders".to_string(), json!([loader]).to_string()));
    }
    let value = http_json_query(&format!("{API}/project/{project_id}/version"), &params)?;
    Ok(value
        .as_array()
        .map(|list| list.iter().filter_map(parse_version).collect::<Vec<_>>())
        .unwrap_or_default())
}

fn fetch_version(version_id: &str) -> Result<ContentVersion, String> {
    let value = http_json_query(&format!("{API}/version/{version_id}"), &[])?;
    parse_version(&value).ok_or_else(|| "Modrinth вернул пустое описание версии".to_string())
}

fn pick_file(files: &[ContentFile]) -> Option<&ContentFile> {
    files
        .iter()
        .find(|file| file.primary)
        .or_else(|| files.first())
}

/// Папка установки для типа контента.
pub fn content_dir(project_type: &str) -> &'static str {
    match project_type {
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        "datapack" => "datapacks",
        "world" => "saves",
        _ => "mods",
    }
}

fn safe_file_name(name: &str) -> Option<String> {
    let base = Path::new(name).file_name()?.to_string_lossy().into_owned();
    if base.is_empty() || base.contains('/') || base.contains('\\') {
        None
    } else {
        Some(base)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Масштабирует прогресс вложенной загрузки в диапазон [from, to] процентов.
fn scaled(progress: &Progress, from: u32, to: u32) -> Progress {
    let span = to.saturating_sub(from);
    let mut out = progress.clone();
    out.percent = from + progress.percent * span / 100;
    out
}

fn report_stage(report: &(dyn Fn(Progress) + Sync), percent: u32, message: &str) {
    report(Progress {
        percent,
        message: message.to_string(),
        ..Default::default()
    });
}

/// Параметры одной установки из очереди.
pub struct InstallRequest {
    pub project_type: String,
    pub project_id: String,
    pub title: String,
    pub slug: String,
    pub version_id: Option<String>,
    pub game_version: Option<String>,
    pub loader: Option<String>,
    pub icon_url: Option<String>,
}

/// Проверяет, что относительный путь из архива/индекса не выходит за пределы инстанса.
fn is_safe_relative(relative: &str) -> bool {
    let path = Path::new(relative);
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

/// Устанавливает обычный файл (мод, ресурспак, шейдер) в папку игры.
fn install_file(
    request: &InstallRequest,
    game: &Path,
    version: &ContentVersion,
    cancel: Option<Arc<AtomicBool>>,
    report: &(dyn Fn(Progress) + Sync),
) -> Result<Option<String>, String> {
    let file = pick_file(&version.files)
        .ok_or_else(|| "У этой версии нет файлов для скачивания".to_string())?;
    let name = safe_file_name(&file.filename)
        .ok_or_else(|| format!("Некорректное имя файла: {}", file.filename))?;
    let dir = game.join(content_dir(&request.project_type));
    fs::create_dir_all(&dir).map_err(|e| format!("Не удалось создать {}: {e}", dir.display()))?;
    let dest = dir.join(name);

    report_stage(
        report,
        5,
        &format!("Скачиваем {}…", version.version_number),
    );
    let job = Job::new(file.url.clone(), dest.clone()).with_meta(
        file.sha1.as_deref(),
        if file.size > 0 { Some(file.size) } else { None },
    );
    let mapped = |progress: Progress| {
        let mut out = scaled(&progress, 5, 100);
        out.message = format!("Скачивание · {}", version.version_number);
        report(out);
    };
    installer::download_all(vec![job], "Скачивание файла", cancel, &mapped)?;
    Ok(Some(dest.to_string_lossy().into_owned()))
}

fn join_dependency_loader(dependencies: &Value) -> (String, String) {
    if let Some(version) = dependencies.get("fabric-loader").and_then(Value::as_str) {
        return ("fabric".to_string(), version.to_string());
    }
    if let Some(version) = dependencies.get("quilt-loader").and_then(Value::as_str) {
        return ("quilt".to_string(), version.to_string());
    }
    if let Some(version) = dependencies.get("neoforge").and_then(Value::as_str) {
        return ("neoforge".to_string(), version.to_string());
    }
    if let Some(version) = dependencies.get("forge").and_then(Value::as_str) {
        return ("forge".to_string(), version.to_string());
    }
    ("vanilla".to_string(), String::new())
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "instance".to_string()
    } else {
        trimmed
    }
}

fn extract_zip_prefix(
    archive: &mut zip::ZipArchive<fs::File>,
    prefixes: &[&str],
    dest_root: &Path,
) -> Result<u32, String> {
    let mut written = 0u32;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| format!("Повреждённый архив сборки: {e}"))?;
        let raw_name = entry.name().replace('\\', "/");
        if raw_name.ends_with('/') {
            continue;
        }
        let Some(prefix) = prefixes.iter().find(|p| raw_name.starts_with(*p)) else {
            continue;
        };
        let relative = &raw_name[prefix.len()..];
        if relative.is_empty() {
            continue;
        }
        if !is_safe_relative(relative) {
            continue;
        }
        let target = dest_root.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = fs::File::create(&target)
            .map_err(|e| format!("Не удалось записать {}: {e}", target.display()))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("Ошибка распаковки: {e}"))?;
        written += 1;
    }
    Ok(written)
}

/// Устанавливает сборку .mrpack в отдельный инстанс `instances/<slug>/`.
fn install_modpack(
    request: &InstallRequest,
    game: &Path,
    version: &ContentVersion,
    cancel: Option<Arc<AtomicBool>>,
    report: &(dyn Fn(Progress) + Sync),
) -> Result<Option<String>, String> {
    let file = pick_file(&version.files)
        .ok_or_else(|| "У этой версии сборки нет файла .mrpack".to_string())?;

    let temp_dir = std::env::temp_dir().join("mylauncher");
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    let archive_path = temp_dir.join(format!("{}.mrpack", slugify(&version.id)));

    report_stage(report, 2, "Скачиваем сборку…");
    let mapped = |progress: Progress| {
        let mut out = scaled(&progress, 2, 30);
        out.message = "Скачивание сборки".to_string();
        report(out);
    };
    installer::download_all(
        vec![Job::new(file.url.clone(), archive_path.clone()).with_meta(
            file.sha1.as_deref(),
            if file.size > 0 { Some(file.size) } else { None },
        )],
        "Сборка",
        cancel.clone(),
        &mapped,
    )?;

    let archive_file = fs::File::open(&archive_path)
        .map_err(|e| format!("Не удалось открыть скачанную сборку: {e}"))?;
    let mut archive = zip::ZipArchive::new(archive_file)
        .map_err(|e| format!("Файл сборки повреждён: {e}"))?;

    let index: Value = {
        let mut entry = archive
            .by_name("modrinth.index.json")
            .map_err(|_| "В .mrpack нет modrinth.index.json".to_string())?;
        let mut text = String::new();
        entry
            .read_to_string(&mut text)
            .map_err(|e| format!("Не удалось прочитать индекс сборки: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("Индекс сборки повреждён: {e}"))?
    };

    let slug = slugify(if request.slug.is_empty() {
        &request.title
    } else {
        &request.slug
    });
    let instance_dir = game.join("instances").join(&slug);
    fs::create_dir_all(&instance_dir)
        .map_err(|e| format!("Не удалось создать инстанс: {e}"))?;

    report_stage(report, 32, "Распаковываем overrides…");
    extract_zip_prefix(&mut archive, &["overrides/"], &instance_dir)?;
    report_stage(report, 38, "Распаковываем client-overrides…");
    extract_zip_prefix(&mut archive, &["client-overrides/"], &instance_dir)?;
    drop(archive);

    let mut jobs: Vec<Job> = Vec::new();
    if let Some(files) = index.get("files").and_then(Value::as_array) {
        for entry in files {
            let client_env = entry
                .get("env")
                .and_then(|env| env.get("client"))
                .and_then(Value::as_str)
                .unwrap_or("required");
            if client_env == "unsupported" {
                continue;
            }
            let Some(path) = entry.get("path").and_then(Value::as_str) else {
                continue;
            };
            let Some(url) = entry
                .get("downloads")
                .and_then(Value::as_array)
                .and_then(|list| list.first())
                .and_then(Value::as_str)
            else {
                continue;
            };
            if !is_safe_relative(path) {
                continue;
            }
            let dest = instance_dir.join(path);
            let sha1 = entry
                .get("hashes")
                .and_then(|h| h.get("sha1"))
                .and_then(Value::as_str);
            let size = entry.get("fileSize").and_then(Value::as_u64);
            jobs.push(Job::new(url, dest).with_meta(sha1, size));
        }
    }

    report_stage(
        report,
        42,
        &format!("Скачиваем файлы сборки: {}", jobs.len()),
    );
    let mapped = |progress: Progress| {
        let mut out = scaled(&progress, 42, 97);
        out.message = format!("Файлы сборки · {}", progress.message);
        report(out);
    };
    installer::download_all(jobs, "Файлы сборки", cancel, &mapped)?;

    let dependencies = index.get("dependencies").cloned().unwrap_or(Value::Null);
    let fallback_version = request.game_version.clone().unwrap_or_default();
    let mc_version = dependencies
        .get("minecraft")
        .and_then(Value::as_str)
        .unwrap_or(fallback_version.as_str())
        .to_string();
    let (loader, loader_version) = join_dependency_loader(&dependencies);
    let display_name = index
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&request.title)
        .to_string();

    let mod_count = fs::read_dir(instance_dir.join("mods"))
        .map(|entries| entries.filter_map(Result::ok).count() as u32)
        .unwrap_or(0);

    let meta = json!({
        "name": display_name,
        "slug": slug,
        "versionId": version.id,
        "versionNumber": version.version_number,
        "mcVersion": mc_version,
        "loader": loader,
        "loaderVersion": loader_version,
        "iconUrl": request.icon_url.clone(),
        "projectId": request.project_id.clone(),
        "installedAt": now_secs(),
        "modCount": mod_count,
        "path": instance_dir.to_string_lossy(),
    });
    fs::write(
        instance_dir.join("instance.json"),
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    )
    .map_err(|e| format!("Не удалось сохранить описание инстанса: {e}"))?;

    let _ = fs::remove_file(&archive_path);
    report_stage(report, 100, "Сборка установлена");
    Ok(Some(instance_dir.to_string_lossy().into_owned()))
}

/// Точка входа для очереди: выбирает файл или сборку и устанавливает.
pub fn install(
    request: &InstallRequest,
    game: &Path,
    cancel: Option<Arc<AtomicBool>>,
    report: &(dyn Fn(Progress) + Sync),
) -> Result<Option<String>, String> {
    report_stage(report, 1, "Получаем версию из Modrinth…");
    let version = match request.version_id.as_deref().filter(|id| !id.is_empty()) {
        Some(id) => fetch_version(id)?,
        None => {
            let loader = if request.project_type == "modpack" {
                None
            } else {
                request.loader.as_deref()
            };
            let mut versions = project_versions(
                &request.project_id,
                request.game_version.as_deref(),
                loader,
            )?;
            if versions.is_empty() {
                versions = project_versions(&request.project_id, None, None)?;
            }
            versions
                .into_iter()
                .next()
                .ok_or_else(|| "У проекта нет подходящих версий".to_string())?
        }
    };

    if request.project_type == "modpack" || version.files.iter().any(|f| {
        f.filename.to_lowercase().ends_with(".mrpack")
    }) {
        install_modpack(request, game, &version, cancel, report)
    } else {
        install_file(request, game, &version, cancel, report)
    }
}

// ---------- Tauri-команды ----------

#[tauri::command]
pub async fn search_content(
    query: String,
    project_type: String,
    game_version: Option<String>,
    loader: Option<String>,
    sort: Option<String>,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<SearchResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        search(
            &query,
            &project_type,
            game_version.as_deref(),
            loader.as_deref(),
            sort.as_deref().unwrap_or("relevance"),
            offset.unwrap_or(0),
            limit.unwrap_or(20),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_content_versions(
    project_id: String,
    game_version: Option<String>,
    loader: Option<String>,
) -> Result<Vec<ContentVersion>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        project_versions(&project_id, game_version.as_deref(), loader.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Список установленных сборок (инстансов).
#[tauri::command]
pub async fn list_instances(app: AppHandle) -> Result<Vec<InstanceInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let game = crate::settings::game_dir(&app)?;
        let dir = game.join("instances");
        let mut out: Vec<InstanceInfo> = Vec::new();
        let Ok(entries) = fs::read_dir(&dir) else {
            return Ok(out);
        };
        for entry in entries.filter_map(Result::ok) {
            let instance_dir = entry.path();
            if !instance_dir.is_dir() {
                continue;
            }
            let Ok(text) = fs::read_to_string(instance_dir.join("instance.json")) else {
                continue;
            };
            let Ok(meta) = serde_json::from_str::<Value>(&text) else {
                continue;
            };
            let slug = instance_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push(InstanceInfo {
                name: meta
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(&slug)
                    .to_string(),
                slug,
                path: meta
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or(&instance_dir.to_string_lossy())
                    .to_string(),
                mc_version: meta
                    .get("mcVersion")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                loader: meta
                    .get("loader")
                    .and_then(Value::as_str)
                    .unwrap_or("vanilla")
                    .to_string(),
                loader_version: meta
                    .get("loaderVersion")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                mod_count: meta
                    .get("modCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32,
                installed_at: meta
                    .get("installedAt")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                icon_url: meta
                    .get("iconUrl")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            });
        }
        out.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}
