import { useCallback, useEffect, useState } from "react";
import {
  Bug,
  CheckCircle2,
  Cpu,
  Download,
  FolderOpen,
  Globe,
  HardDrive,
  Info,
  Languages,
  Palette,
  RefreshCw,
  ShieldCheck,
  Sparkles,
  Trash2,
  User,
  Users,
  XCircle,
} from "lucide-react";
import type { JavaInfo, LauncherSettings, ProgressEvent, UpdateInfo } from "../types";
import { checkForUpdates, openGameFolder, openLogsFolder } from "../lib/api";

type Section = "general" | "game" | "java" | "accounts" | "updates";

interface Props {
  settings: LauncherSettings;
  onPatch: (p: Partial<LauncherSettings>) => void;
  java: JavaInfo | null;
  jdk: ProgressEvent | null;
  onRefreshJava: () => void;
  onDownloadJdk: () => void;
}

const SECTIONS: { id: Section; label: string; icon: any; desc: string }[] = [
  { id: "general", label: "Общие", icon: Palette, desc: "Язык и тема" },
  { id: "game", label: "Игра", icon: HardDrive, desc: "RAM и папки" },
  { id: "java", label: "Java", icon: Cpu, desc: "JDK и путь" },
  { id: "accounts", label: "Аккаунты", icon: Users, desc: "Офлайн-профили" },
  { id: "updates", label: "Обновления", icon: Sparkles, desc: "Лаунчер" },
];

