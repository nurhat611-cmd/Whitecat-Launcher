import { motion } from "framer-motion";
import {
  AlertCircle,
  ArrowDownToLine,
  LoaderCircle,
  Sparkles,
} from "lucide-react";

interface UpdateDialogProps {
  version: string;
  currentVersion: string;
  releaseNotes?: string;
  releaseDate?: string;
  downloading: boolean;
  progress: number | null;
  error: string | null;
  onUpdate: () => void;
  onLater: () => void;
}

export function UpdateDialog({
  version,
  currentVersion,
  releaseNotes,
  releaseDate,
  downloading,
  progress,
  error,
  onUpdate,
  onLater,
}: UpdateDialogProps) {
  const parsedDate = releaseDate ? new Date(releaseDate) : null;
  const formattedDate =
    parsedDate && Number.isFinite(parsedDate.getTime())
      ? new Intl.DateTimeFormat("ru-RU", { dateStyle: "long" }).format(
          parsedDate,
        )
      : null;

  return (
    <motion.div
      className="update-overlay"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.18 }}
    >
      <motion.section
        className="update-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="update-title"
        aria-describedby="update-description"
        initial={{ opacity: 0, y: 18, scale: 0.96 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 12, scale: 0.98 }}
        transition={{ type: "spring", stiffness: 360, damping: 28 }}
      >
        <div className="update-glow" aria-hidden="true" />
        <div className="update-icon" aria-hidden="true">
          <Sparkles size={23} strokeWidth={2.1} />
          <span className="update-icon-dot" />
        </div>

        <div className="update-eyebrow">НОВАЯ ВЕРСИЯ ЛАУНЧЕРА</div>
        <h2 id="update-title">Доступно обновление</h2>
        <p id="update-description" className="update-lead">
          MyLauncher стал лучше — установите свежую версию, чтобы продолжить
          играть.
        </p>

        <div className="update-version-row">
          <span className="update-version">v{version}</span>
          <span className="update-version-caption">
            Текущая: v{currentVersion}
          </span>
          {formattedDate && (
            <span className="update-date">{formattedDate}</span>
          )}
        </div>

        {releaseNotes?.trim() && (
          <p className="update-notes">{releaseNotes.trim()}</p>
        )}

        {downloading && (
          <div className="update-progress" aria-live="polite">
            <div className="update-progress-label">
              <span>
                {progress === 100
                  ? "Подготовка установки…"
                  : "Загрузка обновления…"}
              </span>
              <span>{progress === null ? "" : `${progress}%`}</span>
            </div>
            <div
              className={`update-progress-track${progress === null ? " indeterminate" : ""}`}
              role="progressbar"
              aria-label="Загрузка обновления"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={progress ?? undefined}
            >
              <motion.div
                className="update-progress-fill"
                animate={{ width: `${progress ?? 28}%` }}
                transition={{ duration: 0.22, ease: "easeOut" }}
              />
            </div>
          </div>
        )}

        {error && (
          <div className="update-error" role="alert">
            <AlertCircle size={17} />
            <span>{error}</span>
          </div>
        )}

        <div className="update-actions">
          <button
            className="update-later"
            type="button"
            onClick={onLater}
            disabled={downloading}
          >
            Позже
          </button>
          <button
            className="update-install"
            type="button"
            onClick={onUpdate}
            disabled={downloading}
          >
            {downloading ? (
              <LoaderCircle className="update-spinner" size={17} />
            ) : (
              <ArrowDownToLine size={17} />
            )}
            <span>
              {downloading ? "Обновление…" : error ? "Повторить" : "Обновить"}
            </span>
          </button>
        </div>
        <p className="update-footnote">
          Обновление безопасно проверено перед установкой
        </p>
      </motion.section>
    </motion.div>
  );
}
