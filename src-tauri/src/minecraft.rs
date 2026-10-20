//! Офлайн-запуск Minecraft (без аккаунта Mojang):
//! манифест версий → JSON версии (Vanilla или профиль Fabric) → клиент → библиотеки
//! (с нативками) → ассеты → сборка командной строки → запуск java-процесса.

use crate::java::detect_java;
use crate::{emit_event, get_json, os_name, LauncherSettings};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Manager};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const FABRIC_META: &str = "https://meta.fabricmc.net";
const EVENT: &str = "launch_progress";

struct RulesContext {
    java_major: u32,
    mc_version: String,
}

impl RulesContext {
    fn feature_enabled(&self, name: &str) -> bool {
        // Лаунчер запускает окно с фиксированным разрешением; быстрые игры отключены.
        name == "has_custom_resolution"
    }
}

pub fn launch(app: &AppHandle, settings: &LauncherSettings) -> Result<u32, String> {
    let nickname = settings.nickname.trim().to_string();
    if nickname.is_empty() || nickname.chars().count() > 16 {
        return Err("Введите ник: 1–16 символов (латиница, цифры, «_»)".into());
    }
    if !nickname.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Ник может содержать только латиницу, цифры и «_»".into());
    }

    let loader = settings.loader.to_lowercase();
    match loader.as_str() {
        "vanilla" | "fabric" => {}
        "forge" => {
            return Err(
                "Forge-загрузчик появится во 2-й части лаунчера. Пока выберите Vanilla или Fabric."
                    .into(),
            )
        }
        _ => return Err("Неизвестный загрузчик".into()),
    }
    let ram = settings.ram_gb.clamp(1, 16);

    let game_dir = match settings.game_dir.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => app
            .path()
            .app_local_data_dir()
            .map_err(|e| e.to_string())?
            .join("minecraft"),
    };
    for sub in ["versions", "libraries", "assets/indexes", "assets/objects", "natives"] {
        fs::create_dir_all(game_dir.join(sub))
            .map_err(|e| format!("Не удалось создать папки игры: {e}"))?;
    }

    let java_info = detect_java(app)?;
    let java_path = java_info
        .path
        .clone()
        .ok_or_else(|| "Java не найдена. Скачайте JDK во вкладке «Настройки»".to_string())?;
    let java_major = java_info.major.unwrap_or(21);

    emit_event(app, EVENT, "manifest", "Загружаем список версий Minecraft…", 2);
    let mc_id = settings.version_id.clone();
    let vanilla = load_vanilla_version(&mc_id)?;

    // Fabric: наложение официального loader-профиля поверх ванильной версии.
    let overlay: Option<Value> = if loader == "fabric" {
        emit_event(app, EVENT, "fabric", "Загружаем профиль Fabric…", 4);
        let url = format!("{FABRIC_META}/v2/versions/loader/{mc_id}/stable/json");
        let list = get_json(&url).map_err(|e| format!("Не удалось получить Fabric: {e}"))?;
        let profile = list
            .get(0)
            .and_then(|entry| entry.get("profile"))
            .cloned()
            .ok_or_else(|| "Для этой версии нет stable-сборки Fabric".to_string())?;
        Some(profile)
    } else {
        None
    };

    let profile_id = if overlay.is_some() {
        format!("fabric-{mc_id}")
    } else {
        mc_id.clone()
    };
    let profile = match &overlay {
        Some(o) => o,
        None => &vanilla,
    };
    let main_class = profile
        .get("mainClass")
        .and_then(Value::as_str)
        .or_else(|| vanilla.get("mainClass").and_then(Value::as_str))
        .unwrap_or("net.minecraft.client.main.Main")
        .to_string();

    emit_event(app, EVENT, "client", "Скачиваем клиент Minecraft…", 6);
    let client = vanilla
        .get("downloads")
        .and_then(|d| d.get("client"))
        .cloned()
        .unwrap_or(Value::Null);
    let client_url = client
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| "В манифесте версии нет ссылки на клиент".to_string())?
        .to_string();
    let client_jar = game_dir
        .join("versions")
        .join(&profile_id)
        .join(format!("{profile_id}.jar"));
    fs::create_dir_all(client_jar.parent().ok_or_else(|| "Некорректный путь к клиенту".to_string())?)
        .map_err(|e| e.to_string())?;
    download_file(
        &client_url,
        &client_jar,
        client.get("sha1").and_then(Value::as_str),
        client.get("size").and_then(Value::as_u64),
        Some(&|done, total| {
            let percent = match total {
                Some(t) if t > 0 => 6 + ((done as f64 / t as f64) * 8.0) as u32,
                _ => 6,
            };
            emit_event(
                app,
                EVENT,
                "client",
                format!("Клиент: {} МБ", done / (1024 * 1024)),
                percent.min(14),
            );
        }),
    )?;

    let natives_dir = game_dir.join("natives");
    let _ = fs::remove_dir_all(&natives_dir);
    fs::create_dir_all(&natives_dir).map_err(|e| e.to_string())?;
    let ctx = RulesContext {
        java_major,
        mc_version: mc_id.clone(),
    };

    emit_event(app, EVENT, "libraries", "Скачиваем библиотеки…", 15);
    let libraries = collect_libraries(&vanilla, &overlay);
    let total_libs = libraries.len().max(1);
    let mut classpath: Vec<PathBuf> = Vec::new();
    for (index, lib) in libraries.iter().enumerate() {
        let percent = 15 + ((index as f64 / total_libs as f64) * 28.0) as u32;
        match resolve_library(app, &game_dir, &natives_dir, &ctx, lib, percent) {
            Ok(Some(path)) => classpath.push(path),
            Ok(None) => {}
            Err(e) => return Err(e),
        }
    }
    classpath.push(client_jar);

    let index_id = ensure_assets(app, &game_dir, &vanilla, &overlay)?;

    run_game(
        app,
        &game_dir,
        &java_path,
        &vanilla,
        &overlay,
        &profile_id,
        &main_class,
        &classpath,
        &index_id,
        &nickname,
        ram,
        java_major,
        &mc_id,
    )
}

