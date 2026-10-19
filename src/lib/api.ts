import { invoke } from "@tauri-apps/api/core";
import type {
  ContentHit,
  ContentVersion,
  DownloadTask,
  GameLogs,
  InstanceInfo,
  InstallParams,
  JavaInfo,
  LauncherSettings,
  SearchParams,
  SearchResult,
  SkinInfo,
  UpdateInfo,
  VersionList,
} from "../types";

const MANIFEST_URL = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const MODRINTH_API = "https://api.modrinth.com/v2";

export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/* ---------- часть 1 ---------- */

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

/* ---------- версии Minecraft ---------- */

export async function getVersions(refresh = false): Promise<VersionList> {
  if (!isTauri()) return browserVersions();
  return invoke<VersionList>("get_versions", { refresh });
}

/* ---------- Modrinth ---------- */

export async function searchContent(params: SearchParams): Promise<SearchResult> {
  if (!isTauri()) return browserSearch(params);
  return invoke<SearchResult>("search_content", {
    query: params.query,
    projectType: params.projectType,
    gameVersion: params.gameVersion ?? null,
    loader: params.loader ?? null,
    sort: params.sort ?? "relevance",
    offset: params.offset ?? 0,
    limit: params.limit ?? 20,
  });
}

export function getContentVersions(
  projectId: string,
  gameVersion?: string | null,
  loader?: string | null,
): Promise<ContentVersion[]> {
  return invoke<ContentVersion[]>("get_content_versions", {
    projectId,
    gameVersion: gameVersion ?? null,
    loader: loader ?? null,
  });
}

export function listInstances(): Promise<InstanceInfo[]> {
  return invoke<InstanceInfo[]>("list_instances");
}

/* ---------- очередь загрузок ---------- */

export function enqueueVersion(versionId: string, loader: string): Promise<string> {
  return invoke<string>("enqueue_version", { versionId, loader });
}

export function enqueueContent(params: InstallParams): Promise<string> {
  return invoke<string>("enqueue_content", {
    projectId: params.projectId,
    projectType: params.projectType,
    title: params.title,
    subtitle: params.subtitle ?? null,
    iconUrl: params.iconUrl ?? null,
    gameVersion: params.gameVersion ?? null,
    loader: params.loader ?? null,
    versionId: params.versionId ?? null,
  });
}

export function getDownloadQueue(): Promise<DownloadTask[]> {
  return invoke<DownloadTask[]>("get_download_queue");
}

export function cancelDownload(id: string): Promise<void> {
  return invoke<void>("cancel_download", { id });
}

export function removeDownload(id: string): Promise<void> {
  return invoke<void>("remove_download", { id });
}

export function clearFinishedDownloads(): Promise<void> {
  return invoke<void>("clear_finished_downloads");
}

/* ---------- 3/4 скины ---------- */

export function getSkinInfo(): Promise<SkinInfo> {
  if (!isTauri()) {
    return Promise.resolve({
      exists: false,
      path: null,
      model: "steve",
      hasCustom: false,
      previewBase64: null,
    });
  }
  return invoke<SkinInfo>("get_skin_info");
}

export function saveSkinUrl(url: string, model: string): Promise<SkinInfo> {
  return invoke<SkinInfo>("save_skin_url", { url, model });
}

export function saveSkinFile(base64Data: string, filename: string | null, model: string): Promise<SkinInfo> {
  return invoke<SkinInfo>("save_skin_file", { base64Data, filename, model });
}

export function deleteSkin(): Promise<SkinInfo> {
  return invoke<SkinInfo>("delete_skin_cmd");
}

export function openSkinsFolder(): Promise<string> {
  return invoke<string>("open_skins_folder_cmd");
}

/* ---------- 3/4 логи / ошибки ---------- */

