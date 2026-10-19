import { useMemo, useState } from "react";
import {
  Cpu,
  Download,
  FolderOpen,
  Gamepad2,
  HardDrive,
  Info,
  RefreshCw,
  User,
} from "lucide-react";
import type {
  DownloadTask,
  JavaInfo,
  LauncherSettings,
  VersionList,
} from "../types";
import { DownloadRow } from "./DownloadBar";

const LOADERS = [
  { id: "vanilla" as const, label: "Vanilla" },
  { id: "fabric" as const, label: "Fabric" },
  { id: "forge" as const, label: "Forge" },
];

interface Props {
  settings: LauncherSettings;
  onPatch: (p: Partial<LauncherSettings>) => void;
  java: JavaInfo | null;
  onRefreshJava: () => void;
  versions: VersionList | null;
  versionsLoading: boolean;
  onRefreshVersions: () => void;
  queue: DownloadTask[];
  onDownloadVersion: (versionId: string, loader: string) => void;
}

function cacheLabel(versions: VersionList | null): string {
  if (!versions) return "список не загружен";
  if (!versions.fetchedAt) return "кеш";
  const date = new Date(versions.fetchedAt * 1000);
  const time = date.toLocaleString("ru-RU", {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
  return versions.fromCache ? `кеш · ${time}` : `обновлено · ${time}`;
}

export function HomeTab({
  settings,
  onPatch,
  java,
  onRefreshJava,
  versions,
  versionsLoading,
  onRefreshVersions,
  queue,
  onDownloadVersion,
}: Props) {
  const [nickError, setNickError] = useState<string | null>(null);

  const handleNick = (value: string) => {
    setNickError(value.length > 16 ? "Максимум 16 символов" : null);
    onPatch({ nickname: value });
  };

  const groups = useMemo(() => {
    const list = versions?.versions ?? [];
    const releases = list.filter((version) => version.type === "release");
    const snapshots = list.filter((version) => version.type === "snapshot").slice(0, 40);
    const old = list.filter((version) => version.type.startsWith("old"));
    const result = [
      { label: "Релизы", items: releases },
      { label: "Снапшоты", items: snapshots },
      { label: "Старые версии", items: old },
    ].filter((group) => group.items.length > 0);
    const hasSelected = list.some((version) => version.id === settings.versionId);
    if (!hasSelected && settings.versionId) {
      result.unshift({
        label: "Выбрано",
        items: [
          {
            id: settings.versionId,
            type: "release",
            releaseTime: "",
            url: "",
          },
        ],
      });
    }
    return result;
  }, [versions, settings.versionId]);

  const versionTasks = queue.filter((task) => task.kind === "version");
  const activeTask =
    versionTasks.find((task) => task.status === "running" || task.status === "queued") ??
    versionTasks[versionTasks.length - 1];
  const downloading = versionTasks.some(
    (task) => task.status === "running" || task.status === "queued",
  );
  const releaseCount = (versions?.versions ?? []).filter(
    (version) => version.type === "release",
  ).length;

  return (
    <div className="home-grid">
      <section className="card profile-card">
        <div className="card-head">
          <span className="card-ic">
            <User size={18} />
          </span>
          <div>
            <h2>Профиль</h2>
            <p>Ник и параметры запуска</p>
          </div>
          <span className={`badge ${java?.found ? "ok" : "warn"}`}>
            {java?.found ? "Java найдена" : "Java не найдена"}
          </span>
        </div>

        <label className="field">
          <span className="field-label">Никнейм</span>
          <input
            className="input"
            type="text"
            value={settings.nickname}
            maxLength={16}
            placeholder="Steve"
            onChange={(e) => handleNick(e.target.value)}
          />
          <span className="field-hint">
            {nickError ?? (
              <span className="muted">
                1–16 символов: латиница, цифры, «_» · аккаунт Mojang не требуется
              </span>
            )}
          </span>
        </label>

        <div className="row2">
          <div className="field">
            <div className="field-row">
              <span className="field-label">Версия</span>
              <button
                className="mini-link"
                type="button"
                onClick={onRefreshVersions}
                disabled={versionsLoading}
                title="Обновить список версий из piston-meta"
              >
                <RefreshCw size={12} className={versionsLoading ? "spin" : undefined} />
                {versionsLoading ? "обновляем…" : cacheLabel(versions)}
              </button>
            </div>
            <select
              className="input select"
              value={settings.versionId}
              onChange={(e) => onPatch({ versionId: e.target.value })}
            >
              {groups.map((group) => (
                <optgroup key={group.label} label={group.label}>
                  {group.items.map((version) => (
                    <option key={version.id} value={version.id}>
                      {version.id}
                    </option>
                  ))}
                </optgroup>
              ))}
            </select>
          </div>

          <label className="field">
            <span className="field-label">Папка игры</span>
            <input
              className="input"
              type="text"
              value={settings.gameDir ?? ""}
              placeholder="по умолчанию — папка данных приложения"
              onChange={(e) => onPatch({ gameDir: e.target.value.trim() || null })}
            />
          </label>
        </div>

        <div className="field">
          <span className="field-label">Загрузчик</span>
          <div className="segmented">
            {LOADERS.map((l) => (
              <button
                key={l.id}
                className={`seg ${settings.loader === l.id ? "active" : ""}`}
                onClick={() => onPatch({ loader: l.id })}
                type="button"
              >
                {l.label}
              </button>
            ))}
          </div>
          {settings.loader === "forge" && (
            <span className="field-hint muted">
              Forge появится в следующей части — для модов выберите Fabric.
            </span>
          )}
        </div>

        <div className="download-version">
          <div className="download-version-head">
            <span className="field-label">
              <Download size={14} className="inline-ic" /> Файлы игры
            </span>
            <span className="muted">
              {versions ? `${releaseCount} релизов в списке` : "манифест Mojang"}
            </span>
          </div>
          <button
            className="btn btn-accent download-version-btn"
            type="button"
            disabled={downloading || !settings.versionId}
            onClick={() => onDownloadVersion(settings.versionId, settings.loader)}
          >
            <Download size={15} />
            {downloading
              ? "Скачивание…"
              : `Скачать ${settings.versionId}${settings.loader !== "vanilla" ? ` · ${settings.loader}` : ""}`}
          </button>
          <span className="field-hint muted">
            Клиент, библиотеки и ассеты — параллельно в {8} потоков, с проверкой SHA-1.
          </span>
        </div>

        {activeTask && (
          <div className="home-download">
            <DownloadRow task={activeTask} compact />
          </div>
        )}

        <div className="field">
          <div className="ram-head">
            <span className="field-label">
              <HardDrive size={14} className="inline-ic" /> Оперативная память
            </span>
            <span className="ram-value">{settings.ramGb} ГБ</span>
          </div>
          <input
            className="range"
            type="range"
            min={1}
            max={16}
            step={1}
            value={settings.ramGb}
            onChange={(e) => onPatch({ ramGb: Number(e.target.value) })}
          />
          <div className="ram-scale">
            <span>1</span>
            <span>8</span>
            <span>16 ГБ</span>
          </div>
        </div>
      </section>

      <section className="card">
        <div className="card-head">
          <span className="card-ic">
            <Cpu size={18} />
          </span>
          <div>
            <h2>Система</h2>
            <p>Состояние окружения</p>
          </div>
        </div>

        <div className="sys-row">
          <span className="sys-label">Java</span>
          {java?.found ? (
            <span className="sys-value">
              v{java.major ?? "?"} · <code>{java.path}</code>
            </span>
          ) : (
            <span className="sys-value warn">не найдена</span>
          )}
        </div>
        <div className="sys-row">
          <span className="sys-label">Источник</span>
          <span className="sys-value">{java?.found ? java.source : "—"}</span>
        </div>
        <div className="sys-row">
          <span className="sys-label">Режим входа</span>
          <span className="sys-value">Офлайн (без аккаунта)</span>
        </div>
        <div className="sys-row">
          <span className="sys-label">Версия</span>
          <span className="sys-value">
            {settings.versionId} · {settings.loader}
          </span>
        </div>
        <div className="sys-row">
          <span className="sys-label">Манифест</span>
          <span className="sys-value">
            <Info size={13} className="inline-ic" /> piston-meta · {cacheLabel(versions)}
          </span>
        </div>

        {java && !java.found && (
          <div className="java-fix">
            <Gamepad2 size={16} className="inline-ic" /> Java не обнаружена. Скачайте JDK во
            вкладке «Настройки» — лаунчер установит его в свою папку автоматически.
          </div>
        )}

        <div className="card-foot">
          <span className="muted">
            <FolderOpen size={14} className="inline-ic" /> Файлы игры можно скачать заранее —
            кнопка в карточке профиля
          </span>
          <button className="link" onClick={onRefreshJava} type="button">
            <RefreshCw size={14} className="inline-ic" /> Проверить Java
          </button>
        </div>
      </section>
    </div>
  );
}
