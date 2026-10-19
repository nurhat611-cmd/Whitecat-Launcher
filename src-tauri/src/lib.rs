//! MyLauncher — бэкенд (lighty-launcher).
//!
//! Часть 1/4: офлайн-запуск Minecraft (без аккаунта Mojang), автоопределение Java,
//! скачивание JDK с прогрессом, полный пайплайн установки клиента (Vanilla + Fabric).
//!
//! Часть 2/4: список версий Mojang (piston-meta) с кешем и фоновым обновлением,
//! многопоточное скачивание игры с прогрессом, поиск и установка контента из
//! Modrinth (моды, ресурспаки, шейдеры, сборки .mrpack) через очередь загрузок.
//!
//! Часть 3/4: скины (URL/файл, превью Steve/Alex, папка skins), красивый экран настроек
//! (общие/игра/java/аккаунты/обновления), модальное окно ошибок с парсингом логов.

mod installer;
mod java;
mod jdk;
mod logs;
mod minecraft;
mod modrinth;
mod queue;
mod settings;
mod skin;
mod updater;
mod versions;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

/// Офлайн-профиль (аккаунт)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineProfile {
    pub name: String,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub created_at: Option<u64>,
}

/// Настройки, управляемые из интерфейса.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    pub nickname: String,
    pub version_id: String,
    /// "vanilla" | "fabric" | "forge"
    pub loader: String,
    pub ram_gb: u32,
    pub game_dir: Option<String>,

    // --- 3/4 additions ---
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub jvm_args: String,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default = "default_skin_model")]
    pub skin_model: String,
    #[serde(default)]
    pub skin_url: Option<String>,
    #[serde(default)]
    pub skin_path: Option<String>,
    #[serde(default)]
    pub profiles: Vec<OfflineProfile>,
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,
}

fn default_language() -> String {
    "ru".to_string()
}
fn default_theme() -> String {
    "dark".to_string()
}
fn default_skin_model() -> String {
    "steve".to_string()
}
fn default_auto_update() -> bool {
    true
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            nickname: String::new(),
            version_id: "1.21.4".to_string(),
            loader: "vanilla".to_string(),
            ram_gb: 4,
            game_dir: None,
            language: "ru".to_string(),
            theme: "dark".to_string(),
            jvm_args: String::new(),
            java_path: None,
            skin_model: "steve".to_string(),
            skin_url: None,
            skin_path: None,
            profiles: Vec::new(),
            auto_update: true,
        }
    }
}

/// Результат автоопределения Java.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInfo {
    pub found: bool,
    pub path: Option<String>,
    /// Первая строка `java -version`, например `openjdk version "21.0.5" 2024-10-15`.
    pub version: Option<String>,
    /// Крупная версия: 21, 17, 8…
    pub major: Option<u32>,
    /// Откуда нашли: «Встроенный JDK», `JAVA_HOME`, `PATH`, «Системная установка».
    pub source: String,
}

/// Событие прогресса (jdk_progress / launch_progress).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub stage: String,
    pub message: String,
    pub percent: u32,
}

/// Имя ОС в формате манифестов Mojang.
pub fn os_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

pub(crate) fn emit_event(
    app: &AppHandle,
    event: &str,
    stage: &str,
    message: impl Into<String>,
    percent: u32,
) {
    let payload = ProgressEvent {
        stage: stage.to_string(),
        message: message.into(),
        percent,
    };
    let _ = app.emit(event, payload);
}

/// GET-запрос JSON с User-Agent лаунчера (его требуют Mojang и Modrinth).
pub(crate) fn http_json(url: &str) -> Result<serde_json::Value, String> {
    let client = installer::http_client()?;
    let response = client
        .get(url)
        .send()
        .map_err(|e| format!("Сетевая ошибка: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Сервер вернул статус {} для {url}",
            response.status()
        ));
    }
    response
        .json::<serde_json::Value>()
        .map_err(|e| format!("Некорректный JSON: {e}"))
}

pub(crate) fn get_json(url: &str) -> Result<serde_json::Value, String> {
    http_json(url)
}

#[tauri::command]
pub async fn get_java(app: AppHandle) -> Result<JavaInfo, String> {
    tauri::async_runtime::spawn_blocking(move || java::detect_java(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn download_jdk(app: AppHandle) -> Result<JavaInfo, String> {
    tauri::async_runtime::spawn_blocking(move || jdk::download_jdk(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn load_settings(app: AppHandle) -> Result<LauncherSettings, String> {
    Ok(settings::load(&app)?)
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: LauncherSettings) -> Result<(), String> {
    crate::settings::save(&app, &settings)
}

#[tauri::command]
pub async fn launch_game(app: AppHandle, settings: LauncherSettings) -> Result<u32, String> {
    let config = settings.clone();
    tauri::async_runtime::spawn_blocking(move || minecraft::launch(&app, &config))
        .await
        .map_err(|e| e.to_string())?
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            // Фоновое обновление списка версий + кеш на диске.
            versions::spawn_background_refresh(handle.clone());
            // Очередь загрузок живёт всё время работы приложения.
            app.manage(queue::DownloadQueue::start(handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_java,
            download_jdk,
            load_settings,
            save_settings,
            launch_game,
            versions::get_versions,
            modrinth::search_content,
            modrinth::get_content_versions,
            modrinth::list_instances,
            queue::get_download_queue,
            queue::enqueue_version,
            queue::enqueue_content,
            queue::cancel_download,
            queue::remove_download,
            queue::clear_finished_downloads,
            skin::get_skin_info,
            skin::save_skin_url,
            skin::save_skin_file,
            skin::delete_skin_cmd,
            skin::open_skins_folder_cmd,
            logs::get_game_logs,
            logs::open_logs_folder,
            logs::open_game_folder,
            logs::get_launcher_log,
            updater::check_for_updates
        ])
        .run(tauri::generate_context!())
        .expect("error while running MyLauncher");
}
