import {
  AlertCircle,
  CheckCircle2,
  DownloadCloud,
  Loader2,
  Trash2,
  X,
} from "lucide-react";
import type { DownloadTask, TaskProgress } from "../types";

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "0 Б";
  const units = ["Б", "КБ", "МБ", "ГБ"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  return `${value >= 100 || index === 0 ? Math.round(value) : value.toFixed(1)} ${units[index]}`;
}

export function formatSpeed(bytesPerSecond: number): string {
  if (!bytesPerSecond || bytesPerSecond <= 0) return "";
  return `${formatBytes(bytesPerSecond)}/с`;
}

export function formatCount(count: number): string {
  if (!Number.isFinite(count) || count < 0) return "0";
  if (count >= 1_000_000_000) return `${(count / 1_000_000_000).toFixed(1)} млрд`;
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)} млн`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)} тыс.`;
  return String(Math.round(count));
}

export function ProgressBar({ progress }: { progress: TaskProgress }) {
  const width = Math.max(0, Math.min(100, progress.percent));
  return (
    <div className="progress">
      <i style={{ width: `${width}%` }} />
    </div>
  );
}

interface RowProps {
  task: DownloadTask;
  onCancel?: (id: string) => void;
  onRemove?: (id: string) => void;
  compact?: boolean;
}

export function DownloadRow({ task, onCancel, onRemove, compact }: RowProps) {
  const running = task.status === "running" || task.status === "queued";
  const progress = task.progress;
  return (
    <div className={`dl-row status-${task.status}`}>
      <div className="dl-icon">
        {task.status === "done" ? (
          <CheckCircle2 size={16} />
        ) : task.status === "error" || task.status === "cancelled" ? (
          <AlertCircle size={16} />
        ) : (
          <Loader2 size={16} className="spin" />
        )}
      </div>
      <div className="dl-body">
        <div className="dl-head">
          <b className="dl-title">{task.title}</b>
          <span className="dl-pct">
            {task.status === "done" ? "готово" : task.status === "cancelled" ? "отменено" : `${progress.percent}%`}
          </span>
        </div>
        {!compact && (
          <div className="dl-sub">
            {task.error ??
              (progress.currentFile
                ? progress.currentFile
                : progress.message || task.subtitle)}
          </div>
        )}
        {compact && <div className="dl-sub">{progress.message || task.subtitle}</div>}
        {running && <ProgressBar progress={progress} />}
        {running && (
          <div className="dl-meta">
            <span>{progress.message}</span>
            {progress.totalFiles > 0 && (
              <span>
                файлы {progress.doneFiles}/{progress.totalFiles}
              </span>
            )}
            {progress.totalBytes > 0 && (
              <span>
                {formatBytes(progress.downloadedBytes)} / {formatBytes(progress.totalBytes)}
              </span>
            )}
            {progress.speedBps > 0 && <span>{formatSpeed(progress.speedBps)}</span>}
          </div>
        )}
      </div>
      <div className="dl-actions">
        {running && onCancel && (
          <button
            className="icon-btn"
            type="button"
            title="Отменить"
            onClick={() => onCancel(task.id)}
          >
            <X size={15} />
          </button>
        )}
        {!running && onRemove && (
          <button
            className="icon-btn"
            type="button"
            title="Убрать из списка"
            onClick={() => onRemove(task.id)}
          >
            <Trash2 size={15} />
          </button>
        )}
      </div>
    </div>
  );
}

interface PanelProps {
  tasks: DownloadTask[];
  onCancel: (id: string) => void;
  onRemove: (id: string) => void;
  onClear: () => void;
}

export function DownloadQueuePanel({ tasks, onCancel, onRemove, onClear }: PanelProps) {
  if (tasks.length === 0) return null;
  const active = tasks.filter((t) => t.status === "running" || t.status === "queued").length;
  const finished = tasks.length - active;
  return (
    <section className="card queue-card">
      <div className="card-head">
        <span className="card-ic">
          <DownloadCloud size={18} />
        </span>
        <div>
          <h2>Загрузки</h2>
          <p>
            {active > 0 ? `${active} в работе · ` : ""}
            {tasks.length} в очереди{finished > 0 ? `, ${finished} завершено` : ""}
          </p>
        </div>
        {finished > 0 && (
          <button className="link queue-clear" type="button" onClick={onClear}>
            <Trash2 size={14} /> Очистить
          </button>
        )}
      </div>
      <div className="dl-list">
        {tasks.map((task) => (
          <DownloadRow key={task.id} task={task} onCancel={onCancel} onRemove={onRemove} />
        ))}
      </div>
    </section>
  );
}
