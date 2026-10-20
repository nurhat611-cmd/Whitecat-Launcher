//! Движок загрузок лаунчера.
//!
//! * `download_all` — многопоточное скачивание (общий пул из `WORKERS` потоков),
//!   проверка SHA-1, докачка отсутствующих файлов и единый прогресс
//!   (проценты, счётчик файлов, размер, скорость, имя текущего файла).
//! * `build_plan` — полный план установки версии: клиент, библиотеки (включая
//!   нативки), индекс ассетов и сами объекты ассетов, а также библиотеки
//!   Fabric-профиля, если выбран этот загрузчик.

use crate::{os_name, versions};
use serde::Serialize;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::AppHandle;

/// Сколько файлов качаем одновременно.
pub const WORKERS: usize = 8;
/// Служебная ошибка «задача отменена» — по ней очередь отличает отмену от сбоя.
pub const CANCELLED: &str = "__mylauncher_cancelled__";
pub const USER_AGENT: &str = "MyLauncher/0.2.0 (nurhat611-cmd/Whitecat-Launcher)";
const FABRIC_META: &str = "https://meta.fabricmc.net";
const ASSET_CDN: &str = "https://resources.download.minecraft.net";

/// Один файл к скачиванию.
#[derive(Clone, Debug)]
pub struct Job {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

impl Job {
    pub fn new(url: impl Into<String>, dest: PathBuf) -> Self {
        Self {
            url: url.into(),
            dest,
            sha1: None,
            size: None,
        }
    }

    pub fn with_meta(mut self, sha1: Option<&str>, size: Option<u64>) -> Self {
        self.sha1 = sha1.map(|s| s.to_lowercase());
        self.size = size;
        self
    }
}

/// Снимок прогресса загрузки.
#[derive(Clone, Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub percent: u32,
    pub message: String,
    pub current_file: String,
    pub done_files: u32,
    pub total_files: u32,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
}

/// HTTP-клиент с обязательным User-Agent (его требуют Modrinth и Mojang).
pub fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Не удалось создать HTTP-клиент: {e}"))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

