//! Tauri-команды, которые раньше жили прямо в корне крейта (`lib.rs`).
//!
//! Почему отдельный модуль: `#[tauri::command]` на `pub`-функции генерирует
//! `#[macro_export] macro_rules! __cmd__<имя>` и следом `pub use __cmd__<имя>;`.
//! `#[macro_export]` кладёт макрос в корень крейта, поэтому в самом `lib.rs`
//! этот `use` превращается в повторное объявление того же имени —
//! `error[E0255]: the name __cmd__<имя> is defined multiple times`. В дочернем
//! модуле импорт не конфликтует с корнем крейта.

use crate::{java, jdk, minecraft, settings, JavaInfo, LauncherSettings};
use tauri::AppHandle;

#[tauri::command]
pub async fn get_java(app: AppHandle) -> Result<JavaInfo, String> {
    // spawn_blocking возвращает JoinHandle<Result<_, _>>, поэтому после `await`
    // нужен `?`: иначе получится вложенный Result<Result<_, _>, _> (E0308).
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
    tauri::async_runtime::spawn_blocking(move || settings::load(&app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: LauncherSettings) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || crate::settings::save(&app, &settings))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn launch_game(app: AppHandle, settings: LauncherSettings) -> Result<u32, String> {
    tauri::async_runtime::spawn_blocking(move || minecraft::launch(&app, &settings))
        .await
        .map_err(|e| e.to_string())?
}
