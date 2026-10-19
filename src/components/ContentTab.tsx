import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertCircle,
  Boxes,
  Check,
  Download,
  FolderDown,
  Loader2,
  Package,
  Palette,
  Puzzle,
  Search,
  SlidersHorizontal,
  Sun,
  Users,
} from "lucide-react";
import type {
  ContentHit,
  DownloadTask,
  InstanceInfo,
  LauncherSettings,
  ProjectType,
  VersionList,
} from "../types";
import {
  cancelDownload,
  clearFinishedDownloads,
  enqueueContent,
  isTauri,
  listInstances,
  removeDownload,
  searchContent,
} from "../lib/api";
import { DownloadQueuePanel, formatCount } from "./DownloadBar";

const TYPES: {
  id: ProjectType;
  label: string;
  icon: typeof Puzzle;
  usesLoader: boolean;
}[] = [
  { id: "mod", label: "Моды", icon: Puzzle, usesLoader: true },
  { id: "resourcepack", label: "Ресурспаки", icon: Palette, usesLoader: false },
  { id: "shader", label: "Шейдеры", icon: Sun, usesLoader: false },
  { id: "modpack", label: "Сборки", icon: Boxes, usesLoader: true },
];

const LOADERS = [
  { id: "", label: "Любой загрузчик" },
  { id: "fabric", label: "Fabric" },
  { id: "forge", label: "Forge" },
  { id: "neoforge", label: "NeoForge" },
  { id: "quilt", label: "Quilt" },
];

const SORTS = [
  { id: "relevance", label: "По релевантности" },
  { id: "downloads", label: "По скачиваниям" },
  { id: "follows", label: "По подпискам" },
  { id: "newest", label: "Сначала новые" },
  { id: "updated", label: "Недавно обновлённые" },
];

const FALLBACK_VERSIONS = ["1.21.4", "1.21.1", "1.20.6", "1.20.1", "1.19.4", "1.18.2", "1.16.5", "1.12.2"];

const PAGE_SIZE = 20;

interface Props {
  versions: VersionList | null;
  settings: LauncherSettings;
  queue: DownloadTask[];
}

function Icon({ url, title }: { url: string | null; title: string }) {
  const [broken, setBroken] = useState(false);
  if (url && !broken) {
    return (
      <img
        className="mod-icon"
        src={url}
        alt=""
        loading="lazy"
        onError={() => setBroken(true)}
      />
    );
  }
  return <span className="mod-icon mod-icon-empty">{title.slice(0, 1).toUpperCase()}</span>;
}

interface CardProps {
  hit: ContentHit;
  task: DownloadTask | undefined;
  installed: boolean;
  onInstall: (hit: ContentHit) => void;
}

function ContentCard({ hit, task, installed, onInstall }: CardProps) {
  const busy = task !== undefined && (task.status === "queued" || task.status === "running");
  const failed = task?.status === "error";
  let label = "Установить";
  if (task?.status === "running") label = `${task.progress.percent}%`;
  else if (task?.status === "queued") label = "В очереди";
  else if (busy) label = "…";
  else if (installed) label = "Установлено";

  return (
    <article className="mod-card">
      <div className="mod-head">
        <Icon url={hit.iconUrl} title={hit.title} />
        <div className="mod-titles">
          <h3 title={hit.title}>{hit.title}</h3>
          <span className="mod-author">
            <Users size={12} /> {hit.author}
          </span>
        </div>
      </div>
      <p className="mod-desc">{hit.description || "Без описания"}</p>
      <div className="mod-tags">
        {hit.categories.slice(0, 3).map((category) => (
          <span className="tag" key={category}>
            {category}
          </span>
        ))}
        <span className="tag tag-count">
          <Download size={11} /> {formatCount(hit.downloads)}
        </span>
      </div>
      <button
        className={`btn install-btn${installed ? " installed" : ""}${failed ? " failed" : ""}`}
        type="button"
        disabled={busy || installed || !isTauri()}
        title={!isTauri() ? "Установка доступна в приложении (npm run tauri dev)" : undefined}
        onClick={() => onInstall(hit)}
      >
        {task?.status === "running" ? (
          <Loader2 size={15} className="spin" />
        ) : installed ? (
          <Check size={15} />
        ) : (
          <FolderDown size={15} />
        )}
        {label}
      </button>
    </article>
  );
}