/// Ванильная версия из манифеста Mojang (нужна всегда: клиент + ассеты).
fn load_vanilla_version(mc_id: &str) -> Result<Value, String> {
    let manifest =
        get_json(MANIFEST_URL).map_err(|e| format!("Не удалось загрузить манифест версий: {e}"))?;
    let entry = manifest
        .get("versions")
        .and_then(Value::as_array)
        .and_then(|list| list.iter().find(|v| v.get("id").and_then(Value::as_str) == Some(mc_id)))
        .cloned()
        .ok_or_else(|| format!("Версия {mc_id} не найдена в списке Minecraft"))?;
    let url = entry
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| "Повреждённый манифест версий".to_string())?;
    get_json(url).map_err(|e| format!("Не удалось загрузить описание версии {mc_id}: {e}"))
}

fn collect_libraries(vanilla: &Value, overlay: &Option<Value>) -> Vec<Value> {
    let mut libs: Vec<Value> = vanilla
        .get("libraries")
        .and_then(Value::as_array)
        .map(|list| list.to_vec())
        .unwrap_or_default();
    if let Some(o) = overlay {
        if let Some(extra) = o.get("libraries").and_then(Value::as_array) {
            libs.extend(extra.iter().cloned());
        }
    }
    libs
}

/// Разрешает одну библиотеку: обычный jar (в класспасс), нативки (распаковка)
/// либо пропуск (правила не подходят / другая ОС).
fn resolve_library(
    app: &AppHandle,
    game: &Path,
    natives_dir: &Path,
    ctx: &RulesContext,
    lib: &Value,
    percent: u32,
) -> Result<Option<PathBuf>, String> {
    let rules = lib.get("rules");

    // Классические нативки (Minecraft 1.12 и младше): "natives": {"linux": "natives-linux", …}.
    if let Some(natives_map) = lib.get("natives").and_then(Value::as_object) {
        if natives_map.contains_key(os_name()) {
            if !rules_allow(rules, ctx) {
                return Ok(None);
            }
            let artifact = lib
                .get("downloads")
                .and_then(|d| d.get("artifact"))
                .ok_or_else(|| "Нет данных о natives-библиотеке".to_string())?;
            let url = artifact
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| "Нет ссылки на natives".to_string())?
                .to_string();
            let rel = artifact
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| "Нет пути natives-библиотеки".to_string())?;
            let file = download_file(
                &url,
                &game.join("libraries").join(rel),
                artifact.get("sha1").and_then(Value::as_str),
                artifact.get("size").and_then(Value::as_u64),
                None,
            )?;
            extract_jar_to_natives(&file, natives_dir, None)?;
            return Ok(None);
        }
        return Ok(None);
    }

    // Современные нативки (1.13+): "downloads.classifiers"{"natives-linux": …}.
    if let Some(classifiers) = lib
        .get("downloads")
        .and_then(|d| d.get("classifiers"))
        .and_then(Value::as_object)
    {
        let key = format!("natives-{}", os_name());
        if let Some(entry) = classifiers.get(&key) {
            if !rules_allow(rules, ctx) {
                return Ok(None);
            }
            let url = entry
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| "Нет ссылки на natives".to_string())?
                .to_string();
            let rel = entry.get("path").and_then(Value::as_str).unwrap_or(&key);
            let file = download_file(
                &url,
                &game.join("libraries").join(rel),
                entry.get("sha1").and_then(Value::as_str),
                entry.get("size").and_then(Value::as_u64),
                None,
            )?;
            let exclude = lib
                .get("extract")
                .and_then(|e| e.get("exclude"))
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect::<Vec<_>>()
                })
                .filter(|v| !v.is_empty());
            extract_jar_to_natives(&file, natives_dir, exclude.as_deref())?;
            return Ok(None);
        }
        return Ok(None);
    }

    if !rules_allow(rules, ctx) {
        return Ok(None);
    }

    // Обычный jar в формате Mojang (downloads.artifact).
    let artifact = lib
        .get("downloads")
        .and_then(|d| d.get("artifact"))
        .filter(|a| !a.is_null());
    if let Some(artifact) = artifact {
        let Some(url) = artifact.get("url").and_then(Value::as_str) else {
            return Ok(None);
        };
        let rel = artifact
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "Нет пути библиотеки".to_string())?;
        let file = download_file(
            url,
            &game.join("libraries").join(rel),
            artifact.get("sha1").and_then(Value::as_str),
            artifact.get("size").and_then(Value::as_u64),
            Some(&move |_, _| emit_event(app, EVENT, "libraries", "Библиотеки…", percent)),
        )?;
        return Ok(Some(file));
    }

    // Maven-библиотеки (Fabric и др.): "name": "group:artifact:version[:classifier]", "url": base.
    if let Some(name) = lib.get("name").and_then(Value::as_str) {
        let base = lib
            .get("url")
            .and_then(Value::as_str)
            .unwrap_or("https://libraries.minecraft.net/");
        let (url, rel) = maven_coordinates(name, base).ok_or_else(|| "Некорректные maven-координаты".to_string())?;
        let file = download_file(
            &url,
            &game.join("libraries").join(&rel),
            None,
            None,
            Some(&move |_, _| emit_event(app, EVENT, "libraries", "Библиотеки…", percent)),
        )?;
        return Ok(Some(file));
    }

    Ok(None)
}

