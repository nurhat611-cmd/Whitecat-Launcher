import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const version = process.env.APP_VERSION?.trim();
const assetUrl = process.env.UPDATER_ASSET_URL?.trim();
const signaturePath = process.env.UPDATER_SIGNATURE_PATH;
const manifestPath =
  process.env.UPDATER_MANIFEST_PATH ?? "release-assets/latest.json";

if (!version || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error("APP_VERSION должен содержать корректную версию SemVer.");
}
if (!assetUrl) throw new Error("Не задан UPDATER_ASSET_URL.");
if (!signaturePath) throw new Error("Не задан UPDATER_SIGNATURE_PATH.");

const parsedUrl = new URL(assetUrl);
if (parsedUrl.protocol !== "https:" || parsedUrl.hostname !== "github.com") {
  throw new Error(
    "UPDATER_ASSET_URL должен быть HTTPS-ссылкой на GitHub Releases.",
  );
}

const signature = (await readFile(signaturePath, "utf8")).trim();
if (!signature) throw new Error(`Файл подписи пуст: ${signaturePath}`);

const manifest = {
  version,
  notes: `Обновление MyLauncher ${version}`,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: assetUrl,
    },
  },
};

await mkdir(path.dirname(manifestPath), { recursive: true });
await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
console.log(`Создан ${manifestPath} для MyLauncher ${version}.`);