export function ContentTab({ versions, settings, queue }: Props) {
  const [projectType, setProjectType] = useState<ProjectType>("mod");
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [gameVersion, setGameVersion] = useState(settings.versionId || "1.21.4");
  const [loader, setLoader] = useState(
    settings.loader === "fabric" ? "fabric" : settings.loader === "forge" ? "forge" : "",
  );
  const [sort, setSort] = useState("relevance");
  const [hits, setHits] = useState<ContentHit[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [installError, setInstallError] = useState<string | null>(null);
  const [instances, setInstances] = useState<InstanceInfo[]>([]);
  const requestRef = useRef(0);

  const typeInfo = TYPES.find((item) => item.id === projectType) ?? TYPES[0];
  const usesLoader = typeInfo.usesLoader;
  const activeLoader = usesLoader ? loader || null : null;

  const versionOptions = useMemo(() => {
    const fromManifest = (versions?.versions ?? [])
      .filter((version) => version.type === "release")
      .map((version) => version.id);
    const list = fromManifest.length > 0 ? fromManifest : FALLBACK_VERSIONS;
    if (gameVersion && !list.includes(gameVersion)) {
      return [gameVersion, ...list];
    }
    return list;
  }, [versions, gameVersion]);

  // Debounce строки поиска.
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(query.trim()), 350);
    return () => clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);
    const timer = setTimeout(async () => {
      try {
        const result = await searchContent({
          query: debounced,
          projectType,
          gameVersion: gameVersion || null,
          loader: activeLoader,
          sort,
          offset: 0,
          limit: PAGE_SIZE,
        });
        if (requestRef.current !== requestId) return;
        setHits(result.hits);
        setTotal(result.totalHits);
      } catch (e) {
        if (requestRef.current !== requestId) return;
        setHits([]);
        setTotal(0);
        setError(String(e));
      } finally {
        if (requestRef.current === requestId) setLoading(false);
      }
    }, 180);
    return () => clearTimeout(timer);
  }, [debounced, projectType, gameVersion, activeLoader, sort]);

  // Обновляем список сборок, когда завершилась установка модпака.
  const finishedModpacks = queue.filter(
    (task) => task.kind === "modpack" && task.status === "done",
  ).length;
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    listInstances()
      .then((list) => {
        if (!disposed) setInstances(list);
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, [finishedModpacks]);

  const loadMore = useCallback(async () => {
    if (loadingMore) return;
    setLoadingMore(true);
    try {
      const result = await searchContent({
        query: debounced,
        projectType,
        gameVersion: gameVersion || null,
        loader: activeLoader,
        sort,
        offset: hits.length,
        limit: PAGE_SIZE,
      });
      setHits((prev) => {
        const seen = new Set(prev.map((hit) => hit.id));
        return [...prev, ...result.hits.filter((hit) => !seen.has(hit.id))];
      });
      setTotal(result.totalHits);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoadingMore(false);
    }
  }, [loadingMore, debounced, projectType, gameVersion, activeLoader, sort, hits.length]);

  const install = useCallback(
    async (hit: ContentHit) => {
      setInstallError(null);
      if (!isTauri()) {
        setInstallError("Установка работает в приложении: запустите npm run tauri dev");
        return;
      }
      const loaderLabel =
        usesLoader && loader ? LOADERS.find((item) => item.id === loader)?.label : null;
      const subtitle = [gameVersion || "любая версия", loaderLabel]
        .filter(Boolean)
        .join(" · ");
      try {
        await enqueueContent({
          projectId: hit.id,
          projectType: (hit.projectType as ProjectType) || projectType,
          title: hit.title,
          subtitle,
          iconUrl: hit.iconUrl,
          gameVersion: gameVersion || null,
          loader: activeLoader,
        });
      } catch (e) {
        setInstallError(String(e));
      }
    },
    [gameVersion, loader, usesLoader, projectType, activeLoader],
  );

  const cancel = useCallback((id: string) => {
    cancelDownload(id).catch(() => undefined);
  }, []);
  const remove = useCallback((id: string) => {
    removeDownload(id).catch(() => undefined);
  }, []);
  const clear = useCallback(() => {
    clearFinishedDownloads().catch(() => undefined);
  }, []);

  const taskFor = (hit: ContentHit): DownloadTask | undefined => {
    let latest: DownloadTask | undefined;
    for (const task of queue) {
      if (task.projectId !== hit.id) continue;
      if (!latest || task.createdAt >= latest.createdAt) latest = task;
    }
    return latest;
  };

  return (
    <div>
      <div className="tab-head">
        <h1>Контент</h1>
        <p>Моды, ресурспаки, шейдеры и сборки из Modrinth</p>
      </div>

      <div className="content-toolbar">
        <div className="type-tabs">
          {TYPES.map((type) => {
            const TypeIcon = type.icon;
            return (
              <button
                key={type.id}
                type="button"
                className={`type-tab${projectType === type.id ? " active" : ""}`}
                onClick={() => setProjectType(type.id)}
              >
                <TypeIcon size={15} />
                {type.label}
              </button>
            );
          })}
        </div>

        <div className="search-box">
          <Search size={16} className="search-ic" />
          <input
            className="input search-input"
            type="text"
            value={query}
            placeholder="Поиск по Modrinth — например, Sodium или Create"
            onChange={(event) => setQuery(event.target.value)}
          />
          {query && (
            <button
              className="search-clear"
              type="button"
              title="Очистить"
              onClick={() => setQuery("")}
            >
              ×
            </button>
          )}
        </div>

        <div className="filters">
          <span className="filters-label">
            <SlidersHorizontal size={14} /> Фильтры
          </span>
          <label className="inline-field">
            <span>Версия</span>
            <select
              className="input select small-select"
              value={gameVersion}
              onChange={(event) => setGameVersion(event.target.value)}
            >
              <option value="">Любая</option>
              {versionOptions.map((version) => (
                <option key={version} value={version}>
                  {version}
                </option>
              ))}
            </select>
          </label>
          {usesLoader && (
            <label className="inline-field">
              <span>Загрузчик</span>
              <select
                className="input select small-select"
                value={loader}
                onChange={(event) => setLoader(event.target.value)}
              >
                {LOADERS.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.label}
                  </option>
                ))}
              </select>
            </label>
          )}
          <label className="inline-field">
            <span>Сортировка</span>
            <select
              className="input select small-select"
              value={sort}
              onChange={(event) => setSort(event.target.value)}
            >
              {SORTS.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.label}
                </option>
              ))}
            </select>
          </label>
        </div>
      </div>

      {!isTauri() && (
        <div className="notice">
          Открыт предпросмотр в браузере: поиск и список версий работают напрямую через API,
          установка и запуск игры доступны в приложении <code>npm run tauri dev</code>.
        </div>
      )}

      {installError && (
        <div className="notice notice-err">
          <AlertCircle size={15} /> {installError}
        </div>
      )}

      <div className="results-head">
        <span className="muted">
          {loading ? "Ищем…" : `Найдено ${formatCount(total)} проектов`}
          {versions?.stale ? " · список версий из кеша" : ""}
        </span>
        {activeLoader && (
          <span className="muted">
            {gameVersion || "любая версия"} · {activeLoader}
          </span>
        )}
      </div>

      {error && (
        <div className="notice notice-err">
          <AlertCircle size={15} /> {error}
        </div>
      )}

      {loading ? (
        <div className="content-grid">
          {Array.from({ length: 6 }).map((_, index) => (
            <div className="mod-card skeleton" key={index}>
              <div className="skeleton-line short" />
              <div className="skeleton-line" />
              <div className="skeleton-line" />
            </div>
          ))}
        </div>
      ) : hits.length === 0 && !error ? (
        <div className="empty">
          <Package size={30} />
          <b>Ничего не найдено</b>
          <span className="muted">Попробуйте другой запрос или снимите фильтры</span>
        </div>
      ) : (
        <>
          <div className="content-grid">
            {hits.map((hit) => {
              const task = taskFor(hit);
              return (
                <ContentCard
                  key={hit.id}
                  hit={hit}
                  task={task}
                  installed={task?.status === "done"}
                  onInstall={install}
                />
              );
            })}
          </div>
          {hits.length < total && (
            <div className="load-more">
              <button className="btn" type="button" onClick={loadMore} disabled={loadingMore}>
                {loadingMore ? <Loader2 size={15} className="spin" /> : <Download size={15} />}
                Показать ещё {Math.min(PAGE_SIZE, total - hits.length)}
              </button>
            </div>
          )}
        </>
      )}

      {instances.length > 0 && (
        <section className="card instances-card">
          <div className="card-head">
            <span className="card-ic">
              <Boxes size={18} />
            </span>
            <div>
              <h2>Установленные сборки</h2>
              <p>Отдельные инстансы — папка instances/</p>
            </div>
          </div>
          <div className="instances-list">
            {instances.map((instance) => (
              <div className="instance-row" key={instance.slug}>
                <Icon url={instance.iconUrl} title={instance.name} />
                <div className="instance-texts">
                  <b>{instance.name}</b>
                  <span className="muted">
                    {instance.mcVersion || "?"}
                    {instance.loader !== "vanilla" ? ` · ${instance.loader}` : ""} ·{" "}
                    {instance.modCount} модов
                  </span>
                </div>
                <code className="instance-path" title={instance.path}>
                  {instance.path}
                </code>
              </div>
            ))}
          </div>
        </section>
      )}

      <DownloadQueuePanel
        tasks={queue}
        onCancel={cancel}
        onRemove={remove}
        onClear={clear}
      />
    </div>
  );
}
