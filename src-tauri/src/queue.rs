//! Очередь загрузок: последовательный воркер, прогресс по каждой задаче,
//! отмена и события для интерфейса (`download_queue`).

use crate::{installer, modrinth, settings};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

pub const EVENT: &str = "download_queue";
static TASK_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub percent: u32,
    pub message: String,
    pub current_file: String,
    pub done_files: u32,
    pub total_files: u32,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_bps: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTask {
    pub id: String,
    /// version | mod | resourcepack | shader | modpack
    pub kind: String,
    pub title: String,
    pub subtitle: String,
    pub icon_url: Option<String>,
    pub project_id: Option<String>,
    /// queued | running | done | error | cancelled
    pub status: String,
    pub progress: TaskProgress,
    pub error: Option<String>,
    pub target: Option<String>,
    pub created_at: u64,
}

#[derive(Clone)]
struct JobRequest {
    kind: String,
    project_id: String,
    project_type: String,
    title: String,
    slug: String,
    version_id: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    icon_url: Option<String>,
}

struct Inner {
    app: AppHandle,
    tasks: Mutex<Vec<DownloadTask>>,
    requests: Mutex<HashMap<String, JobRequest>>,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    last_emit: Mutex<Instant>,
    tx: Sender<u64>,
    rx: Mutex<Receiver<u64>>,
}

