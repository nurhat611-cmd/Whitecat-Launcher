import { invoke } from "@tauri-apps/api/core";
import type { JavaInfo, LauncherSettings } from "../types";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function getJava(): Promise<JavaInfo> {
  return invoke<JavaInfo>("get_java");
}

export function downloadJdk(): Promise<JavaInfo> {
  return invoke<JavaInfo>("download_jdk");
}

export function loadSettings(): Promise<LauncherSettings> {
  return invoke<LauncherSettings>("load_settings");
}

export function saveSettings(settings: LauncherSettings): Promise<void> {
  return invoke<void>("save_settings", { settings });
}

export function launchGame(settings: LauncherSettings): Promise<number> {
  return invoke<number>("launch_game", { settings });
}
