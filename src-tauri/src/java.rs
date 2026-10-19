//! Автоопределение Java: встроенный JDK (скачан лаунчером) → JAVA_HOME → PATH → типовые системные каталоги.

use crate::JavaInfo;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};

pub fn java_bin_name() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

pub fn detect_java(app: &AppHandle) -> Result<JavaInfo, String> {
    // 1. JDK, установленный самим лаунчером (com.lighty.mylauncher/jdk).
    if let Ok(data) = app.path().app_local_data_dir() {
        let bundled = data.join("jdk");
        if let Some(path) = find_java(&bundled) {
            return probe(&path, "Встроенный JDK (скачан лаунчером)");
        }
    }

    // 2. JAVA_HOME.
    if let Ok(home) = std::env::var("JAVA_HOME") {
        let path = PathBuf::from(home.trim()).join("bin").join(java_bin_name());
        if path.is_file() {
            return probe(&path, "JAVA_HOME");
        }
    }

    // 3. PATH.
    if let Ok(path) = which::which("java") {
        if path.is_file() {
            return probe(&path, "PATH");
        }
    }

    // 4. Типовые системные каталоги установщиков JDK.
    for path in system_candidates() {
        if path.is_file() {
            if let Ok(info) = probe(&path, "Системная установка") {
                if info.found {
                    return Ok(info);
                }
            }
        }
    }

    Ok(JavaInfo {
        found: false,
        path: None,
        version: None,
        major: None,
        source: String::new(),
    })
}

/// Ищет `bin/java(.exe)` прямо в `root` или в первом вложенном каталоге
/// (распакованный JDK всегда содержит одну корневую папку вида `jdk-21.0.5+11`).
fn find_java(root: &Path) -> Option<PathBuf> {
    let direct = root.join("bin").join(java_bin_name());
    if direct.is_file() {
        return Some(direct);
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return None;
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let path = dir.join("bin").join(java_bin_name());
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn system_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        let program_files =
            std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
        let bases = [
            PathBuf::from(&program_files).join("Java"),
            PathBuf::from(&program_files).join("Eclipse Adoptium"),
            PathBuf::from(&program_files).join("Microsoft"),
            PathBuf::from(&program_files).join("BellSoft"),
            PathBuf::from(&program_files).join("Amazon Corretto"),
            PathBuf::from(&program_files).join("Zulu"),
            PathBuf::from(&program_files).join("Gluon"),
        ];
        for base in bases {
            collect_java_dirs(&base, &mut out);
        }
    } else if cfg!(target_os = "macos") {
        collect_java_dirs(Path::new("/Library/Java/JavaVirtualMachines"), &mut out);
    } else {
        collect_java_dirs(Path::new("/usr/lib/jvm"), &mut out);
        collect_java_dirs(Path::new("/opt/jdk"), &mut out);
    }
    out
}

fn collect_java_dirs(base: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        out.push(dir.join("bin").join(java_bin_name()));
    }
}

/// Запускает `java -version` (вывод приходит в stderr) и парсит версию.
fn probe(path: &Path, source: &str) -> Result<JavaInfo, String> {
    let output = Command::new(path)
        .arg("-version")
        .output()
        .map_err(|e| format!("Не удалось выполнить java -version: {e}"))?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let first_line = combined
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .to_string();
    let (version, major) = if first_line.is_empty() {
        (None, None)
    } else {
        (Some(first_line.clone()), extract_major(&first_line))
    };
    Ok(JavaInfo {
        found: true,
        path: Some(path.to_string_lossy().into_owned()),
        version,
        major,
        source: source.to_string(),
    })
}

/// `openjdk version "21.0.5" …` → 21, `java version "1.8.0_392" …` → 8.
pub fn extract_major(line: &str) -> Option<u32> {
    let start = line.find('"')?;
    let rest = line.get(start + 1..)?;
    let end = rest.find('"').unwrap_or(rest.len());
    let version = &rest[..end];
    let head: String = version
        .split('.')
        .next()?
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let value: u32 = head.parse().ok()?;
    Some(if value >= 2 { value } else { 8 })
}
