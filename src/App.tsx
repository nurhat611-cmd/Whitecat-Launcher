import { useCallback, useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { listen } from "@tauri-apps/api/event";
import type { JavaInfo, LauncherSettings, ProgressEvent, Tab } from "./types";
import { DEFAULT_SETTINGS } from "./types";
import { downloadJdk, getJava, isTauri, launchGame, loadSettings, saveSettings } from "./lib/api";
import { Splash } from "./components/Splash";
import { Sidebar } from "./components/Sidebar";
import { HomeTab } from "./components/HomeTab";
import { ContentTab } from "./components/ContentTab";
import { SettingsTab } from "./components/SettingsTab";

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
  const launchingRef = useRef(false);

  // Splash-экран живёт ровно 2 секунды, после чего плавно растворяется.
  useEffect(() => {
    const timer = setTimeout(() => setSplash(false), 2000);
    return () => clearTimeout(timer);
  }, []);

  useEffect(() => {
    if (!isTauri()) {
      setLoading(false);
      return;
    }
    let disposed = false;
    let unlistenJdk: (() => void) | undefined;
    let unlistenLaunch: (() => void) | undefined;
    (async () => {
      const offJdk = await listen<ProgressEvent>("jdk_progress", (e) => {
        if (!disposed) setJdk(e.payload);
      });
      const offLaunch = await listen<ProgressEvent>("launch_progress", (e) => {
        if (!disposed) setLaunch(e.payload);
      });
      if (disposed) {
        offJdk();
        offLaunch();
        return;
      }
      unlistenJdk = offJdk;
      unlistenLaunch = offLaunch;
      try {
        const [javaInfo, saved] = await Promise.all([getJava(), loadSettings()]);
        if (!disposed) {
          setJava(javaInfo);
          setSettings((prev) => ({ ...prev, ...saved }));
        }
      } catch (e) {
        if (!disposed) setError(String(e));
      } finally {
        if (!disposed) setLoading(false);
      }
    })();
    return () => {
      disposed = true;
      unlistenJdk?.();
      unlistenLaunch?.();
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

  const busy = launch !== null && launch.stage !== "done";

  const play = useCallback(async () => {
    if (launchingRef.current) return;
    if (!isTauri()) {
      setError("Запустите приложение через npm run tauri dev, чтобы подключить Rust-бэкенд");
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

  let status: { text: string; tone: Tone };
  if (error) status = { text: error, tone: "err" };
  else if (busy && launch) status = { text: launch.message, tone: "busy" };
  else if (launch?.stage === "done")
    status = { text: `Игра запущена${gamePid ? ` · PID ${gamePid}` : ""}`, tone: "ok" };
  else if (jdk) status = { text: `${jdk.message} · ${jdk.percent}%`, tone: "busy" };
  else if (loading) status = { text: "Инициализация…", tone: "idle" };
  else if (!isTauri()) status = { text: "Запустите приложение через npm run tauri dev", tone: "warn" };
  else if (!java?.found)
    status = { text: "Java не найдена — скачайте JDK во вкладке «Настройки»", tone: "warn" };
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
                  />
                )}
                {tab === "content" && <ContentTab />}
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
              disabled={busy || (jdk !== null && jdk.stage !== "done")}
            >
              {busy ? (launch?.message ?? "ЗАПУСК…").toUpperCase() : "ИГРАТЬ"}
            </button>
            <div className={`status-bar tone-${status.tone}`}>
              <span className="status-dot" aria-hidden />
              <span className="status-text">{status.text}</span>
              {busy && <span className="status-pct">{launch?.percent ?? 0}%</span>}
            </div>
          </div>
        </main>
      </div>
    </>
  );
}
