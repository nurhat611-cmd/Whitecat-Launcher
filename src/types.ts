export type Tab = "home" | "content" | "settings";

export type LoaderId = "vanilla" | "fabric" | "forge";

export type ProjectType = "mod" | "resourcepack" | "shader" | "modpack";

export interface JavaInfo {
  found: boolean;
  path: string | null;
  version: string | null;
  major: number | null;
  source: string;
}

export interface LauncherSettings {
  nickname: string;
  versionId: string;
  loader: LoaderId;
  ramGb: number;
  gameDir: string | null;
}

export interface ProgressEvent {
  stage: string;
  message: string;
  percent: number;
}

/* ---------- версии Minecraft (piston-meta) ---------- */

export interface VersionSummary {
  id: string;
  type: string;
  releaseTime: string;
  url: string;
}

export interface VersionList {
  versions: VersionSummary[];
  latestRelease: string | null;
  latestSnapshot: string | null;
  fetchedAt: number;
  fromCache: boolean;
  stale: boolean;
}

/* ---------- очередь загрузок ---------- */

export type DownloadStatus = "queued" | "running" | "done" | "error" | "cancelled";

export interface TaskProgress {
  percent: number;
  message: string;
  currentFile: string;
  doneFiles: number;
  totalFiles: number;
  downloadedBytes: number;
  totalBytes: number;
  speedBps: number;
}

export interface DownloadTask {
  id: string;
  kind: "version" | ProjectType;
  title: string;
  subtitle: string;
  iconUrl: string | null;
  projectId: string | null;
  status: DownloadStatus;
  progress: TaskProgress;
  error: string | null;
  target: string | null;
  createdAt: number;
}

/* ---------- контент Modrinth ---------- */

export interface ContentHit {
  id: string;
  slug: string;
  title: string;
  description: string;
  author: string;
  projectType: string;
  iconUrl: string | null;
  downloads: number;
  follows: number;
  categories: string[];
  versions: string[];
  dateModified: string;
}

export interface SearchResult {
  hits: ContentHit[];
  totalHits: number;
  offset: number;
  limit: number;
}

export interface ContentFile {
  url: string;
  filename: string;
  size: number;
  sha1: string | null;
  primary: boolean;
}

export interface ContentVersion {
  id: string;
  projectId: string;
  name: string;
  versionNumber: string;
  gameVersions: string[];
  loaders: string[];
  downloads: number;
  datePublished: string;
  versionType: string;
  files: ContentFile[];
}

export interface InstanceInfo {
  name: string;
  slug: string;
  path: string;
  mcVersion: string;
  loader: string;
  loaderVersion: string;
  modCount: number;
  installedAt: number;
  iconUrl: string | null;
}

export interface SearchParams {
  query: string;
  projectType: ProjectType;
  gameVersion?: string | null;
  loader?: string | null;
  sort?: string;
  offset?: number;
  limit?: number;
}

export interface InstallParams {
  projectId: string;
  projectType: ProjectType;
  title: string;
  subtitle?: string;
  iconUrl?: string | null;
  gameVersion?: string | null;
  loader?: string | null;
  versionId?: string | null;
}

export const DEFAULT_SETTINGS: LauncherSettings = {
  nickname: "",
  versionId: "1.21.4",
  loader: "vanilla",
  ramGb: 4,
  gameDir: null,
};