fn cancelled(cancel: &Option<std::sync::Arc<AtomicBool>>) -> bool {
    cancel
        .as_ref()
        .map(|flag| flag.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn file_sha1(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(hex_digest(&hasher.finalize()))
}

/// Файл уже на месте и совпадает по SHA-1/размеру?
fn is_satisfied(job: &Job) -> bool {
    let Ok(meta) = fs::metadata(&job.dest) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    if let Some(expected) = &job.sha1 {
        return file_sha1(&job.dest)
            .map(|actual| actual == *expected)
            .unwrap_or(false);
    }
    if let Some(size) = job.size {
        return size > 0 && meta.len() == size;
    }
    meta.len() > 0
}

/// Скачивает набор файлов в пуле потоков.
///
/// `report` вызывается не чаще ~7 раз в секунду из рабочих потоков.
pub fn download_all(
    jobs: Vec<Job>,
    stage: &str,
    cancel: Option<std::sync::Arc<AtomicBool>>,
    report: &(dyn Fn(Progress) + Sync),
) -> Result<(), String> {
    let mut todo: VecDeque<Job> = VecDeque::new();
    let mut total_bytes: u64 = 0;
    let mut done_bytes: u64 = 0;
    let mut total_files: u32 = 0;
    let mut done_files: u32 = 0;

    // Дубликаты путей (например, библиотека пришла и из ванильного профиля,
    // и из Fabric) скачиваем один раз.
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    for job in jobs {
        if !seen.insert(job.dest.clone()) {
            continue;
        }
        total_files += 1;
        let size = job.size.unwrap_or(0);
        total_bytes += size;
        if is_satisfied(&job) {
            done_files += 1;
            done_bytes += size;
            continue;
        }
        todo.push_back(job);
    }

    if todo.is_empty() {
        report(Progress {
            percent: 100,
            message: format!("{stage}: всё уже скачано"),
            current_file: String::new(),
            done_files: total_files,
            total_files,
            downloaded_bytes: total_bytes,
            total_bytes,
            speed_bps: 0,
        });
        return Ok(());
    }

    let workers = WORKERS.min(todo.len());
    let base_done_bytes = done_bytes;
    let queue = Mutex::new(todo);
    let done_bytes_counter = AtomicU64::new(done_bytes);
    let done_files_counter = AtomicUsize::new(done_files as usize);
    let abort = AtomicBool::new(false);
    let cancelled_flag = AtomicBool::new(false);
    let current = Mutex::new(String::new());
    let last_emit = Mutex::new(Instant::now() - Duration::from_secs(10));
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let started = Instant::now();

    let snapshot = |force: bool| {
        if force {
            *lock(&last_emit) = Instant::now();
        } else {
            let mut last = lock(&last_emit);
            if last.elapsed() < Duration::from_millis(140) {
                return;
            }
            *last = Instant::now();
        }
        let downloaded = done_bytes_counter.load(Ordering::Relaxed);
        let files = done_files_counter.load(Ordering::Relaxed).min(total_files as usize) as u32;
        let percent = if total_bytes > 0 {
            ((downloaded as f64 / total_bytes as f64) * 100.0).min(99.0) as u32
        } else if total_files > 0 {
            (files * 100 / total_files).min(99)
        } else {
            0
        };
        let elapsed = started.elapsed().as_secs_f64().max(0.05);
        let speed = ((downloaded.saturating_sub(base_done_bytes)) as f64 / elapsed) as u64;
        report(Progress {
            percent,
            message: format!("{stage} · {files}/{total_files}"),
            current_file: lock(&current).clone(),
            done_files: files,
            total_files,
            downloaded_bytes: downloaded,
            total_bytes,
            speed_bps: speed,
        });
    };

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let client = match http_client() {
                    Ok(client) => client,
                    Err(e) => {
                        lock(&errors).push(e);
                        abort.store(true, Ordering::Relaxed);
                        return;
                    }
                };
                loop {
                    if abort.load(Ordering::Relaxed) || cancelled(&cancel) {
                        if cancelled(&cancel) {
                            cancelled_flag.store(true, Ordering::Relaxed);
                        }
                        break;
                    }
                    let job = {
                        let mut queue = lock(&queue);
                        queue.pop_front()
                    };
                    let Some(job) = job else {
                        break;
                    };
                    *lock(&current) = file_name_of(&job.dest);
                    snapshot(false);

                    let mut attempt = 0u32;
                    loop {
                        let before = done_bytes_counter.load(Ordering::Relaxed);
                        let on_chunk = |bytes: u64| {
                            done_bytes_counter.fetch_add(bytes, Ordering::Relaxed);
                            snapshot(false);
                        };
                        match transfer(&client, &job, &on_chunk, &abort, &cancel) {
                            Ok(()) => {
                                done_files_counter.fetch_add(1, Ordering::Relaxed);
                                snapshot(true);
                                break;
                            }
                            Err(e) if e == CANCELLED => {
                                cancelled_flag.store(true, Ordering::Relaxed);
                                abort.store(true, Ordering::Relaxed);
                                break;
                            }
                            Err(e) => {
                                // Откатываем байты неудачной попытки, чтобы прогресс не «переполнялся».
                                let now = done_bytes_counter.load(Ordering::Relaxed);
                                done_bytes_counter.fetch_sub(now.saturating_sub(before), Ordering::Relaxed);
                                attempt += 1;
                                if attempt >= 2 {
                                    lock(&errors).push(e);
                                    abort.store(true, Ordering::Relaxed);
                                    break;
                                }
                            }
                        }
                    }
                }
            });
        }
    });

    if cancelled_flag.load(Ordering::Relaxed) {
        return Err(CANCELLED.to_string());
    }
    if let Some(error) = lock(&errors).first().cloned() {
        return Err(error);
    }
    report(Progress {
        percent: 100,
        message: format!("{stage}: готово"),
        current_file: String::new(),
        done_files: total_files,
        total_files,
        downloaded_bytes: total_bytes.max(done_bytes_counter.load(Ordering::Relaxed)),
        total_bytes,
        speed_bps: 0,
    });
    Ok(())
}