export function getGameLogs(): Promise<GameLogs> {
  if (!isTauri()) {
    return Promise.resolve({
      launcherLog: "Логи доступны только в приложении Tauri",
      launcherLogPath: null,
      latestLog: null,
      latestLogPath: null,
      crashReports: [],
      gameDir: "(браузерный режим)",
    });
  }
  return invoke<GameLogs>("get_game_logs");
}

export function openLogsFolder(kind?: string): Promise<string> {
  return invoke<string>("open_logs_folder", { kind: kind ?? null });
}

export function openGameFolder(): Promise<string> {
  return invoke<string>("open_game_folder");
}

export function checkForUpdates(force?: boolean): Promise<UpdateInfo> {
  if (!isTauri()) {
    return Promise.resolve({
      currentVersion: "0.3.0",
      latestVersion: "0.3.0",
      hasUpdate: false,
      checkedAt: Math.floor(Date.now() / 1000),
      changelog: "Браузерный режим — проверка обновлений недоступна",
      downloadUrl: null,
    });
  }
  return invoke<UpdateInfo>("check_for_updates", { force: force ?? null });
}

/* ---------- браузерный режим (предпросмотр без Tauri) ---------- */

async function fetchJson(url: string): Promise<any> {
  const response = await fetch(url, { headers: { Accept: "application/json" } });
  if (!response.ok) throw new Error(`Сервис вернул статус ${response.status}`);
  return response.json();
}

async function browserVersions(): Promise<VersionList> {
  try {
    const manifest = await fetchJson(MANIFEST_URL);
    const versions = (manifest.versions ?? []).map((item: any) => ({
      id: String(item.id),
      type: String(item.type ?? "release"),
      releaseTime: String(item.releaseTime ?? ""),
      url: String(item.url ?? ""),
    }));
    versions.sort((a: any, b: any) => b.releaseTime.localeCompare(a.releaseTime));
    return {
      versions,
      latestRelease: manifest.latest?.release ?? null,
      latestSnapshot: manifest.latest?.snapshot ?? null,
      fetchedAt: Math.floor(Date.now() / 1000),
      fromCache: false,
      stale: false,
    };
  } catch (error) {
    throw new Error(
      `Не удалось загрузить список версий в браузере (${String(error)}). Запустите npm run tauri dev.`,
    );
  }
}

async function browserSearch(params: SearchParams): Promise<SearchResult> {
  const facets: string[][] = [[`project_type:${params.projectType}`]];
  if (params.gameVersion) facets.push([`versions:${params.gameVersion}`]);
  if (params.loader) facets.push([`categories:${params.loader}`]);
  const offset = params.offset ?? 0;
  const limit = params.limit ?? 20;
  const query = new URLSearchParams({
    query: params.query,
    facets: JSON.stringify(facets),
    index: params.sort ?? "relevance",
    offset: String(offset),
    limit: String(limit),
  });
  let payload: any;
  try {
    payload = await fetchJson(`${MODRINTH_API}/search?${query.toString()}`);
  } catch (error) {
    throw new Error(
      `Modrinth недоступен из браузера (${String(error)}). Запустите npm run tauri dev.`,
    );
  }
  const hits: ContentHit[] = (payload.hits ?? []).map((hit: any) => ({
    id: String(hit.project_id ?? hit.slug ?? ""),
    slug: String(hit.slug ?? ""),
    title: String(hit.title ?? "Без названия"),
    description: String(hit.description ?? ""),
    author: String(hit.author ?? "неизвестный автор"),
    projectType: String(hit.project_type ?? params.projectType),
    iconUrl: hit.icon_url ? String(hit.icon_url) : null,
    downloads: Number(hit.downloads ?? 0),
    follows: Number(hit.follows ?? 0),
    categories: [...(hit.display_categories ?? []), ...(hit.categories ?? [])].map(String),
    versions: (hit.versions ?? []).map(String),
    dateModified: String(hit.date_modified ?? ""),
  }));
  return {
    hits,
    totalHits: Number(payload.total_hits ?? hits.length),
    offset,
    limit,
  };
}