/// "group:artifact:version[:classifier]" + base URL → (ссылка, относительный путь).
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

// ---------- правила манифестов ----------

fn rules_allow(rules: Option<&Value>, ctx: &RulesContext) -> bool {
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

fn rule_matches(rule: &Value, ctx: &RulesContext) -> bool {
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
            if !ctx.feature_enabled(name) {
                return false;
            }
        } else if let Some(obj) = features.as_object() {
            for (name, value) in obj {
                let want = value.as_bool().unwrap_or(false);
                if want != ctx.feature_enabled(name) {
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
    }
    true
}

/// Сравнение версий вида "1.20.5" по числовым компонентам.
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let pa = a.split('.').filter_map(|p| p.parse::<u64>().ok()).collect::<Vec<u64>>();
    let pb = b.split('.').filter_map(|p| p.parse::<u64>().ok()).collect::<Vec<u64>>();
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

// ---------- ассеты ----------

fn ensure_assets(
    app: &AppHandle,
    game: &Path,
    vanilla: &Value,
    overlay: &Option<Value>,
) -> Result<String, String> {
    emit_event(app, EVENT, "assets", "Загружаем индекс ассетов…", 45);
    let profile = match overlay {
        Some(o) => o,
        None => vanilla,
    };
    let index = profile
        .get("assetIndex")
        .or_else(|| vanilla.get("assetIndex"))
        .ok_or_else(|| "Нет данных об индексе ассетов".to_string())?;
    let index_id = index
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "Нет id индекса ассетов".to_string())?
        .to_string();
    let index_url = index
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| "Нет ссылки на индекс ассетов".to_string())?
        .to_string();
    let index_path = game
        .join("assets")
        .join("indexes")
        .join(format!("{index_id}.json"));
    download_file(
        &index_url,
        &index_path,
        index.get("sha1").and_then(Value::as_str),
        index.get("size").and_then(Value::as_u64),
        None,
    )?;
    let raw = fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
    let parsed: Value = raw
        .parse()
        .map_err(|e| format!("Повреждённый индекс ассетов: {e}"))?;

    // Современный формат: "map" {hash: name}, классический: "objects" {name: {hash, size}}.
    let mut assets: Vec<(String, String, Option<u64>)> = Vec::new();
    if let Some(map) = parsed.get("map").and_then(Value::as_object) {
        for (hash, name) in map {
            let name_str = name.as_str().unwrap_or_default();
            let file_name = if name_str.is_empty() {
                hash.clone()
            } else {
                name_str.to_string()
            };
            assets.push((file_name, hash.clone(), None));
        }
    } else if let Some(objects) = parsed.get("objects").and_then(Value::as_object) {
        for (name, value) in objects {
            assets.push((
                name.clone(),
                value
                    .get("hash")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                value.get("size").and_then(Value::as_u64),
            ));
        }
    } else {
        return Err("Пустой или неизвестный формат индекса ассетов".to_string());
    }
    assets.sort();

    let objects_dir = game.join("assets").join("objects");
    let total = assets.len().max(1);
    for (i, (_, hash, size)) in assets.iter().enumerate() {
        if hash.len() < 2 {
            continue;
        }
        let dest = objects_dir.join(&hash[..2]).join(hash);
        if dest.exists() {
            continue;
        }
        let url = format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash);
        emit_event(
            app,
            EVENT,
            "assets",
            format!("Ассеты: {}/{}", i + 1, assets.len()),
            45 + ((i as f64 / total as f64) * 25.0) as u32,
        );
        download_file(&url, &dest, None, *size, None)?;
    }
    emit_event(app, EVENT, "assets", "Ассеты готовы", 70);
    Ok(index_id)
}