/// Разделяемое состояние очереди (управляется Tauri как managed state).
#[derive(Clone)]
pub struct DownloadQueue {
    inner: Arc<Inner>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn new_task_id() -> String {
    format!(
        "{}-{}",
        now_millis(),
        TASK_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn loader_label(loader: &str) -> String {
    match loader.to_lowercase().as_str() {
        "fabric" => "Fabric".to_string(),
        "forge" => "Forge".to_string(),
        "neoforge" => "NeoForge".to_string(),
        "quilt" => "Quilt".to_string(),
        _ => "Vanilla".to_string(),
    }
}

impl DownloadQueue {
    pub fn start(app: AppHandle) -> Self {
        let (tx, rx) = mpsc::channel::<u64>();
        let inner = Arc::new(Inner {
            app,
            tasks: Mutex::new(Vec::new()),
            requests: Mutex::new(HashMap::new()),
            cancels: Mutex::new(HashMap::new()),
            last_emit: Mutex::new(Instant::now() - Duration::from_secs(10)),
            tx,
            rx: Mutex::new(rx),
        });
        let queue = Self { inner };
        let worker = queue.clone();
        std::thread::spawn(move || worker.worker_loop());
        queue
    }

    pub fn snapshot(&self) -> Vec<DownloadTask> {
        lock(&self.inner.tasks).clone()
    }

    fn emit(&self, force: bool) {
        if force {
            *lock(&self.inner.last_emit) = Instant::now();
        } else {
            let mut last = lock(&self.inner.last_emit);
            if last.elapsed() < Duration::from_millis(120) {
                return;
            }
            *last = Instant::now();
        }
        let _ = self.inner.app.emit(EVENT, self.snapshot());
    }

    fn update<F: FnOnce(&mut DownloadTask)>(&self, id: &str, apply: F) {
        let mut tasks = lock(&self.inner.tasks);
        if let Some(task) = tasks.iter_mut().find(|task| task.id == id) {
            apply(task);
        }
    }

    fn set_progress(&self, id: &str, progress: installer::Progress, force: bool) {
        self.update(id, |task| {
            task.progress = TaskProgress {
                percent: progress.percent.min(100),
                message: progress.message,
                current_file: progress.current_file,
                done_files: progress.done_files,
                total_files: progress.total_files,
                downloaded_bytes: progress.downloaded_bytes,
                total_bytes: progress.total_bytes,
                speed_bps: progress.speed_bps,
            };
        });
        self.emit(force);
    }

    fn enqueue(&self, task: DownloadTask, request: JobRequest) -> String {
        let id = task.id.clone();
        lock(&self.inner.requests).insert(id.clone(), request);
        lock(&self.inner.cancels).insert(id.clone(), Arc::new(AtomicBool::new(false)));
        lock(&self.inner.tasks).push(task);
        let _ = self.inner.tx.send(0);
        self.emit(true);
        id
    }

    pub fn enqueue_version(&self, version_id: &str, loader: &str) -> String {
        let task = DownloadTask {
            id: new_task_id(),
            kind: "version".to_string(),
            title: format!("Minecraft {version_id}"),
            subtitle: loader_label(loader),
            icon_url: None,
            project_id: None,
            status: "queued".to_string(),
            progress: TaskProgress {
                message: "В очереди".to_string(),
                ..Default::default()
            },
            error: None,
            target: None,
            created_at: now_millis(),
        };
        self.enqueue(
            task,
            JobRequest {
                kind: "version".to_string(),
                project_id: String::new(),
                project_type: "version".to_string(),
                title: format!("Minecraft {version_id}"),
                slug: version_id.to_string(),
                version_id: Some(version_id.to_string()),
                game_version: Some(version_id.to_string()),
                loader: Some(loader.to_string()),
                icon_url: None,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enqueue_content(
        &self,
        project_id: &str,
        project_type: &str,
        title: &str,
        subtitle: &str,
        icon_url: Option<&str>,
        game_version: Option<&str>,
        loader: Option<&str>,
        version_id: Option<&str>,
    ) -> String {
        let task = DownloadTask {
            id: new_task_id(),
            kind: project_type.to_string(),
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            icon_url: icon_url.map(str::to_string),
            project_id: Some(project_id.to_string()),
            status: "queued".to_string(),
            progress: TaskProgress {
                message: "В очереди".to_string(),
                ..Default::default()
            },
            error: None,
            target: None,
            created_at: now_millis(),
        };
        self.enqueue(
            task,
            JobRequest {
                kind: project_type.to_string(),
                project_id: project_id.to_string(),
                project_type: project_type.to_string(),
                title: title.to_string(),
                slug: String::new(),
                version_id: version_id.map(str::to_string),
                game_version: game_version.map(str::to_string),
                loader: loader.map(str::to_string),
                icon_url: icon_url.map(str::to_string),
            },
        )
    }

    pub fn cancel(&self, id: &str) {
        if let Some(flag) = lock(&self.inner.cancels).get(id).cloned() {
            flag.store(true, Ordering::Relaxed);
        }
        let mut was_queued = false;
        self.update(id, |task| {
            if task.status == "queued" {
                task.status = "cancelled".to_string();
                task.progress.message = "Отменено".to_string();
                was_queued = true;
            } else if task.status == "running" {
                task.progress.message = "Отмена…".to_string();
            }
        });
        if was_queued {
            lock(&self.inner.requests).remove(id);
        }
        self.emit(true);
    }

    pub fn remove(&self, id: &str) {
        let removable = {
            let tasks = lock(&self.inner.tasks);
            tasks
                .iter()
                .find(|task| task.id == id)
                .map(|task| task.status != "running" && task.status != "queued")
                .unwrap_or(false)
        };
        if !removable {
            return;
        }
        lock(&self.inner.tasks).retain(|task| task.id != id);
        lock(&self.inner.requests).remove(id);
        lock(&self.inner.cancels).remove(id);
        self.emit(true);
    }

    pub fn clear_finished(&self) {
        lock(&self.inner.tasks)
            .retain(|task| task.status == "running" || task.status == "queued");
        self.emit(true);
    }

    fn worker_loop(self) {
        loop {
            let next = {
                let mut tasks = lock(&self.inner.tasks);
                tasks
                    .iter_mut()
                    .find(|task| task.status == "queued")
                    .map(|task| {
                        task.status = "running".to_string();
                        task.progress.message = "Подготовка…".to_string();
                        task.id.clone()
                    })
            };

            let Some(id) = next else {
                let rx = lock(&self.inner.rx);
                match rx.recv_timeout(Duration::from_millis(400)) {
                    Ok(_) | Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            };

            self.emit(true);
            let request = lock(&self.inner.requests).get(&id).cloned();
            let cancel = lock(&self.inner.cancels).get(&id).cloned();

            match request {
                Some(request) => {
                    let result = self.execute(&id, &request, cancel);
                    let mut cancelled = false;
                    self.update(&id, |task| match result {
                        Ok(target) => {
                            task.status = "done".to_string();
                            task.progress.percent = 100;
                            task.progress.message = "Готово".to_string();
                            task.progress.current_file = String::new();
                            task.target = target;
                        }
                        Err(error) if error == installer::CANCELLED => {
                            cancelled = true;
                            task.status = "cancelled".to_string();
                            task.progress.message = "Отменено".to_string();
                        }
                        Err(error) => {
                            task.status = "error".to_string();
                            task.progress.message = error.clone();
                            task.error = Some(error);
                        }
                    });
                    if cancelled {
                        self.update(&id, |task| task.progress.percent = 0);
                    }
                }
                None => {
                    self.update(&id, |task| {
                        task.status = "error".to_string();
                        task.error = Some("Задача потеряна".to_string());
                    });
                }
            }

            lock(&self.inner.requests).remove(&id);
            lock(&self.inner.cancels).remove(&id);
            self.emit(true);
        }
    }

    fn execute(
        &self,
        id: &str,
        request: &JobRequest,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Result<Option<String>, String> {
        let game = settings::game_dir(&self.inner.app)?;
        let report = |progress: installer::Progress| {
            self.set_progress(id, progress, false);
        };

        match request.kind.as_str() {
            "version" => {
                let version_id = request.version_id.clone().unwrap_or_default();
                let loader = request
                    .loader
                    .clone()
                    .unwrap_or_else(|| "vanilla".to_string());
                self.update(id, |task| {
                    task.progress.message = "Готовим список файлов…".to_string();
                });
                self.emit(true);
                let plan =
                    installer::build_plan(&self.inner.app, &game, &version_id, &loader)?;
                self.update(id, |task| {
                    task.subtitle = format!(
                        "{} · {} файлов",
                        loader_label(&plan.loader),
                        plan.jobs.len()
                    );
                });
                let stage = format!("Minecraft {} · {}", plan.mc_version, loader_label(&plan.loader));
                installer::download_all(plan.jobs, &stage, cancel, &report)?;
                Ok(Some(game.to_string_lossy().into_owned()))
            }
            "modpack" | "mod" | "resourcepack" | "shader" | "datapack" => {
                let install = modrinth::InstallRequest {
                    project_type: request.project_type.clone(),
                    project_id: request.project_id.clone(),
                    title: request.title.clone(),
                    slug: request.slug.clone(),
                    version_id: request.version_id.clone(),
                    game_version: request.game_version.clone(),
                    loader: request.loader.clone(),
                    icon_url: request.icon_url.clone(),
                };
                modrinth::install(&install, &game, cancel, &report)
            }
            other => Err(format!("Неизвестный тип задачи: {other}")),
        }
    }
}

// ---------- Tauri-команды ----------

#[tauri::command]
pub fn get_download_queue(queue: tauri::State<'_, DownloadQueue>) -> Vec<DownloadTask> {
    queue.snapshot()
}

#[tauri::command]
pub fn enqueue_version(
    queue: tauri::State<'_, DownloadQueue>,
    version_id: String,
    loader: String,
) -> Result<String, String> {
    if version_id.trim().is_empty() {
        return Err("Не выбрана версия Minecraft".into());
    }
    Ok(queue.enqueue_version(&version_id, &loader))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn enqueue_content(
    queue: tauri::State<'_, DownloadQueue>,
    project_id: String,
    project_type: String,
    title: String,
    subtitle: Option<String>,
    icon_url: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    version_id: Option<String>,
) -> Result<String, String> {
    if project_id.trim().is_empty() {
        return Err("Не указан проект Modrinth".into());
    }
    let subtitle = subtitle
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| project_type.clone());
    Ok(queue.enqueue_content(
        &project_id,
        &project_type,
        &title,
        &subtitle,
        icon_url.as_deref(),
        game_version.as_deref(),
        loader.as_deref(),
        version_id.as_deref(),
    ))
}

#[tauri::command]
pub fn cancel_download(queue: tauri::State<'_, DownloadQueue>, id: String) {
    queue.cancel(&id);
}

#[tauri::command]
pub fn remove_download(queue: tauri::State<'_, DownloadQueue>, id: String) {
    queue.remove(&id);
}

#[tauri::command]
pub fn clear_finished_downloads(queue: tauri::State<'_, DownloadQueue>) {
    queue.clear_finished();
}
