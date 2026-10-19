import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri dev-порт зафиксирован в src-tauri/tauri.conf.json (devUrl).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: true,
    // Превью-хост песочницы динамический (*.{sandbox}.e2b.app) — разрешаем все.
    allowedHosts: true,
  },
  build: {
    target: "es2021",
    outDir: "dist",
  },
});