// ---------- запуск процесса ----------

fn run_game(
    app: &AppHandle,
    game: &Path,
    java_path: &str,
    vanilla: &Value,
    overlay: &Option<Value>,
    profile_id: &str,
    main_class: &str,
    classpath: &[PathBuf],
    index_id: &str,
    nickname: &str,
    ram: u32,
    java_major: u32,
    mc_version: &str,
) -> Result<u32, String> {
    let ctx = RulesContext {
        java_major,
        mc_version: mc_version.to_string(),
    };

    // Офлайн-аутентификация: детерминированный offline-UUID (MD5 «OfflinePlayer:<ник>»,
    // разряды версий/варианта по RFC 4122) и произвольный токен сессии.
    let auth_uuid = offline_uuid(nickname);
    let token = random_token();

    let user_props = game
        .join("versions")
        .join(profile_id)
        .join("user_properties.json");
    let user_props_content = if compare_versions(mc_version, "1.12") != std::cmp::Ordering::Less {
        serde_json::json!({
            "userProperties": {
                "username": nickname,
                "uuid": auth_uuid
            }
        })
        .to_string()
    } else {
        format!("username={nickname}\nuuid={auth_uuid}\n")
    };
    fs::write(&user_props, user_props_content).map_err(|e| e.to_string())?;

    let separator = if cfg!(windows) { ";" } else { ":" };
    let classpath_value = classpath
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(separator);

    let mut props: BTreeMap<String, String> = BTreeMap::new();
    let mut insert = |key: &str, value: String| {
        props.insert(key.to_string(), value);
    };
    insert("auth_player_name", nickname.to_string());
    insert("auth_uuid", auth_uuid.clone());
    insert("auth_access_token", token.clone());
    insert("auth_session", format!("token:{token}:{auth_uuid}"));
    insert("user_type", "legacy".to_string());
    insert("version_name", profile_id.to_string());
    insert("version_type", "release".to_string());
    insert("game_directory", game.to_string_lossy().into_owned());
    insert("assets_root", game.join("assets").to_string_lossy().into_owned());
    insert(
        "game_assets",
        game.join("assets")
            .join("objects")
            .to_string_lossy()
            .into_owned(),
    );
    insert("assets_index_name", index_id.to_string());
    insert(
        "natives_directory",
        game.join("natives").to_string_lossy().into_owned(),
    );
    insert("launcher_name", "MyLauncher".to_string());
    insert("launcher_version", env!("CARGO_PKG_VERSION").to_string());
    insert("classpath", classpath_value);
    insert("classpath_separator", separator.to_string());
    insert(
        "libraryDirectory",
        game.join("libraries").to_string_lossy().into_owned(),
    );
    insert("clientid", String::new());
    insert("auth_xuid", String::new());
    insert("quickPlayMultiplayer", String::new());
    insert("quickPlaySingleplayer", String::new());
    insert("resolution_width", "1280".to_string());
    insert("resolution_height", "720".to_string());
    insert(
        "user_properties_file",
        user_props.to_string_lossy().into_owned(),
    );
    drop(insert);

    let profile = match overlay {
        Some(o) => o,
        None => vanilla,
    };

    let mut args: Vec<String> = Vec::new();
    args.push(format!("-Xms{}G", (ram / 2).max(1)));
    args.push(format!("-Xmx{ram}G"));

    let jvm_items = profile
        .get("arguments")
        .and_then(|a| a.get("jvm"))
        .or_else(|| vanilla.get("arguments").and_then(|a| a.get("jvm")));
    let mut jvm_args: Vec<String> = if let Some(items) = jvm_items {
        eval_items(items, &ctx, &props)
    } else {
        vec![
            substitute("-Djava.library.path=${natives_directory}", &props),
            "-cp".to_string(),
            substitute("${classpath}", &props),
        ]
    };
    if !jvm_args.iter().any(|a| a == "-cp") {
        let mut prefix = vec![
            substitute("-Djava.library.path=${natives_directory}", &props),
            "-cp".to_string(),
            substitute("${classpath}", &props),
        ];
        prefix.append(&mut jvm_args);
        jvm_args = prefix;
    }
    args.extend(jvm_args);

    args.push(main_class.to_string());
    let game_items = profile
        .get("arguments")
        .and_then(|a| a.get("game"))
        .or_else(|| vanilla.get("arguments").and_then(|a| a.get("game")));
    if let Some(items) = game_items {
        args.extend(eval_items(items, &ctx, &props));
    } else if let Some(legacy) = vanilla.get("minecraftArguments").and_then(Value::as_str) {
        for token in legacy.split_whitespace() {
            args.push(substitute(token, &props));
        }
    } else {
        return Err("В описании версии нет игровых аргументов".to_string());
    }

    emit_event(app, EVENT, "launch", "Запускаем игру…", 95);
    let log = fs::File::create(game.join("launcher.log"))
        .map_err(|e| format!("Не удалось открыть файл launcher.log: {e}"))?;
    let log_err = log.try_clone().map_err(|e| e.to_string())?;
    let mut command = Command::new(java_path);
    command.args(&args);
    command.current_dir(game);
    command.stdout(Stdio::from(log));
    command.stderr(Stdio::from(log_err));
    let child = command
        .spawn()
        .map_err(|e| format!("Не удалось запустить Java: {e}"))?;
    let pid = child.id();
    // Игра продолжает работать независимо от лаунчера.
    drop(child);
    Ok(pid)
}

