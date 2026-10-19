import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { listen } from "@tauri-apps/api/event";
import { check } from "@tauri-apps/plugin-updater";
import type { DownloadEvent, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type {
  DownloadTask,
  JavaInfo,
  LauncherSettings,
  ProgressEvent,
  Tab,
  VersionList,
} from "./types";
import { DEFAULT_SETTINGS } from "./types";
import {
  downloadJdk,
  enqueueVersion,
  getDownloadQueue,
  getJava,
  getVersions,
  isTauri,
  launchGame,
  loadSettings,
  saveSettings,
} from "./lib/api";
import { Splash } from "./components/Splash";
import { Sidebar } from "./components/Sidebar";
import { HomeTab } from "./components/HomeTab";
import { ContentTab } from "./components/ContentTab";
import { SettingsTab } from "./components/SettingsTab";
import { UpdateDialog } from "./components/UpdateDialog";

type Tone = "ok" | "warn" | "err" | "busy" | "idle";

export default function App() {
  const [splash, setSplash] = useState(true);
  const [tab, setTab] = useState<Tab>("home");
  const [settings, setSettings] = useState<LauncherSettings>(DEFAULT_SETTINGS);
  const [java, setJava] = useState<JavaInfo | null>(null);
  const [jdk, setJdk] = useState<ProgressEvent | null>(null);
  const [launch, setLaunch] = useState<ProgressEvent | null>(null);
  const [gamePid, setGamePid] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [versions, setVersions] = useState<VersionList | null>(null);
  const [versionsLoading, setVersionsLoading] = useState(false);
  const [queue, setQueue] = useState<DownloadTask[]>([]);
  const [availableUpdate, setAvailableUpdate] = useState<Update | null>(null);
  const [updateDownloading, setUpdateDownloading] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<number | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const launchingRef = useRef(false);

  // Splash-экран живёт ровно 2 секунды, после чего плавно растворяется.
  useEffect(() => {
    const timer = setTimeout(() => setSplash(false), 2000);
    return () => clearTimeout(timer);
  }, []);

  // В production-сборке проверяем статический latest.json при запуске.
  // В браузерном предпросмотре и локальном dev-режиме updater не вызывается.
  useEffect(() => {
    if (!isTauri() || import.meta.env.VITE_UPDATER_ENABLED !== "true") return;
    let disposed = false;
    let pendingUpdate: Update | null = null;

    void check({ timeout: 20_000 })
      .then(async (nextUpdate) => {
        pendingUpdate = nextUpdate;
        if (disposed) {
          await nextUpdate?.close().catch(() => undefined);
          return;
        }
        if (nextUpdate) setAvailableUpdate(nextUpdate);
      })
      .catch((reason: unknown) => {
        // Недоступный GitHub или отсутствие сети не мешают запуску лаунчера.
        console.warn("Не удалось проверить обновление MyLauncher:", reason);
      });

    return () => {
      disposed = true;
      if (pendingUpdate) void pendingUpdate.close().catch(() => undefined);
    };
  }, []);

  useEffect(() => {
    if (!isTauri()) {
      setLoading(false);
      return;
    }
    let disposed = false;
    const unlisteners: (() => void)[] = [];
    (async () => {
      const offJdk = await listen<ProgressEvent>("jdk_progress", (e) => {
        if (!disposed) setJdk(e.payload);
      });
      const offLaunch = await listen<ProgressEvent>("launch_progress", (e) => {
        if (!disposed) setLaunch(e.payload);
      });
      // Фоновое обновление манифеста Mojang приходит отдельным событием.
      const offVersions = await listen<VersionList>("versions_updated", (e) => {
        if (!disposed) setVersions(e.payload);
      });
      // Очередь загрузок присылает полный снимок после каждого изменения.
      const offQueue = await listen<DownloadTask[]>("download_queue", (e) => {
        if (!disposed) setQueue(e.payload);
      });
      if (disposed) {
        offJdk();
        offLaunch();
        offVersions();
        offQueue();
        return;
      }
      unlisteners.push(offJdk, offLaunch, offVersions, offQueue);

      try {
        const [javaInfo, saved, versionList, queueList] = await Promise.all([
          getJava(),
          loadSettings(),
          getVersions(false).catch(() => null),
          getDownloadQueue().catch(() => [] as DownloadTask[]),
        ]);
        if (!disposed) {
          setJava(javaInfo);
          setSettings((prev) => ({ ...prev, ...saved }));
          if (versionList) setVersions(versionList);
          setQueue(queueList);
        }
      } catch (e) {
        if (!disposed) setError(String(e));
      } finally {
        if (!disposed) setLoading(false);
      }
    })();
    return () => {
      disposed = true;
      unlisteners.forEach((off) => off());
    };
  }, []);

  // Автосохранение настроек (debounce).
  useEffect(() => {
    if (!isTauri() || loading) return;
    const timer = setTimeout(() => {
      saveSettings(settings).catch(() => undefined);
    }, 400);
    return () => clearTimeout(timer);
  }, [settings, loading]);

  const patch = useCallback((part: Partial<LauncherSettings>) => {
    setSettings((prev) => ({ ...prev, ...part }));
    setError(null);
  }, []);

  const refreshJava = useCallback(async () => {
    if (!isTauri()) return;
    try {
      setJava(await getJava());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const refreshVersions = useCallback(async () => {
    if (!isTauri()) return;
    setVersionsLoading(true);
    try {
      setVersions(await getVersions(true));
    } catch (e) {
      setError(String(e));
    } finally {
      setVersionsLoading(false);
    }
  }, []);

  const onDownloadJdk = useCallback(async () => {
    if (!isTauri()) return;
    setError(null);
    setJdk({ stage: "download", message: "Подготовка…", percent: 0 });
    try {
      const info = await downloadJdk();
      setJava(info);
      setJdk(null);
    } catch (e) {
      setJdk(null);
      setError(String(e));
    }
  }, []);

  const onDownloadVersion = useCallback(
    async (versionId: string, loader: string) => {
      if (!isTauri()) {
        setError(
          "Скачивание работает в приложении: запустите npm run tauri dev",
        );
        return;
      }
      try {
        await enqueueVersion(versionId, loader);
        setError(null);
      } catch (e) {
        setError(String(e));
      }
    },
    [],
  );

  const busy = launch !== null && launch.stage !== "done";
  const activeTask =
    queue.find((task) => task.status === "running") ??
    queue.find((task) => task.status === "queued");
  const downloadingVersion = queue.some(
    (task) =>
      task.kind === "version" &&
      (task.status === "running" || task.status === "queued"),
  );

  const play = useCallback(async () => {
    if (launchingRef.current) return;
    if (!isTauri()) {
      setError(
        "Запустите приложение через npm run tauri dev, чтобы подключить Rust-бэкенд",
      );
      return;
    }
    setError(null);
    setGamePid(null);
    setLaunch({ stage: "prepare", message: "Проверка настроек…", percent: 0 });
    launchingRef.current = true;
    try {
      const pid = await launchGame(settings);
      setGamePid(pid);
      setLaunch({ stage: "done", message: "Игра запущена", percent: 100 });
    } catch (e) {
      setLaunch(null);
      setError(String(e));
    } finally {
      launchingRef.current = false;
    }
  }, [settings]);

  const installUpdate = useCallback(async () => {
    if (!availableUpdate || updateDownloading) return;
    setUpdateDownloading(true);
    setUpdateProgress(null);
    setUpdateError(null);

    let receivedBytes = 0;
    let expectedBytes: number | null = null;
    try {
      await availableUpdate.downloadAndInstall((event: DownloadEvent) => {
        if (event.event === "Started") {
          receivedBytes = 0;
          expectedBytes = event.data.contentLength ?? null;
          setUpdateProgress(expectedBytes ? 0 : null);
        } else if (event.event === "Progress") {
          receivedBytes += event.data.chunkLength;
          setUpdateProgress(
            expectedBytes
              ? Math.min(99, Math.floor((receivedBytes / expectedBytes) * 100))
              : null,
          );
        } else if (event.event === "Finished") {
          setUpdateProgress(100);
        }
      });
      await relaunch();
    } catch (reason) {
      const details = reason instanceof Error ? reason.message : String(reason);
      setUpdateError(`Не удалось установить обновление. ${details}`);
      setUpdateProgress(null);
    } finally {
      setUpdateDownloading(false);
    }
  }, [availableUpdate, updateDownloading]);

  const dismissUpdate = useCallback(() => {
    const pending = availableUpdate;
    setAvailableUpdate(null);
    setUpdateError(null);
    setUpdateProgress(null);
    if (pending) void pending.close().catch(() => undefined);
  }, [availableUpdate]);

  let status: { text: string; tone: Tone; percent?: number };
  if (error) status = { text: error, tone: "err" };
  else if (busy && launch)
    status = { text: launch.message, tone: "busy", percent: launch.percent };
  else if (activeTask)
    status = {
      text: `${activeTask.title} · ${activeTask.progress.currentFile || activeTask.progress.message}`,
      tone: "busy",
      percent: activeTask.progress.percent,
    };
  else if (launch?.stage === "done")
    status = {
      text: `Игра запущена${gamePid ? ` · PID ${gamePid}` : ""}`,
      tone: "ok",
    };
  else if (jdk)
    status = { text: `${jdk.message} · ${jdk.percent}%`, tone: "busy" };
  else if (loading) status = { text: "Инициализация…", tone: "idle" };
  else if (!isTauri())
    status = {
      text: "Запустите приложение через npm run tauri dev",
      tone: "warn",
    };
  else if (!java?.found)
    status = {
      text: "Java не найдена — скачайте JDK во вкладке «Настройки»",
      tone: "warn",
    };
  else status = { text: "Готов к запуску", tone: "ok" };

  return (
    <>
      <AnimatePresence>{splash && <Splash key="splash" />}</AnimatePresence>
      <div className="app">
        <Sidebar tab={tab} onTab={setTab} />
        <main className="main">
          <div className="main-scroll">
            <AnimatePresence mode="wait">
              <motion.div
                key={tab}
                className="tab-panel"
                initial={{ opacity: 0, x: 22 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -22 }}
                transition={{ duration: 0.22, ease: "easeOut" }}
              >
                {tab === "home" && (
                  <HomeTab
                    settings={settings}
                    onPatch={patch}
                    java={java}
                    onRefreshJava={refreshJava}
                    versions={versions}
                    versionsLoading={versionsLoading}
                    onRefreshVersions={refreshVersions}
                    queue={queue}
                    onDownloadVersion={onDownloadVersion}
                  />
                )}
                {tab === "content" && (
                  <ContentTab
                    versions={versions}
                    settings={settings}
                    queue={queue}
                  />
                )}
                {tab === "settings" && (
                  <SettingsTab
                    settings={settings}
                    onPatch={patch}
                    java={java}
                    jdk={jdk}
                    onRefreshJava={refreshJava}
                    onDownloadJdk={onDownloadJdk}
                  />
                )}
              </motion.div>
            </AnimatePresence>
          </div>
          <div className="play-zone">
            <button
              className={`play-btn${busy ? " playing" : ""}`}
              onClick={play}
              disabled={
                busy ||
                downloadingVersion ||
                (jdk !== null && jdk.stage !== "done")
              }
            >
              {busy
                ? (launch?.message ?? "ЗАПУСК…").toUpperCase()
                : downloadingVersion
                  ? "СКАЧИВАНИЕ…"
                  : "ИГРАТЬ"}
            </button>
            <div className={`status-bar tone-${status.tone}`}>
              <span className="status-dot" aria-hidden />
              <span className="status-text">{status.text}</span>
              {status.percent !== undefined && (
                <span className="status-pct">{status.percent}%</span>
              )}
            </div>
          </div>
        </main>
      </div>
      <AnimatePresence>
        {availableUpdate && (
          <UpdateDialog
            key="available-update"
            version={availableUpdate.version}
            currentVersion={availableUpdate.currentVersion}
            releaseNotes={availableUpdate.body}
            releaseDate={availableUpdate.date}
            downloading={updateDownloading}
            progress={updateProgress}
            error={updateError}
            onUpdate={installUpdate}
            onLater={dismissUpdate}
          />
        )}
      </AnimatePresence>
    </>
  );
}
