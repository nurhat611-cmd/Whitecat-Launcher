import { useState } from "react";
import {
  Cpu,
  FolderOpen,
  Gamepad2,
  HardDrive,
  RefreshCw,
  User,
} from "lucide-react";
import type { JavaInfo, LauncherSettings } from "../types";
import { VERSIONS } from "../types";

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
}

export function HomeTab({ settings, onPatch, java, onRefreshJava }: Props) {
  const [nickError, setNickError] = useState<string | null>(null);

  const handleNick = (value: string) => {
    setNickError(value.length > 16 ? "Максимум 16 символов" : null);
    onPatch({ nickname: value });
  };

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
          <label className="field">
            <span className="field-label">Версия</span>
            <select
              className="input select"
              value={settings.versionId}
              onChange={(e) => onPatch({ versionId: e.target.value })}
            >
              {VERSIONS.map((v) => (
                <option key={v} value={v}>
                  {v}
                </option>
              ))}
            </select>
          </label>

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
              >
                {l.label}
              </button>
            ))}
          </div>
        </div>

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

        {java && !java.found && (
          <div className="java-fix">
            <Gamepad2 size={16} className="inline-ic" /> Java не обнаружена. Скачайте JDK во
            вкладке «Настройки» — лаунчер установит его в свою папку автоматически.
          </div>
        )}

        <div className="card-foot">
          <span className="muted">
            <FolderOpen size={14} className="inline-ic" /> Клиент и ассеты скачаются при первом
            запуске
          </span>
          <button className="link" onClick={onRefreshJava} type="button">
            <RefreshCw size={14} className="inline-ic" /> Проверить Java
          </button>
        </div>
      </section>
    </div>
  );
}
