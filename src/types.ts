export type Tab = "home" | "content" | "settings";

export type LoaderId = "vanilla" | "fabric" | "forge";

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

// Список версий-заглушек для выпадающего списка (полный список
// подтянется из манифеста Mojang в следующей части).
export const VERSIONS = [
  "1.21.4",
  "1.21.1",
  "1.20.6",
  "1.20.1",
  "1.19.4",
  "1.18.2",
  "1.16.5",
  "1.12.2",
  "1.8.9",
];

export const DEFAULT_SETTINGS: LauncherSettings = {
  nickname: "",
  versionId: "1.21.4",
  loader: "vanilla",
  ramGb: 4,
  gameDir: null,
};