fn transfer(
    client: &reqwest::blocking::Client,
    job: &Job,
    on_chunk: &dyn Fn(u64),
    abort: &AtomicBool,
    cancel: &Option<std::sync::Arc<AtomicBool>>,
) -> Result<(), String> {
    if let Some(parent) = job.dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Не удалось создать папку {}: {e}", parent.display()))?;
    }
    let mut response = client
        .get(&job.url)
        .send()
        .map_err(|e| format!("Сетевая ошибка ({}): {e}", file_name_of(&job.dest)))?;
    if !response.status().is_success() {
        return Err(format!(
            "Сервер вернул статус {} для {}",
            response.status(),
            job.url
        ));
    }

    let mut part_name = job.dest.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    part_name.push(".part");
    let part = job.dest.with_file_name(part_name);

    let mut file = fs::File::create(&part)
        .map_err(|e| format!("Не удалось создать {}: {e}", part.display()))?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        if abort.load(Ordering::Relaxed) || cancelled(cancel) {
            drop(file);
            let _ = fs::remove_file(&part);
            return Err(CANCELLED.to_string());
        }
        let read = match response.read(&mut buffer) {
            Ok(read) => read,
            Err(e) => {
                drop(file);
                let _ = fs::remove_file(&part);
                return Err(format!("Обрыв соединения при загрузке {}: {e}", job.url));
            }
        };
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|e| format!("Ошибка записи {}: {e}", part.display()))?;
        hasher.update(&buffer[..read]);
        on_chunk(read as u64);
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);

    if let Some(expected) = &job.sha1 {
        let actual = hex_digest(&hasher.finalize());
        if actual != *expected {
            let _ = fs::remove_file(&part);
            return Err(format!(
                "Файл {} не прошёл проверку SHA-1",
                file_name_of(&job.dest)
            ));
        }
    }
    if job.dest.exists() {
        let _ = fs::remove_file(&job.dest);
    }
    fs::rename(&part, &job.dest)
        .map_err(|e| format!("Не удалось сохранить {}: {e}", job.dest.display()))?;
    Ok(())
}

// ---------- план установки версии ----------

pub struct Plan {
    pub jobs: Vec<Job>,
    pub profile_id: String,
    pub mc_version: String,
    pub loader: String,
}

struct RulesCtx {
    java_major: u32,
    mc_version: String,
}

impl RulesCtx {
    fn feature(&self, _name: &str) -> bool {
        false
    }
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let pa = a
        .split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect::<Vec<u64>>();
    let pb = b
        .split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect::<Vec<u64>>();
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
    }
    std::cmp::Ordering::Equal
}

fn rules_allow(rules: Option<&Value>, ctx: &RulesCtx) -> bool {
    let Some(list) = rules.and_then(Value::as_array) else {
        return true;
    };
    let mut allowed = false;
    for rule in list {
        if !rule_matches(rule, ctx) {
            continue;
        }
        allowed = rule.get("action").and_then(Value::as_str) == Some("allow");
    }
    allowed
}

fn rule_matches(rule: &Value, ctx: &RulesCtx) -> bool {
    if let Some(os) = rule.get("os") {
        if let Some(name) = os.get("name").and_then(Value::as_str) {
            if name != os_name() {
                return false;
            }
        }
        if let Some(arch) = os.get("arch").and_then(Value::as_str) {
            if arch != std::env::consts::ARCH {
                return false;
            }
        }
    }
    if let Some(features) = rule.get("features") {
        if let Some(name) = features.as_str() {
            if ctx.feature(name) {
                return false;
            }
        } else if let Some(obj) = features.as_object() {
            for (name, value) in obj {
                let want = value.as_bool().unwrap_or(false);
                if want {
                    return false;
                }
            }
        }
    }
    if let Some(java_version) = rule.get("java_version") {
        if let Some(major) = java_version.get("major_version").and_then(Value::as_u64) {
            if major as u32 != ctx.java_major {
                return false;
            }
        }
    }
    if let Some(mc) = rule.get("minecraft") {
        if let Some(min) = mc.get("greater_than_equal_to").and_then(Value::as_str) {
            if compare_versions(&ctx.mc_version, min) == std::cmp::Ordering::Less {
                return false;
            }
        }
        if let Some(max) = mc.get("less_than").and_then(Value::as_str) {
            if compare_versions(&ctx.mc_version, max) != std::cmp::Ordering::Less {
                return false;
            }
        }
    }
    true
}