export function SettingsTab({ settings, onPatch, java, jdk, onRefreshJava, onDownloadJdk }: Props) {
  const [section, setSection] = useState<Section>("general");
  const [newProfile, setNewProfile] = useState("");
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [checking, setChecking] = useState(false);
  const downloading = jdk !== null && jdk.stage !== "done";

  const refreshUpdate = useCallback(async (force = false) => {
    setChecking(true);
    try {
      const info = await checkForUpdates(force);
      setUpdateInfo(info);
    } catch (e) {
      setUpdateInfo({
        currentVersion: "0.3.0",
        latestVersion: null,
        hasUpdate: false,
        checkedAt: Math.floor(Date.now() / 1000),
        changelog: String(e),
        downloadUrl: null,
      });
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    refreshUpdate(false);
  }, [refreshUpdate]);

  const addProfile = () => {
    const name = newProfile.trim();
    if (!name) return;
    if (name.length > 16 || !/^[a-zA-Z0-9_]+$/.test(name)) return;
    const exists = settings.profiles.some((p) => p.name.toLowerCase() === name.toLowerCase());
    if (exists) {
      onPatch({ nickname: name });
      setNewProfile("");
      return;
    }
    const next = [...settings.profiles, { name, createdAt: Math.floor(Date.now() / 1000) }];
    onPatch({ profiles: next, nickname: name });
    setNewProfile("");
  };

  const switchProfile = (name: string) => {
    onPatch({ nickname: name });
  };

  const deleteProfile = (name: string) => {
    const next = settings.profiles.filter((p) => p.name !== name);
    // if we delete current, fallback to first or empty
    let patch: Partial<LauncherSettings> = { profiles: next };
    if (settings.nickname === name) {
      patch.nickname = next[0]?.name ?? "";
    }
    onPatch(patch);
  };

  return (
    <div>
      <div className="tab-head">
        <h1>Настройки</h1>
        <p>Все изменения сохраняются в JSON и применяются сразу — без перезагрузки</p>
      </div>

      <div className="settings-layout">
        {/* Left nav */}
        <nav className="settings-nav">
          {SECTIONS.map((s) => {
            const Icon = s.icon;
            const active = section === s.id;
            return (
              <button key={s.id} className={`settings-nav-btn ${active ? "active" : ""}`} onClick={() => setSection(s.id)} type="button">
                <span className="settings-nav-ic">
                  <Icon size={18} />
                </span>
                <span className="settings-nav-tx">
                  <b>{s.label}</b>
                  <span>{s.desc}</span>
                </span>
              </button>
            );
          })}
          <div className="settings-nav-foot">
            <Info size={14} />
            <span>v0.3.0 · часть 3/4</span>
          </div>
        </nav>

        {/* Right content */}
        <div className="settings-content">
          {section === "general" && (
            <div className="settings-section">
              <section className="card">
                <div className="card-head">
                  <span className="card-ic">
                    <Languages size={18} />
                  </span>
                  <div>
                    <h2>Общие</h2>
                    <p>Язык интерфейса и внешний вид</p>
                  </div>
                </div>

                <label className="field">
                  <span className="field-label">
                    <Globe size={12} /> Язык
                  </span>
                  <select className="input select" value={settings.language} onChange={(e) => onPatch({ language: e.target.value })}>
                    <option value="ru">Русский</option>
                    <option value="en">English</option>
                  </select>
                  <span className="field-hint muted">Применяется мгновенно — перезагрузка не нужна</span>
                </label>

                <label className="field">
                  <span className="field-label">
                    <Palette size={12} /> Тема
                  </span>
                  <div className="segmented">
                    <button className={`seg ${settings.theme === "dark" ? "active" : ""}`} onClick={() => onPatch({ theme: "dark" })} type="button">
                      Тёмная
                    </button>
                    <button className={`seg ${settings.theme === "light" ? "active" : ""}`} onClick={() => onPatch({ theme: "light" })} type="button">
                      Светлая
                    </button>
                  </div>
                  <span className="field-hint muted">Лаунчер использует CSS-переменные — тема меняется на лету</span>
                </label>

                <div className="settings-preview">
                  <span className="muted small">Предпросмотр темы:</span>
                  <div className={`theme-dot theme-${settings.theme}`} />
                  <code className="muted small">{settings.theme === "light" ? "Светлая · мягкий фон" : "Тёмная · неон"}</code>
                </div>
              </section>

              <section className="card">
                <div className="card-head">
                  <span className="card-ic">
                    <ShieldCheck size={18} />
                  </span>
                  <div>
                    <h2>О лаунчере</h2>
                    <p>MyLauncher · lighty-launcher</p>
                  </div>
                </div>
                <div className="sys-row">
                  <span className="sys-label">Версия</span>
                  <span className="sys-value">0.3.0 (часть 3/4)</span>
                </div>
                <div className="sys-row">
                  <span className="sys-label">Режим</span>
                  <span className="sys-value">Офлайн — вход без аккаунта Mojang</span>
                </div>
                <div className="sys-row">
                  <span className="sys-label">Язык</span>
                  <span className="sys-value">{settings.language === "en" ? "English" : "Русский"}</span>
                </div>
                <div className="sys-row">
                  <span className="sys-label">Стек</span>
                  <span className="sys-value">Tauri 2 · React 18 · Rust · Modrinth</span>
                </div>
              </section>
            </div>
          )}

          {section === "game" && (
            <div className="settings-section">
              <section className="card">
                <div className="card-head">
                  <span className="card-ic">
                    <HardDrive size={18} />
                  </span>
                  <div>
                    <h2>Игра</h2>
                    <p>Параметры запуска Minecraft</p>
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
                  <span className="field-hint muted">Выделяется как -Xmx, половина идёт в -Xms. Рекомендуется 4–8 ГБ.</span>
                </div>

                <label className="field">
                  <span className="field-label">
                    <FolderOpen size={12} /> Папка .minecraft
                  </span>
                  <div className="inline-2">
                    <input
                      className="input"
                      type="text"
                      value={settings.gameDir ?? ""}
                      placeholder="пусто — папка данных приложения"
                      onChange={(e) => onPatch({ gameDir: e.target.value || null })}
                    />
                    <button
                      className="btn"
                      type="button"
                      onClick={() => openGameFolder().catch(() => {})}
                      title="Открыть папку игры"
                    >
                      <FolderOpen size={14} /> Открыть
                    </button>
                  </div>
                  <span className="field-hint muted">Сохраняется автоматически. Текущая: {settings.gameDir ?? "папка по умолчанию"}</span>
                </label>

                <label className="field">
                  <span className="field-label">Дополнительные JVM-аргументы</span>
                  <textarea
                    className="input textarea"
                    value={settings.jvmArgs}
                    placeholder="-XX:+UseG1GC -Dfile.encoding=UTF-8"
                    rows={3}
                    onChange={(e) => onPatch({ jvmArgs: e.target.value })}
                  />
                  <span className="field-hint muted">Подставляются перед -cp. Кавычки поддерживаются. Пусто — без доп. аргументов.</span>
                </label>
              </section>
            </div>
          )}

          {section === "java" && (
            <div className="settings-section">
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
                  {java?.found ? <CheckCircle2 size={20} className="ok-ic" /> : <XCircle size={20} className="err-ic" />}
                  <div className="java-texts">
                    <b>{java?.found ? `Найдена: ${java.version ?? "Java"}` : "Java не найдена"}</b>
                    {java?.found && java.path && <code>{java.path}</code>}
                    {java?.found && <span className="muted">Источник: {java.source}</span>}
                    {!java?.found && <span className="muted">Укажите путь вручную или скачайте JDK</span>}
                  </div>
                  <div className="java-actions">
                    <button className="btn" type="button" onClick={onRefreshJava} disabled={downloading}>
                      <RefreshCw size={15} /> Обновить
                    </button>
                    <button className="btn btn-accent" type="button" onClick={onDownloadJdk} disabled={downloading}>
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

                <label className="field">
                  <span className="field-label">Ручной путь к Java</span>
                  <input
                    className="input"
                    type="text"
                    value={settings.javaPath ?? ""}
                    placeholder="/usr/lib/jvm/java-21-openjdk/bin/java или C:\Program Files\Java\...\java.exe"
                    onChange={(e) => onPatch({ javaPath: e.target.value.trim() || null })}
                  />
                  <span className="field-hint muted">Приоритет выше автоопределения. Можно указать как путь к бинарнику, так и к папке JDK.</span>
                </label>

                <div style={{ display: "flex", gap: 8 }}>
                  <button className="btn" type="button" onClick={onRefreshJava}>
                    <RefreshCw size={14} /> Проверить версию
                  </button>
                  <button className="btn" type="button" onClick={() => onPatch({ javaPath: null })}>
                    <Trash2 size={14} /> Сбросить путь
                  </button>
                </div>

                {!java?.found && !downloading && (
                  <p className="hint">Рекомендуется JDK 21 (Adoptium). Кнопка выше скачает и установит его в папку лаунчера — отдельная настройка системы не нужна.</p>
                )}
              </section>

              <section className="card">
                <div className="card-head">
                  <span className="card-ic">
                    <Info size={18} />
                  </span>
                  <div>
                    <h2>Совместимость</h2>
                    <p>Какая Java нужна для версий</p>
                  </div>
                </div>
                <div className="sys-row">
                  <span className="sys-label">1.20.5+</span>
                  <span className="sys-value">Java 21</span>
                </div>
                <div className="sys-row">
                  <span className="sys-label">1.17–1.20.4</span>
                  <span className="sys-value">Java 17</span>
                </div>
                <div className="sys-row">
                  <span className="sys-label">1.12–1.16</span>
                  <span className="sys-value">Java 8 / 11</span>
                </div>
                <span className="muted small">Лаунчер сам выберет подходящую, если установлено несколько JDK.</span>
              </section>
            </div>
          )}

          {section === "accounts" && (
            <div className="settings-section">
              <section className="card">
                <div className="card-head">
                  <span className="card-ic">
                    <Users size={18} />
                  </span>
                  <div>
                    <h2>Аккаунты</h2>
                    <p>Офлайн-профили · переключение без перезагрузки</p>
                  </div>
                </div>

                <div className="field">
                  <span className="field-label">Текущий ник</span>
                  <div className="inline-2">
                    <input
                      className="input"
                      type="text"
                      value={settings.nickname}
                      maxLength={16}
                      placeholder="Steve"
                      onChange={(e) => onPatch({ nickname: e.target.value })}
                    />
                    <span className={`badge ${settings.nickname ? "ok" : "warn"}`}>{settings.nickname || "не задан"}</span>
                  </div>
                  <span className="field-hint muted">1–16 символов: латиница, цифры, «_». Сохраняется мгновенно.</span>
                </div>

                <label className="field">
                  <span className="field-label">Добавить профиль</span>
                  <div className="inline-2">
                    <input
                      className="input"
                      type="text"
                      value={newProfile}
                      maxLength={16}
                      placeholder="Новый ник"
                      onChange={(e) => setNewProfile(e.target.value)}
                      onKeyDown={(e) => e.key === "Enter" && addProfile()}
                    />
                    <button className="btn btn-accent" type="button" onClick={addProfile} disabled={!newProfile.trim()}>
                      <User size={14} /> Добавить
                    </button>
                  </div>
                </label>

                {settings.profiles.length === 0 ? (
                  <div className="hint">Профилей пока нет — добавьте ник выше, он появится в списке. Переключение — один клик.</div>
                ) : (
                  <div className="profiles-list">
                    {settings.profiles.map((p) => {
                      const active = p.name === settings.nickname;
                      return (
                        <div key={p.name} className={`profile-row ${active ? "active" : ""}`}>
                          <span className="profile-ic">
                            <User size={15} />
                          </span>
                          <div className="profile-texts">
                            <b>{p.name}</b>
                            <span className="muted">{p.createdAt ? new Date(p.createdAt * 1000).toLocaleDateString("ru-RU") : "профиль"}</span>
                          </div>
                          <div className="profile-actions">
                            {!active && (
                              <button className="btn" type="button" onClick={() => switchProfile(p.name)}>
                                Выбрать
                              </button>
                            )}
                            {active && <span className="badge ok">Активен</span>}
                            <button className="icon-btn" type="button" title="Удалить профиль" onClick={() => deleteProfile(p.name)}>
                              <Trash2 size={14} />
                            </button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}

                <div style={{ display: "flex", gap: 8 }}>
                  <button
                    className="btn"
                    type="button"
                    onClick={() => {
                      if (!settings.nickname) return;
                      const exists = settings.profiles.some((p) => p.name === settings.nickname);
                      if (!exists) {
                        onPatch({ profiles: [...settings.profiles, { name: settings.nickname, createdAt: Math.floor(Date.now() / 1000) }] });
                      }
                    }}
                  >
                    <Users size={14} /> Сохранить текущий как профиль
                  </button>
                  <button className="btn" type="button" onClick={() => openGameFolder().catch(() => {})}>
                    <FolderOpen size={14} /> Папка игры
                  </button>
                </div>
              </section>
            </div>
          )}

          {section === "updates" && (
            <div className="settings-section">
              <section className="card">
                <div className="card-head">
                  <span className="card-ic">
                    <Sparkles size={18} />
                  </span>
                  <div>
                    <h2>Обновления</h2>
                    <p>Автообновление лаунчера</p>
                  </div>
                </div>

                <label className="field" style={{ flexDirection: "row", alignItems: "center", justifyContent: "space-between" }}>
                  <span className="field-label">Автообновление</span>
                  <label className="switch">
                    <input type="checkbox" checked={settings.autoUpdate} onChange={(e) => onPatch({ autoUpdate: e.target.checked })} />
                    <span className="switch-slider" />
                  </label>
                </label>
                <span className="field-hint muted">Проверять наличие новой версии при запуске (GitHub Releases)</span>

                <div className="sys-row">
                  <span className="sys-label">Текущая</span>
                  <span className="sys-value">{updateInfo?.currentVersion ?? "0.3.0"}</span>
                </div>
                <div className="sys-row">
                  <span className="sys-label">Последняя</span>
                  <span className="sys-value">{updateInfo?.latestVersion ?? (checking ? "проверка…" : "—")}</span>
                </div>
                <div className="sys-row">
                  <span className="sys-label">Статус</span>
                  <span className={`sys-value ${updateInfo?.hasUpdate ? "warn" : ""}`}>
                    {updateInfo?.hasUpdate ? "Доступно обновление" : checking ? "Проверяем…" : "У вас последняя версия"}
                  </span>
                </div>
                {updateInfo?.changelog && <p className="hint">{updateInfo.changelog}</p>}

                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <button className="btn btn-accent" type="button" onClick={() => refreshUpdate(true)} disabled={checking}>
                    <RefreshCw size={14} className={checking ? "spin" : undefined} /> {checking ? "Проверяем…" : "Проверить сейчас"}
                  </button>
                  {updateInfo?.downloadUrl && (
                    <a className="btn" href={updateInfo.downloadUrl} target="_blank" rel="noreferrer">
                      <Download size={14} /> Страница релизов
                    </a>
                  )}
                  <button className="btn" type="button" onClick={() => openLogsFolder("logs").catch(() => {})}>
                    <FolderOpen size={14} /> Логи
                  </button>
                </div>
                <span className="muted small">
                  Последняя проверка: {updateInfo?.checkedAt ? new Date(updateInfo.checkedAt * 1000).toLocaleString("ru-RU") : "ещё не проверяли"}
                </span>
              </section>

              <section className="card">
                <div className="card-head">
                  <span className="card-ic">
                    <Info size={18} />
                  </span>
                  <div>
                    <h2>Диагностика</h2>
                    <p>Если игра вылетает — логи помогут</p>
                  </div>
                </div>
                <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                  <button className="btn" type="button" onClick={() => openLogsFolder("logs")}>
                    <FolderOpen size={14} /> Открыть логи
                  </button>
                  <button className="btn" type="button" onClick={() => openLogsFolder("crash")}>
                    <Bug size={14} /> Crash-reports
                  </button>
                  <button className="btn" type="button" onClick={() => openGameFolder()}>
                    <HardDrive size={14} /> Папка игры
                  </button>
                </div>
                <p className="hint">При вылете лаунчер покажет модальное окно с текстом ошибки, latest.log и crash-reports — есть кнопки «Копировать лог» и «Открыть папку с логами».</p>
              </section>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}


