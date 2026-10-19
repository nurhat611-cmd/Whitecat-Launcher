import { useEffect, useState } from "react";
import { AlertTriangle, Bug, Copy, FolderOpen, FileText, X } from "lucide-react";
import type { GameLogs } from "../types";
import { getGameLogs, openGameFolder, openLogsFolder } from "../lib/api";

interface Props {
  open: boolean;
  title: string;
  message: string;
  onClose: () => void;
}

function useClipboard() {
  const [copied, setCopied] = useState<string | null>(null);
  const copy = async (text: string, key: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(key);
      setTimeout(() => setCopied(null), 1500);
    } catch {
      // fallback
      const ta = document.createElement("textarea");
      ta.value = text;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      setCopied(key);
      setTimeout(() => setCopied(null), 1500);
    }
  };
  return { copied, copy };
}

export function ErrorModal({ open, title, message, onClose }: Props) {
  const [logs, setLogs] = useState<GameLogs | null>(null);
  const [loading, setLoading] = useState(false);
  const [tab, setTab] = useState<"message" | "latest" | "crash" | "launcher">("message");
  const { copied, copy } = useClipboard();

  useEffect(() => {
    if (!open) return;
    setLoading(true);
    getGameLogs()
      .then((l) => setLogs(l))
      .catch(() => setLogs(null))
      .finally(() => setLoading(false));
  }, [open, message]);

  // lock body scroll when open
  useEffect(() => {
    if (open) {
      const prev = document.body.style.overflow;
      document.body.style.overflow = "hidden";
      return () => {
        document.body.style.overflow = prev;
      };
    }
  }, [open]);

  if (!open) return null;

  const activeCrash = logs?.crashReports?.[0] ?? null;
  const latestSnippet = logs?.latestLog ? logs.latestLog.slice(-12000) : null;
  const launcherSnippet = logs?.launcherLog ? logs.launcherLog.slice(-12000) : null;

  // try to extract nice error excerpt from message + latestLog
  const parseHint = (): string | null => {
    const combined = `${message}\n${latestSnippet ?? ""}\n${activeCrash?.content ?? ""}`;
    const lines = combined.split(/\r?\n/);
    const hit =
      lines.find((l) => l.includes("Exception")) ||
      lines.find((l) => l.includes("Caused by")) ||
      lines.find((l) => l.toLowerCase().includes("error")) ||
      null;
    return hit ? hit.trim().slice(0, 300) : null;
  };

  const hint = parseHint();

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div className="modal-head">
          <span className="modal-ic err">
            <AlertTriangle size={20} />
          </span>
          <div className="modal-titles">
            <h3>{title || "Произошла ошибка"}</h3>
            <p>{hint || "Игра завершилась с ошибкой — подробности ниже"}</p>
          </div>
          <button className="icon-btn" onClick={onClose} aria-label="Закрыть">
            <X size={16} />
          </button>
        </div>

        <div className="modal-tabs">
          <button className={`type-tab ${tab === "message" ? "active" : ""}`} onClick={() => setTab("message")} type="button">
            <Bug size={14} /> Ошибка
          </button>
          <button className={`type-tab ${tab === "latest" ? "active" : ""}`} onClick={() => setTab("latest")} type="button">
            <FileText size={14} /> latest.log
          </button>
          <button className={`type-tab ${tab === "crash" ? "active" : ""}`} onClick={() => setTab("crash")} type="button">
            <Bug size={14} /> crash-reports ({logs?.crashReports?.length ?? 0})
          </button>
          <button className={`type-tab ${tab === "launcher" ? "active" : ""}`} onClick={() => setTab("launcher")} type="button">
            <FileText size={14} /> launcher.log
          </button>
        </div>

        <div className="modal-body">
          {tab === "message" && (
            <div className="log-view">
              <pre className="log-pre err-pre">{message}</pre>
              {logs?.gameDir && <div className="muted small">Папка игры: {logs.gameDir}</div>}
              {loading && <div className="muted">Загружаем логи…</div>}
              {activeCrash && (
                <div className="notice notice-err" style={{ marginTop: 10 }}>
                  <Bug size={14} /> Найден crash-report: <b>{activeCrash.name}</b> — откройте вкладку crash-reports
                </div>
              )}
            </div>
          )}

          {tab === "latest" && (
            <div className="log-view">
              {latestSnippet ? <pre className="log-pre">{latestSnippet}</pre> : <div className="empty small">latest.log не найден (игра ещё не запускалась или путь изменён)</div>}
              <div className="log-actions">
                <button className="btn" type="button" onClick={() => latestSnippet && copy(latestSnippet, "latest")}>
                  <Copy size={14} /> {copied === "latest" ? "Скопировано" : "Копировать лог"}
                </button>
                <button className="btn" type="button" onClick={() => openLogsFolder("logs")}>
                  <FolderOpen size={14} /> Открыть папку с логами
                </button>
              </div>
            </div>
          )}

          {tab === "crash" && (
            <div className="log-view">
              {logs?.crashReports?.length ? (
                logs.crashReports.map((cr) => (
                  <div key={cr.name} className="crash-item">
                    <div className="crash-head">
                      <b>{cr.name}</b>
                      <span className="muted">{new Date(cr.modifiedAt * 1000).toLocaleString("ru-RU")}</span>
                      <button className="icon-btn" title="Копировать" onClick={() => copy(cr.content, cr.name)}>
                        <Copy size={14} />
                      </button>
                    </div>
                    <pre className="log-pre">{cr.content.slice(0, 12000)}</pre>
                  </div>
                ))
              ) : (
                <div className="empty small">Нет crash-reports — проверьте latest.log</div>
              )}
              <div className="log-actions">
                <button className="btn" type="button" onClick={() => activeCrash && copy(activeCrash.content, "crash")}>
                  <Copy size={14} /> {copied === "crash" ? "Скопировано" : "Копировать последний crash"}
                </button>
                <button className="btn" type="button" onClick={() => openLogsFolder("crash")}>
                  <FolderOpen size={14} /> Открыть папку crash-reports
                </button>
              </div>
            </div>
          )}

          {tab === "launcher" && (
            <div className="log-view">
              {launcherSnippet ? <pre className="log-pre">{launcherSnippet}</pre> : <div className="empty small">launcher.log пуст</div>}
              <div className="log-actions">
                <button className="btn" type="button" onClick={() => launcherSnippet && copy(launcherSnippet, "launcher")}>
                  <Copy size={14} /> {copied === "launcher" ? "Скопировано" : "Копировать лог"}
                </button>
                <button className="btn" type="button" onClick={() => openGameFolder()}>
                  <FolderOpen size={14} /> Открыть папку игры
                </button>
              </div>
            </div>
          )}
        </div>

        <div className="modal-foot">
          <span className="muted small">Подсказка: приложите лог при обращении за помощью · кнопка «Копировать лог» скопирует текст в буфер</span>
          <div style={{ display: "flex", gap: 8, marginLeft: "auto" }}>
            <button className="btn" type="button" onClick={onClose}>
              Закрыть
            </button>
            <button
              className="btn btn-accent"
              type="button"
              onClick={() => {
                const all = [message, latestSnippet ?? "", activeCrash?.content ?? ""].filter(Boolean).join("\n\n---\n\n");
                copy(all, "all");
              }}
            >
              <Copy size={14} /> {copied === "all" ? "Скопировано" : "Копировать всё"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