fn maven_coordinates(name: &str, base: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let classifier = parts.get(3).copied().unwrap_or_default();
    let file_name = if classifier.is_empty() {
        format!("{artifact}-{version}.jar")
    } else {
        format!("{artifact}-{version}-{classifier}.jar")
    };
    let rel = format!("{group}/{artifact}/{version}/{file_name}");
    let url = if base.ends_with('/') {
        format!("{base}{rel}")
    } else {
        format!("{base}/{rel}")
    };
    Some((url, rel))
}

fn push_artifact(artifact: &Value, lib_dir: &Path, jobs: &mut Vec<Job>) {
    let Some(url) = artifact.get("url").and_then(Value::as_str) else {
        return;
    };
    let Some(rel) = artifact.get("path").and_then(Value::as_str) else {
        return;
    };
    jobs.push(
        Job::new(url, lib_dir.join(rel)).with_meta(
            artifact.get("sha1").and_then(Value::as_str),
            artifact.get("size").and_then(Value::as_u64),
        ),
    );
}

fn push_library_jobs(lib: &Value, ctx: &RulesCtx, game: &Path, jobs: &mut Vec<Job>) {
    let rules = lib.get("rules");
    let lib_dir = game.join("libraries");

    // Классические нативки (<= 1.12).
    if let Some(natives) = lib.get("natives").and_then(Value::as_object) {
        if natives.contains_key(os_name()) && rules_allow(rules, ctx) {
            if let Some(artifact) = lib.get("downloads").and_then(|d| d.get("artifact")) {
                push_artifact(artifact, &lib_dir, jobs);
            }
        }
        return;
    }

    // Современные нативки (1.13+).
    if let Some(classifiers) = lib
        .get("downloads")
        .and_then(|d| d.get("classifiers"))
        .and_then(Value::as_object)
    {
        let key = format!("natives-{}", os_name());
        if let Some(entry) = classifiers.get(&key) {
            if rules_allow(rules, ctx) {
                push_artifact(entry, &lib_dir, jobs);
            }
            return;
        }
    }

    if !rules_allow(rules, ctx) {
        return;
    }

    if let Some(artifact) = lib
        .get("downloads")
        .and_then(|d| d.get("artifact"))
        .filter(|a| !a.is_null())
    {
        if artifact.get("url").and_then(Value::as_str).is_some() {
            push_artifact(artifact, &lib_dir, jobs);
            return;
        }
    }

    if let Some(name) = lib.get("name").and_then(Value::as_str) {
        let base = lib
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("https://libraries.minecraft.net/");
        if let Some((url, rel)) = maven_coordinates(name, base) {
            jobs.push(Job::new(url, lib_dir.join(rel)));
        }
    }
}

