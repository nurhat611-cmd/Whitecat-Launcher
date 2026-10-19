import { useCallback, useEffect, useRef, useState } from "react";
import { AlertCircle, Download, FolderOpen, Image as ImageIcon, Trash2, Upload } from "lucide-react";
import type { SkinInfo } from "../types";
import { isTauri, getSkinInfo, saveSkinUrl, saveSkinFile, deleteSkin, openSkinsFolder } from "../lib/api";

interface Props {
  skinModelGlobal: string;
  onModelChange: (m: string) => void;
}

function SkinPreviewCanvas({ dataUrl, model }: { dataUrl: string | null; model: string }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [imgError, setImgError] = useState(false);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    // clear
    ctx.imageSmoothingEnabled = false;
    const w = canvas.width;
    const h = canvas.height;
    ctx.clearRect(0, 0, w, h);

    // background grid
    ctx.fillStyle = "rgba(255,255,255,0.03)";
    ctx.fillRect(0, 0, w, h);
    // subtle vignette
    const grad = ctx.createRadialGradient(w / 2, h * 0.45, 60, w / 2, h * 0.45, 180);
    grad.addColorStop(0, "rgba(62,166,255,0.14)");
    grad.addColorStop(1, "rgba(0,0,0,0)");
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, w, h);

    if (!dataUrl) {
      // placeholder steve
      ctx.fillStyle = "rgba(255,255,255,0.08)";
      ctx.font = "12px Inter, sans-serif";
      ctx.textAlign = "center";
      ctx.fillText("Нет скина", w / 2, h / 2 - 6);
      ctx.fillStyle = "rgba(152,162,179,0.9)";
      ctx.font = "11px Inter, sans-serif";
      ctx.fillText(model === "alex" ? "Модель Alex (3px)" : "Модель Steve (4px)", w / 2, h / 2 + 10);
      // draw placeholder silhouette
      drawPlaceholder(ctx, w, h, model);
      return;
    }

    const img = new Image();
    img.onload = () => {
      setImgError(false);
      drawSkin(ctx, img, w, h, model);
    };
    img.onerror = () => setImgError(true);
    img.src = dataUrl;
  }, [dataUrl, model]);

  // subtle continuous redraw for bobbing effect (requestAnimationFrame not needed for static; we use simple)
  return (
    <div className="skin-preview-wrap">
      <canvas ref={canvasRef} width={260} height={360} className="skin-canvas" />
      {imgError && <div className="skin-error">Не удалось загрузить изображение скина</div>}
      <div className="skin-model-badge">{model === "alex" ? "Alex · 3px руки" : "Steve · 4px руки"}</div>
    </div>
  );
}

function drawPlaceholder(ctx: CanvasRenderingContext2D, w: number, h: number, model: string) {
  const cx = w / 2;
  const top = 56;
  const scale = 6;
  ctx.fillStyle = "#8e9aaf";
  // head
  ctx.fillRect(cx - 4 * scale, top, 8 * scale, 8 * scale);
  // body
  ctx.fillStyle = "#3ea6ff";
  ctx.fillRect(cx - 4 * scale, top + 8 * scale, 8 * scale, 12 * scale);
  // arms
  const armW = model === "alex" ? 3 * scale : 4 * scale;
  ctx.fillStyle = "#f4a7a7";
  ctx.fillRect(cx - 4 * scale - armW, top + 8 * scale, armW, 12 * scale);
  ctx.fillRect(cx + 4 * scale, top + 8 * scale, armW, 12 * scale);
  // legs
  ctx.fillStyle = "#2d5a9e";
  ctx.fillRect(cx - 4 * scale, top + 20 * scale, 4 * scale, 12 * scale);
  ctx.fillRect(cx, top + 20 * scale, 4 * scale, 12 * scale);
  // shadow
  ctx.fillStyle = "rgba(0,0,0,0.22)";
  ctx.beginPath();
  ctx.ellipse(cx, top + 34 * scale, 28, 6, 0, 0, Math.PI * 2);
  ctx.fill();
}

