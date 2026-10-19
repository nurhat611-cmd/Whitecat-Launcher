// Генерация иконок MyLauncher: PNG (32/128/256) + ICO (256, PNG-сжатие).
// Без зависимостей: только node:zlib и node:fs.
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const iconsDir = join(here, "..", "src-tauri", "icons");
const publicDir = join(here, "..", "public");
mkdirSync(iconsDir, { recursive: true });
mkdirSync(publicDir, { recursive: true });

// ---------- CRC32 и PNG ----------
const crcTable = new Int32Array(256);
for (let n = 0; n < 256; n++) {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  crcTable[n] = c;
}
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function pngChunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}
function encodePng(size, sampleAt) {
  const stride = size * 4 + 1;
  const raw = Buffer.alloc(size * stride);
  for (let y = 0; y < size; y++) {
    raw[y * stride] = 0;
    for (let x = 0; x < size; x++) {
      const [r, g, b, a] = sampleAt(x, y);
      const o = y * stride + 1 + x * 4;
      raw[o] = r;
      raw[o + 1] = g;
      raw[o + 2] = b;
      raw[o + 3] = a;
    }
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // битная глубина
  ihdr[9] = 6; // RGBA
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    pngChunk("IHDR", ihdr),
    pngChunk("IDAT", deflateSync(raw, { level: 9 })),
    pngChunk("IEND", Buffer.alloc(0)),
  ]);
}

// ---------- рисование (координатное пространство 64x64) ----------
const clamp = (v) => Math.max(0, Math.min(255, Math.round(v)));
const lerp = (a, b, t) => a + (b - a) * t;

function inRoundedRect(u, v, x0, y0, x1, y1, r) {
  if (u < x0 || u > x1 || v < y0 || v > y1) return false;
  const cx = Math.min(Math.max(u, x0 + r), x1 - r);
  const cy = Math.min(Math.max(v, y0 + r), y1 - r);
  const dx = u - cx;
  const dy = v - cy;
  const inCore = (u >= x0 + r && u <= x1 - r) || (v >= y0 + r && v <= y1 - r);
  return inCore || dx * dx + dy * dy <= r * r;
}

function inTriangle(u, v, a, b, c) {
  const sign = (p, q, r) => (p[0] - r[0]) * (q[1] - r[1]) - (q[0] - r[0]) * (p[1] - r[1]);
  const d1 = sign([u, v], a, b);
  const d2 = sign([u, v], b, c);
  const d3 = sign([u, v], c, a);
  const hasNeg = d1 < 0 || d2 < 0 || d3 < 0;
  const hasPos = d1 > 0 || d2 > 0 || d3 > 0;
  return !(hasNeg && hasPos);
}

function logoPixel(u, v) {
  if (!inRoundedRect(u, v, 2, 2, 62, 62, 13)) return [0, 0, 0, 0];
  const t = Math.min(1, Math.max(0, (v - 2) / 60));
  let r = lerp(0x2a, 0x13, t);
  let g = lerp(0x2f, 0x15, t);
  let b = lerp(0x3d, 0x1d, t);

  // неоновая синяя кайма
  if (
    inRoundedRect(u, v, 3.6, 3.6, 60.4, 60.4, 12) &&
    !inRoundedRect(u, v, 5.4, 5.4, 58.6, 58.6, 10.6)
  ) {
    r = lerp(r, 0x3e, 0.9);
    g = lerp(g, 0xa6, 0.9);
    b = lerp(b, 0xff, 0.9);
  }

  // Основания ушей залезают внутрь круга головы, поэтому «приклеиваются» к ней.
  const earL = inTriangle(u, v, [5, 5], [28, 20], [14, 31]);
  const earR = inTriangle(u, v, [59, 5], [36, 20], [50, 31]);
  const head = (u - 32) ** 2 + (v - 38.5) ** 2 <= 20.5 ** 2;

  if (earL || earR || head) {
    const fur = lerp(0xf7, 0xdc, Math.min(1, (v - 5) / 48));
    r = fur;
    g = fur;
    b = Math.min(255, fur + 4);
    if (earL && inTriangle(u, v, [9.8, 11.2], [22.5, 19.4], [14.8, 25.5])) {
      r = 0x3e;
      g = 0xa6;
      b = 0xff;
    }
    if (earR && inTriangle(u, v, [54.2, 11.2], [41.5, 19.4], [49.2, 25.5])) {
      r = 0x3e;
      g = 0xa6;
      b = 0xff;
    }
  }

  if (head) {
    const eyeL = (u - 24.5) ** 2 + (v - 36.5) ** 2 <= 3.6 ** 2;
    const eyeR = (u - 39.5) ** 2 + (v - 36.5) ** 2 <= 3.6 ** 2;
    if (eyeL || eyeR) {
      r = 0x2f;
      g = 0x7c;
      b = 0xf6;
    }
    if (
      (u - 25.7) ** 2 + (v - 35.4) ** 2 <= 1.2 ** 2 ||
      (u - 40.7) ** 2 + (v - 35.4) ** 2 <= 1.2 ** 2
    ) {
      r = 0xe8;
      g = 0xf4;
      b = 0xff;
    }
    if ((u - 32) ** 2 + (v - 42.2) ** 2 <= 1.8 ** 2) {
      r = 0x3e;
      g = 0xa6;
      b = 0xff;
    }
    const smileL = (u - 29.6) ** 2 + (v - 44.6) ** 2 <= 2.6 ** 2 && v >= 44.2;
    const smileR = (u - 34.4) ** 2 + (v - 44.6) ** 2 <= 2.6 ** 2 && v >= 44.2;
    if (smileL || smileR) {
      r = 0x8b;
      g = 0x96;
      b = 0xa8;
    }
  }

  return [clamp(r), clamp(g), clamp(b), 255];
}

function makeIcon(size) {
  return encodePng(size, (x, y) => {
    // 2x2 суперсэмплинг для сглаженных краёв
    let r = 0;
    let g = 0;
    let b = 0;
    let a = 0;
    for (const [dx, dy] of [
      [0.25, 0.25],
      [0.75, 0.25],
      [0.25, 0.75],
      [0.75, 0.75],
    ]) {
      const [sr, sg, sb, sa] = logoPixel(((x + dx) / size) * 64, ((y + dy) / size) * 64);
      r += sr * sa;
      g += sg * sa;
      b += sb * sa;
      a += sa;
    }
    if (a === 0) return [0, 0, 0, 0];
    return [clamp(r / a), clamp(g / a), clamp(b / a), clamp(a / 4)];
  });
}

function encodeIco(png256) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // тип: icon
  header.writeUInt16LE(1, 4); // количество изображений
  const dir = Buffer.alloc(16);
  dir[0] = 0; // ширина 256
  dir[1] = 0; // высота 256
  dir.writeUInt16LE(1, 2); // color planes
  dir.writeUInt16LE(32, 4); // бит на пиксель
  dir.writeUInt32LE(png256.length, 8);
  dir.writeUInt32LE(6 + 16, 12); // смещение данных
  return Buffer.concat([header, dir, png256]);
}

const png32 = makeIcon(32);
const png128 = makeIcon(128);
const png256 = makeIcon(256);
writeFileSync(join(iconsDir, "32x32.png"), png32);
writeFileSync(join(iconsDir, "128x128.png"), png128);
writeFileSync(join(iconsDir, "128x128@2x.png"), png256);
writeFileSync(join(iconsDir, "icon.ico"), encodeIco(png256));
writeFileSync(join(publicDir, "favicon.png"), png128);
console.log("Icons generated: src-tauri/icons/*, public/favicon.png");