/// Собирает план загрузки версии (`loader` = vanilla | fabric).
pub fn build_plan(
    app: &AppHandle,
    game: &Path,
    mc_id: &str,
    loader: &str,
) -> Result<Plan, String> {
    let loader = loader.to_lowercase();
    match loader.as_str() {
        "vanilla" | "fabric" => {}
        "forge" | "neoforge" => {
            return Err("Forge/NeoForge появятся в следующей части лаунчера. Выберите Vanilla или Fabric.".into())
        }
        _ => return Err("Неизвестный загрузчик".into()),
    }

    let (manifest, _) = versions::manifest(app, 2 * 3600)?;
    let entry = manifest
        .get("versions")
        .and_then(Value::as_array)
        .and_then(|list| {
            list.iter()
                .find(|v| v.get("id").and_then(Value::as_str) == Some(mc_id))
        })
        .cloned()
        .ok_or_else(|| format!("Версия {mc_id} не найдена в манифесте Mojang"))?;
    let version_url = entry
        .get("url")
        .and_then(Value::as_str)
        .ok_or("Повреждённый манифест версий")?;
    let vanilla = crate::http_json(version_url)
        .map_err(|e| format!("Не удалось загрузить описание версии {mc_id}: {e}"))?;

    let overlay: Option<Value> = if loader == "fabric" {
        let url = format!("{FABRIC_META}/v2/versions/loader/{mc_id}/stable/json");
        let list = crate::http_json(&url)
            .map_err(|e| format!("Не удалось получить профиль Fabric: {e}"))?;
        let profile = list
            .get(0)
            .and_then(|item| item.get("profile"))
            .cloned()
            .ok_or("Для этой версии нет stable-сборки Fabric")?;
        Some(profile)
    } else {
        None
    };

    let profile_id = if overlay.is_some() {
        format!("fabric-{mc_id}")
    } else {
        mc_id.to_string()
    };
    let version_dir = game.join("versions").join(&profile_id);
    fs::create_dir_all(&version_dir)
        .map_err(|e| format!("Не удалось создать папку версии: {e}"))?;

    // Сохраняем профиль версии рядом с jar — пригодится для истории запусков.
    let profile_json = overlay.clone().unwrap_or_else(|| vanilla.clone());
    if let Ok(text) = serde_json::to_string_pretty(&profile_json) {
        let _ = fs::write(version_dir.join(format!("{profile_id}.json")), text);
    }

    let java_major = crate::java::detect_java(app)
        .ok()
        .and_then(|info| info.major)
        .unwrap_or(21);
    let ctx = RulesCtx {
        java_major,
        mc_version: mc_id.to_string(),
    };

    let mut jobs: Vec<Job> = Vec::new();

    // Клиент.
    let client = vanilla
        .get("downloads")
        .and_then(|d| d.get("client"))
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(url) = client.get("url").and_then(Value::as_str) {
        jobs.push(
            Job::new(url, version_dir.join(format!("{profile_id}.jar"))).with_meta(
                client.get("sha1").and_then(Value::as_str),
                client.get("size").and_then(Value::as_u64),
            ),
        );
    } else {
        return Err("В манифесте версии нет ссылки на клиент".into());
    }

    // Библиотеки: ванильные + профиль загрузчика.
    let mut libraries: Vec<Value> = vanilla
        .get("libraries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(extra) = overlay
        .as_ref()
        .and_then(|o| o.get("libraries"))
        .and_then(Value::as_array)
    {
        libraries.extend(extra.iter().cloned());
    }
    for lib in &libraries {
        push_library_jobs(lib, &ctx, game, &mut jobs);
    }

    // Ассеты: индекс + объекты.
    let profile = overlay.as_ref().unwrap_or(&vanilla);
    let index = profile
        .get("assetIndex")
        .or_else(|| vanilla.get("assetIndex"))
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(index_id) = index.get("id").and_then(Value::as_str) {
        let index_path = game
            .join("assets")
            .join("indexes")
            .join(format!("{index_id}.json"));
        if let Some(url) = index.get("url").and_then(Value::as_str) {
            jobs.push(
                Job::new(url, index_path.clone()).with_meta(
                    index.get("sha1").and_then(Value::as_str),
                    index.get("size").and_then(Value::as_u64),
                ),
            );
        }
        let parsed = match fs::read_to_string(&index_path)
            .ok()
            .and_then(|text| text.parse::<Value>().ok())
        {
            Some(value) => Some(value),
            None => {
                let url = index
                    .get("url")
                    .and_then(Value::as_str)
                    .ok_or("Нет ссылки на индекс ассетов")?;
                let value = crate::http_json(url)
                    .map_err(|e| format!("Не удалось загрузить индекс ассетов: {e}"))?;
                if let Ok(text) = serde_json::to_string(&value) {
                    if let Some(parent) = index_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&index_path, text);
                }
                Some(value)
            }
        };
        if let Some(parsed) = parsed {
            let objects_dir = game.join("assets").join("objects");
            if let Some(map) = parsed.get("map").and_then(Value::as_object) {
                for (hash, _) in map {
                    if hash.len() < 2 {
                        continue;
                    }
                    let dest = objects_dir.join(&hash[..2]).join(hash);
                    let url = format!("{ASSET_CDN}/{}/{}", &hash[..2], hash);
                    jobs.push(Job::new(url, dest));
                }
            } else if let Some(objects) = parsed.get("objects").and_then(Value::as_object) {
                for value in objects.values() {
                    let hash = value
                        .get("hash")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    if hash.len() < 2 {
                        continue;
                    }
                    let size = value.get("size").and_then(Value::as_u64);
                    let dest = objects_dir.join(&hash[..2]).join(&hash);
                    let url = format!("{ASSET_CDN}/{}/{}", &hash[..2], hash);
                    jobs.push(Job::new(url, dest).with_meta(Some(&hash), size));
                }
            }
        }
    }

    Ok(Plan {
        jobs,
        profile_id,
        mc_version: mc_id.to_string(),
        loader,
    })
}