/// Строка/объект-аргумент из "arguments" → список готовых аргументов.
fn eval_items(items: &Value, ctx: &RulesContext, props: &BTreeMap<String, String>) -> Vec<String> {
    let mut out = Vec::new();
    // `as_array` возвращает Option<&Vec<Value>>: разворачиваем его, иначе `item`
    // получает тип &Vec<Value> и методы Value (as_str/get) не работают.
    let Some(list) = items.as_array() else {
        return out;
    };
    for item in list {
        if let Some(s) = item.as_str() {
            out.push(substitute(s, props));
            continue;
        }
        if let Some(rules) = item.get("rules") {
            if !rules_allow(Some(rules), ctx) {
                continue;
            }
        }
        let value = item.get("value").unwrap_or(item);
        if let Some(s) = value.as_str() {
            out.push(substitute(s, props));
        } else if let Some(arr) = value.as_array() {
            for s in arr.iter().filter_map(Value::as_str) {
                out.push(substitute(s, props));
            }
        }
    }
    out
}

/// Подстановка плейсхолдеров ${key}.
fn substitute(input: &str, props: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                out.push_str(props.get(key).map(|s| s.as_str()).unwrap_or(""));
                rest = &after[end + 1..];
            }
            None => {
                out.push_str("${");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

// ---------- скачивание с проверкой SHA-1 ----------

fn download_file(
    url: &str,
    dest: &Path,
    sha1_expected: Option<&str>,
    size_expected: Option<u64>,
    progress: Option<&dyn Fn(u64, Option<u64>)>,
) -> Result<PathBuf, String> {
    let cached_ok = sha1_expected.map(|expected| {
        file_sha1(dest)
            .as_deref()
            .is_some_and(|actual| actual == &expected.to_lowercase())
    });
    if dest.exists() && (sha1_expected.is_none() || cached_ok.unwrap_or(false)) {
        return Ok(dest.to_path_buf());
    }
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut response =
        reqwest::blocking::get(url).map_err(|e| format!("Сбой сети при загрузке {url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Сервер вернул статус {} для {url}", response.status()));
    }
    // content_length — метод, а блокирующий Response — обычный Read (метода chunk() у него нет).
    let total = size_expected.or_else(|| response.content_length());

    let mut part_name = dest.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    part_name.push(".part");
    let part = dest.with_file_name(part_name);
    let mut file = fs::File::create(&part).map_err(|e| format!("Не удалось создать {part:?}: {e}"))?;
    let mut hasher = Sha1::new();
    let mut done: u64 = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = match response.read(&mut buffer) {
            Ok(read) => read,
            Err(e) => {
                drop(file);
                let _ = fs::remove_file(&part);
                return Err(format!("Сбой сети при загрузке {url}: {e}"));
            }
        };
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|e| format!("Не удалось записать {part:?}: {e}"))?;
        hasher.update(&buffer[..read]);
        done += read as u64;
        if let Some(f) = progress {
            f(done, total);
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);
    if let Some(expected) = sha1_expected {
        let got: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if got != expected.to_lowercase() {
            let _ = fs::remove_file(&part);
            return Err("Файл не прошёл проверку SHA-1 — запустите запуск ещё раз".to_string());
        }
    }
    if dest.exists() {
        let _ = fs::remove_file(dest);
    }
    fs::rename(&part, dest).map_err(|e| format!("Не удалось сохранить {dest:?}: {e}"))?;
    Ok(dest.to_path_buf())
}

