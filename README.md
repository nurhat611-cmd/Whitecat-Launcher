# Whitecat-Launcher — MyLauncher

Офлайн-лаунчер Minecraft на **Tauri 2 + React 18 + TypeScript + Rust** (бэкенд `lighty-launcher`).

## Часть 1/4 — что уже работает

**Интерфейс** (тёмная тема, glassmorphism, неоново-синий акцент):
- Splash-экран: логотип + «MyLauncher», появление/растворение на Framer Motion, 2 секунды.
- Левое меню: вкладки «Главная», «Контент», «Настройки» (иконки Lucide, подсветка активной).
- Карточка профиля: ник, список версий (заглушка), загрузчики Vanilla/Fabric/Forge, слайдер RAM 1–16 ГБ.
- Кнопка «ИГРАТЬ» с эффектом свечения и статус-бар («Готов к запуску» и т.д.).
- Плавные анимации переключения вкладок (AnimatePresence), hover-эффекты.

**Бэкенд (Rust):**
- Офлайн-запуск без аккаунта Mojang: legacy-токен, `auth_session`, offline-UUID (MD5 «OfflinePlayer:<ник>»).
- Полный пайплайн: манифест Mojang → JSON версии → клиент → библиотеки (включая нативки, старые и новые форматы) → ассеты (форматы `objects` и `map`) → сборка командной строки по правилам манифеста.
- Vanilla — полностью, Fabric — через официальный loader-профиль (meta.fabricmc.net), Forge — с 2-й части.
- Автоопределение Java: встроенный JDK → `JAVA_HOME` → `PATH` → типовые каталоги (Adoptium, Microsoft, BellSoft, Corretto, Zulu, `/usr/lib/jvm`, JVM на macOS).
- Скачивание JDK 21 (Adoptium API) с прогрессом (событие `jdk_progress`), автоустановка в папку лаунчера (zip / tar.gz).
- Настройки сохраняются в `settings.json` (app config dir); лог игры — `launcher.log` в папке игры.

## Запуск

```bash
npm install
npm run tauri dev
```

## Сборка дистрибутива

```bash
npm run tauri build
```

## Структура

```
src/                    # React + TS (фронтенд)
  components/           # Splash, Sidebar, вкладки
  lib/api.ts            # obёртки invoke-команд
src-tauri/
  src/lib.rs            # Tauri-команды, общие типы
  src/java.rs           # автоопределение Java
  src/jdk.rs            # скачивание/установка JDK
  src/minecraft.rs      # офлайн-запуск, пайплайн установки
  src/settings.rs       # сохранение настроек
scripts/make-icons.mjs  # генерация иконок (npm run icons)
```