function drawSkin(ctx: CanvasRenderingContext2D, img: HTMLImageElement, w: number, h: number, model: string) {
  const cx = w / 2;
  const isLegacy = img.height === 32; // old 64x32 format
  const scale = 7;
  const top = 44;

  // helper to draw region with fallback for legacy
  const drawPart = (
    sx: number,
    sy: number,
    sw: number,
    sh: number,
    dx: number,
    dy: number,
    dw: number,
    dh: number,
  ) => {
    // if legacy and requesting overlay beyond height, skip
    if (isLegacy && sy + sh > 32) return;
    ctx.drawImage(img, sx, sy, sw, sh, dx, dy, dw, dh);
  };

  // shadow
  ctx.fillStyle = "rgba(0,0,0,0.28)";
  ctx.beginPath();
  ctx.ellipse(cx, top + 33 * scale + 14, 30, 7, 0, 0, Math.PI * 2);
  ctx.fill();

  // HEAD: base + overlay
  // head front 8x8 at (8,8)
  const headX = cx - 4 * scale;
  const headY = top;
  const headW = 8 * scale;
  const headH = 8 * scale;
  drawPart(8, 8, 8, 8, headX, headY, headW, headH);
  if (!isLegacy) drawPart(40, 8, 8, 8, headX, headY, headW, headH);

  // HEAD side shading for 3D feel
  ctx.fillStyle = "rgba(0,0,0,0.12)";
  ctx.fillRect(headX + headW - 6, headY, 6, headH);

  // BODY front 8x12 at (20,20)
  const bodyX = cx - 4 * scale;
  const bodyY = headY + headH;
  const bodyW = 8 * scale;
  const bodyH = 12 * scale;
  drawPart(20, 20, 8, 12, bodyX, bodyY, bodyW, bodyH);
  if (!isLegacy) drawPart(20, 36, 8, 12, bodyX, bodyY, bodyW, bodyH);

  // ARMS: Steve 4x12 at (44,20) right arm front, Alex 3x12
  const armWSteve = 4 * scale;
  const armWAlex = 3 * scale;
  const armW = model === "alex" ? armWAlex : armWSteve;
  const armH = 12 * scale;

  // left arm (our right) - for symmetry use right arm texture for both sides (mirror not needed for preview)
  // right arm front
  const leftArmX = bodyX - armW;
  const leftArmY = bodyY;
  // left arm texture source differs for Alex: x 36 vs 44?
  const leftSx = model === "alex" ? 36 : 44;
  drawPart(leftSx, 20, model === "alex" ? 3 : 4, 12, leftArmX, leftArmY, armW, armH);
  if (!isLegacy) drawPart(leftSx, 36, model === "alex" ? 3 : 4, 12, leftArmX, leftArmY, armW, armH);

  const rightArmX = bodyX + bodyW;
  const rightArmY = bodyY;
  // right arm second layer is at 52,52 etc but for preview we reuse same
  drawPart(44, 20, model === "alex" ? 3 : 4, 12, rightArmX, rightArmY, armW, armH);
  if (!isLegacy) drawPart(52, 52 - (model === "alex" ? 0 : 0), model === "alex" ? 3 : 4, 12, rightArmX, rightArmY, armW, armH);

  // LEGS: left leg 4x12 at (4,20), right leg reuse
  const legW = 4 * scale;
  const legH = 12 * scale;
  const leftLegX = bodyX;
  const leftLegY = bodyY + bodyH;
  drawPart(4, 20, 4, 12, leftLegX, leftLegY, legW, legH);
  if (!isLegacy) drawPart(4, 36, 4, 12, leftLegX, leftLegY, legW, legH);

  const rightLegX = bodyX + 4 * scale;
  const rightLegY = leftLegY;
  drawPart(20 - 16, 20, 4, 12, rightLegX, rightLegY, legW, legH);
  if (!isLegacy) drawPart(4 + 8, 36, 4, 12, rightLegX, rightLegY, legW, legH);

  // outline glow for head
  ctx.strokeStyle = "rgba(62,166,255,0.18)";
  ctx.lineWidth = 1;
  ctx.strokeRect(headX - 0.5, headY - 0.5, headW + 1, headH + 1);
}