fn file_sha1(path: &Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    Some(Sha1::digest(&data).iter().map(|b| format!("{b:02x}")).collect())
}

// ---------- нативки ----------

fn extract_jar_to_natives(jar: &Path, dest: &Path, exclude: Option<&[String]>) -> Result<(), String> {
    let file = fs::File::open(jar).map_err(|e| e.to_string())?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Повреждённый jar {jar:?}: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if name.ends_with('/') {
            continue;
        }
        if let Some(list) = exclude {
            if list.iter().any(|prefix| name.starts_with(prefix)) {
                continue;
            }
        }
        let out = dest.join(&name);
        if !out.starts_with(dest) {
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out_file = fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out_file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ---------- офлайн-аутентификация ----------

/// Детерминированный offline-UUID (как в мультиплеере без Mojang):
/// MD5("OfflinePlayer:<ник>") + биты версии 3 и варианта по RFC 4122, без дефисов.
fn offline_uuid(name: &str) -> String {
    let digest = md5::compute(format!("OfflinePlayer:{name}").as_bytes());
    let mut bytes = digest.0.to_vec();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

static TOKEN_COUNTER: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);

fn random_token() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    let counter = TOKEN_COUNTER.fetch_add(0x2545_f491_4f6c_dd1d, Ordering::Relaxed);
    let digest = md5::compute(format!("{now}:{counter}").as_bytes());
    digest.0.iter().map(|b| format!("{b:02x}")).collect()
}
