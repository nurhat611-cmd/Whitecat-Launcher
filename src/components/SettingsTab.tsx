import {
  CheckCircle2,
  Cpu,
  Download,
  FolderOpen,
  Info,
  RefreshCw,
  XCircle,
} from "lucide-react";
import type { JavaInfo, LauncherSettings, ProgressEvent } from "../types";

interface Props {
  settings: LauncherSettings;
  onPatch: (p: Partial<LauncherSettings>) => void;
  java: JavaInfo | null;
  jdk: ProgressEvent | null;
  onRefreshJava: () => void;
  onDownloadJdk: () => void;
}

export function SettingsTab({
  settings,
  onPatch,
  java,
  jdk,
  onRefreshJava,
  onDownloadJdk,
}: Props) {
  const downloading = jdk !== null && jdk.stage !== "done";

  return (
    <div>
      <div className="tab-head">
        <h1>Настройки</h1>
        <p>Java, папка игры и параметры лаунчера</p>
      </div>

      <div className="settings-col">
        <section className="card">
          <div className="card-head">
            <span className="card-ic">
              <Cpu size={18} />
            </span>
            <div>
              <h2>Java-рантайм</h2>
              <p>Автоопределение и установка JDK</p>
            </div>
          </div>

          <div className="java-state">
            {java?.found ? (
              <CheckCircle2 size={20} className="ok-ic" />
            ) : (
              <XCircle size={20} className="err-ic" />
            )}
            <div className="java-texts">
              <b>{java?.found ? `Найдена: ${java.version ?? "Java"}` : "Java не найдена"}</b>
              {java?.found && java.path && <code>{java.path}</code>}
              {java?.found && <span className="muted">Источник: {java.source}</span>}
            </div>
            <div className="java-actions">
              <button className="btn" type="button" onClick={onRefreshJava} disabled={downloading}>
                <RefreshCw size={15} /> Обновить
              </button>
              <button
                className="btn btn-accent"
                type="button"
                onClick={onDownloadJdk}
                disabled={downloading}
              >
                <Download size={15} />
                {downloading ? "Скачивание…" : "Скачать JDK 21"}
              </button>
            </div>
          </div>

          {downloading && jdk && (
            <div className="progress-wrap">
              <div className="progress">
                <i style={{ width: `${jdk.percent}%` }} />
              </div>
              <div className="muted small">
                {jdk.message} · {jdk.percent}%
              </div>
            </div>
          )}

          {!java?.found && !downloading && (
            <p className="hint">
              Рекомендуется JDK 21 (Adoptium). Кнопка выше скачает и установит его в папку
              лаунчера — отдельная настройка системы не нужна.
            </p>
          )}
        </section>

        <section className="card">
          <div className="card-head">
            <span className="card-ic">
              <FolderOpen size={18} />
            </span>
            <div>
              <h2>Папка игры</h2>
              <p>Куда устанавливаются версии и ассеты</p>
            </div>
          </div>
          <label className="field">
            <span className="field-label">Путь</span>
            <input
              className="input"
              type="text"
              value={settings.gameDir ?? ""}
              placeholder="пусто — системная папка данных приложения"
              onChange={(e) => onPatch({ gameDir: e.target.value || null })}
            />
            <span className="field-hint muted">
              Сохраняется автоматически. Текущий: {settings.gameDir ?? "папка по умолчанию"}
            </span>
          </label>
        </section>

        <section className="card">
          <div className="card-head">
            <span className="card-ic">
              <Info size={18} />
            </span>
            <div>
              <h2>О лаунчере</h2>
              <p>MyLauncher · lighty-launcher</p>
            </div>
          </div>
          <div className="sys-row">
            <span className="sys-label">Версия</span>
            <span className="sys-value">0.2.0 (часть 2/4)</span>
          </div>
          <div className="sys-row">
            <span className="sys-label">Режим</span>
            <span className="sys-value">Офлайн — вход без аккаунта Mojang</span>
          </div>
          <div className="sys-row">
            <span className="sys-label">Загрузка</span>
            <span className="sys-value">
              Vanilla и Fabric · Forge — в следующей части
            </span>
          </div>
          <div className="sys-row">
            <span className="sys-label">Контент</span>
            <span className="sys-value">Modrinth: моды, ресурспаки, шейдеры, сборки</span>
          </div>
          <div className="sys-row">
            <span className="sys-label">Файлы</span>
            <span className="sys-value">Параллельно · 8 потоков · проверка SHA-1</span>
          </div>
          <div className="sys-row">
            <span className="sys-label">Стек</span>
            <span className="sys-value">Tauri 2 · React 18 · TypeScript · Rust</span>
          </div>
        </section>
      </div>
    </div>
  );
}