export function SkinsTab({ skinModelGlobal, onModelChange }: Props) {
  const [skin, setSkin] = useState<SkinInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [model, setModel] = useState<string>(skinModelGlobal || "steve");
  const [url, setUrl] = useState("");
  const [fileName, setFileName] = useState<string | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const refresh = useCallback(async () => {
    if (!isTauri()) {
      setLoading(false);
      return;
    }
    try {
      const info = await getSkinInfo();
      setSkin(info);
      setModel(info.model || "steve");
      if (info.previewBase64) {
        setPreviewUrl(`data:image/png;base64,${info.previewBase64}`);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    setModel(skinModelGlobal);
  }, [skinModelGlobal]);

  const handleModel = (m: string) => {
    setModel(m);
    onModelChange(m);
  };

  const onUrlSave = async () => {
    if (!url.trim()) {
      setError("Введите URL скина");
      return;
    }
    setSaving(true);
    setError(null);
    setStatus(null);
    try {
      const res = await saveSkinUrl(url.trim(), model);
      setSkin(res);
      if (res.previewBase64) setPreviewUrl(`data:image/png;base64,${res.previewBase64}`);
      setStatus("Скин сохранён по URL");
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const onFilePick = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    setFileName(file.name);
    setError(null);
    const reader = new FileReader();
    reader.onload = async () => {
      const result = reader.result as string;
      // show preview immediately
      setPreviewUrl(result);
      // auto-save
      if (!isTauri()) {
        setStatus("В браузере предпросмотр только — сохранение доступно в приложении");
        return;
      }
      // extract base64
      const b64 = result.includes(",") ? result.split(",")[1] : result;
      setSaving(true);
      try {
        const res = await saveSkinFile(b64, file.name, model);
        setSkin(res);
        if (res.previewBase64) setPreviewUrl(`data:image/png;base64,${res.previewBase64}`);
        setStatus(`Скин загружен из файла: ${file.name}`);
      } catch (err) {
        setError(String(err));
      } finally {
        setSaving(false);
      }
    };
    reader.readAsDataURL(file);
  };

  const onDelete = async () => {
    if (!isTauri()) return;
    setSaving(true);
    try {
      const res = await deleteSkin();
      setSkin(res);
      setPreviewUrl(null);
      setFileName(null);
      setStatus("Скин удалён");
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const onOpenFolder = async () => {
    try {
      const p = await openSkinsFolder();
      setStatus(`Папка скинов: ${p}`);
    } catch (e) {
      setError(String(e));
    }
  };

  if (loading) {
    return (
      <div className="tab-head">
        <h1>Скины</h1>
        <p>Загрузка…</p>
      </div>
    );
  }

  return (
    <div>
      <div className="tab-head">
        <h1>Скины</h1>
        <p>Загрузите PNG-скин по URL или из файла · превью Steve / Alex · сохранение в папку скинов</p>
      </div>

      <div className="skins-layout">
        {/* Preview */}
        <div className="card skin-preview-card">
          <div className="card-head">
            <span className="card-ic">
              <ImageIcon size={18} />
            </span>
            <div>
              <h2>Превью</h2>
              <p>{model === "alex" ? "Alex — узкие руки 3px" : "Steve — обычные руки 4px"}</p>
            </div>
          </div>

          <SkinPreviewCanvas dataUrl={previewUrl} model={model} />

          <div className="segmented" style={{ marginTop: 12 }}>
            <button className={`seg ${model === "steve" ? "active" : ""}`} onClick={() => handleModel("steve")} type="button">
              Steve
            </button>
            <button className={`seg ${model === "alex" ? "active" : ""}`} onClick={() => handleModel("alex")} type="button">
              Alex
            </button>
          </div>
          <span className="muted" style={{ fontSize: 11.5, marginTop: 6, display: "block", textAlign: "center" }}>
            Переключение модели меняет ширину рук в превью и сохраняется
          </span>

          {skin?.exists && skin.path && (
            <div className="sys-row">
              <span className="sys-label">Файл</span>
              <span className="sys-value" title={skin.path}>
                {skin.path}
              </span>
            </div>
          )}
        </div>

        {/* Controls */}
        <div className="skins-controls">
          <section className="card">
            <div className="card-head">
              <span className="card-ic">
                <Download size={18} />
              </span>
              <div>
                <h2>Загрузка по URL</h2>
                <p>Вставьте прямую ссылку на PNG (NameMC, SkinsMC и т.п.)</p>
              </div>
            </div>
            <div className="field">
              <span className="field-label">URL скина</span>
              <div className="inline-2">
                <input
                  className="input"
                  type="url"
                  placeholder="https://example.com/skin.png"
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                />
                <button className="btn btn-accent" type="button" onClick={onUrlSave} disabled={saving || !url.trim()}>
                  <Download size={15} />
                  {saving ? "Сохранение…" : "Загрузить"}
                </button>
              </div>
              <span className="field-hint muted">Поддерживаются 64×64 и 64×32 PNG. До 10 МБ.</span>
            </div>
          </section>

          <section className="card">
            <div className="card-head">
              <span className="card-ic">
                <Upload size={18} />
              </span>
              <div>
                <h2>Загрузка из файла</h2>
                <p>Выберите PNG на компьютере</p>
              </div>
            </div>
            <input ref={fileRef} type="file" accept=".png,image/png" style={{ display: "none" }} onChange={onFilePick} />
            <button className="btn" type="button" onClick={() => fileRef.current?.click()} disabled={saving} style={{ width: "100%", justifyContent: "center" }}>
              <Upload size={15} /> {fileName ? fileName : "Выбрать файл…"}
            </button>
            {fileName && <span className="muted" style={{ fontSize: 12 }}>Выбран: {fileName}</span>}
          </section>

          <section className="card">
            <div className="card-head">
              <span className="card-ic">
                <FolderOpen size={18} />
              </span>
              <div>
                <h2>Управление</h2>
                <p>Папка скинов и действия</p>
              </div>
            </div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <button className="btn" type="button" onClick={onOpenFolder}>
                <FolderOpen size={15} /> Открыть папку
              </button>
              <button className="btn" type="button" onClick={onDelete} disabled={!skin?.exists || saving}>
                <Trash2 size={15} /> Удалить скин
              </button>
            </div>
            <div className="notice" style={{ marginBottom: 0 }}>
              <AlertCircle size={15} />
              <span>
                <b>Офлайн-режим:</b> применить скин напрямую нельзя — Mojang требует аккаунт. Установите мод{" "}
                <code>CustomSkinLoader</code> или <code>SkinRestorer</code> и поместите скины в <code>skins/</code>.
                Лаунчер сохраняет файл как <code>skins/skin.png</code> ({model}).
              </span>
            </div>
            <div className="hint" style={{ fontSize: 12.5, lineHeight: 1.5 }}>
              <b>Инструкция для офлайна:</b>
              <ol style={{ margin: "6px 0 0 18px", display: "flex", flexDirection: "column", gap: 3 }}>
                <li>Установите Fabric и мод CustomSkinLoader.</li>
                <li>Скопируйте сохранённый <code>skin.png</code> в профиль мода или укажите путь в его конфиге.</li>
                <li>Либо загрузите скин на локальный сервер скинов (skin-server) и укажите URL в настройках мода.</li>
              </ol>
            </div>
          </section>

          {status && (
            <div className="notice">
              <ImageIcon size={15} /> {status}
            </div>
          )}
          {error && (
            <div className="notice notice-err">
              <AlertCircle size={15} /> {error}
            </div>
          )}
          {!isTauri() && (
            <div className="notice">
              Запустите <code>npm run tauri dev</code> для сохранения скина на диск — в браузере доступен только предпросмотр.
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
