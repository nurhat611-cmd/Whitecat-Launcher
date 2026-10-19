import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const configPath = fileURLToPath(
  new URL("../src-tauri/tauri.conf.json", import.meta.url),
);
const publicKey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim();

if (!publicKey) {
  throw new Error(
    "Не задан TAURI_UPDATER_PUBLIC_KEY. Установите одноимённую переменную GitHub Actions с публичным ключом Tauri signer.",
  );
}

const config = JSON.parse(await readFile(configPath, "utf8"));
config.plugins ??= {};
config.plugins.updater ??= {};
config.plugins.updater.pubkey = publicKey;
await writeFile(configPath, `${JSON.stringify(config, null, 2)}\n`, "utf8");
console.log(
  `Публичный ключ автообновления подключён из ${path.basename(configPath)}.`,
);
