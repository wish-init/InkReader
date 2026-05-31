# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

InkReader is a local desktop comic repository reader built with Tauri 2, Vue 3, TypeScript, Rust, and SQLite. The app scans a user-selected local comic repository directory, stores repository/book/chapter/page metadata in SQLite, and renders the library and reader UI in Vue.

Expected first-class comic repository shape:

```text
漫画仓库/
  漫画 A/
    元数据.json
    cover.jpg
    第1话/
      0001.jpg
      0002.jpg
```

`元数据.json` is parsed with camelCase fields such as `id`, `name`, `description`, `author`, `tags`, and `chapterInfos[].chapterTitle/order`. Folder-based comics are supported now; archive detection exists, but archive reading is intentionally deferred.

## Commands

Use pnpm for frontend/Tauri scripts and Cargo for Rust backend checks.

```bash
pnpm install                 # install JS dependencies
pnpm check:env               # verify Node.js, pnpm, rustc, and cargo are available
pnpm dev                     # run Vite dev server only on 127.0.0.1:1420
pnpm tauri:dev               # run environment check, then launch the Tauri app
pnpm build                   # run vue-tsc --noEmit, then Vite production build
pnpm typecheck               # TypeScript/Vue type check only
pnpm tauri:build:installer   # build the Windows NSIS installer
pnpm tauri:build:portable    # build release exe, then create a portable folder/zip
pnpm tauri:build             # alias for the installer build
```

Rust commands should point at the Tauri manifest when run from the repo root:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml <test_name>
cargo build --manifest-path src-tauri/Cargo.toml
```

There is currently no frontend lint script and no frontend test runner configured in `package.json`.

## High-level architecture

### Frontend

- Entry point: `src/main.ts` creates the Vue app, installs the router, and imports global styles from `src/styles/base.css` and `src/styles/theme.css`.
- App shell: `src/App.vue` renders `src/components/common/AppShell.vue`, which owns the persistent sidebar and `RouterView`.
- Routing: `src/router/index.ts` uses hash history and defines `/library`, `/favorites`, `/history`, `/repositories`, `/reader/:bookId`, and `/settings`.
- Pages under `src/pages/` contain most page-level state and orchestration:
  - `RepositoryPage.vue` selects directories with `@tauri-apps/plugin-dialog`, scans repositories, rescans, and removes repository records.
  - `LibraryPage.vue` loads books and library-view settings, provides title/author/tag search, tag filtering, book sorting, display settings, and favorite collection management.
  - `FavoritesPage.vue` displays favorite collections and the books in each collection using the same book data model and reusable book sorting.
  - `HistoryPage.vue` displays reading history grouped by day/week/month and links each record back to the reader.
  - `ReaderPage.vue` loads the current book, chapters, pages, reader settings, keyboard navigation, single/double/scroll reading modes, and progress saving.
  - `SettingsPage.vue` edits persisted reader settings.
- Frontend API wrappers live in `src/api/`. `src/api/tauri.ts` defines shared TypeScript types, wraps `invoke`, and converts local file paths with `convertFileSrc`. Feature-specific modules (`repositories.ts`, `library.ts`, `reader.ts`, `settings.ts`) expose typed functions matching Rust command names.
- Use the `@/*` alias for imports from `src/*`.

### Tauri/Rust backend

- `src-tauri/src/main.rs` is the binary entry point and delegates to `inkreader_lib::run()`.
- `src-tauri/src/lib.rs` wires Tauri plugins, creates shared `AppState`, initializes the SQLite database, and registers all Tauri commands in `generate_handler!`.
- Commands live in `src-tauri/src/commands/` and should stay thin: validate/receive frontend arguments, call scanner or database methods, and return serializable models/errors.
- `src-tauri/src/models/` defines the Rust structures serialized to the frontend. Serde uses camelCase for API payloads, so keep TypeScript types in `src/api/tauri.ts` aligned with these models.
- `src-tauri/src/errors.rs` centralizes application errors and conversion into Tauri command results.

### Scanning and metadata

- Repository scanning is in `src-tauri/src/scanner/repository.rs`.
- Scanning treats first-level child directories as candidate comics, reads optional `元数据.json`, recognizes `cover.jpg`, scans chapter subdirectories for supported image files, and sorts chapters/pages naturally.
- Metadata parsing is in `src-tauri/src/metadata/comic_metadata.rs`; it intentionally accepts flexible JSON values for source IDs.
- Image/archive helpers are under `src-tauri/src/scanner/image.rs` and `archive.rs`; archive files are recognized but skipped by the current repository scanner.

### Persistence

- `src-tauri/src/db/mod.rs` owns SQLite setup and data access. The database is created under the executable's app-local `data/` directory as `inkreader.sqlite3`.
- Portable builds store data at `<portable-folder>/data/inkreader.sqlite3`; installer builds store data at `<install-dir>/data/inkreader.sqlite3`. If the user wants data off C:, the app must be installed/extracted to a non-C writable directory.
- On first launch after migrating from the old AppData-based build, `Database::new` copies the legacy AppData database into the app-local `data/` directory if no new database exists, leaving the old file as a backup.
- Tables include `repositories`, `books`, `chapters`, `pages`, `favorite_books` (legacy compatibility), `favorite_collections`, `favorite_collection_books`, `reading_history`, and `settings`.
- `upsert_scan` replaces repository scan records while preserving reading progress by book path.
- Removing a repository deletes only InkReader database records; it must not delete the user's original comic files.
- Reader settings and library view settings are stored as JSON values in the `settings` table.

### Local file access

- Tauri capabilities are configured in `src-tauri/capabilities/default.json` with dialog and filesystem read permissions.
- `src-tauri/tauri.conf.json` enables the Tauri asset protocol with a broad scope so frontend image tags can load local comic images via `convertFileSrc`.
- Keep complex filesystem traversal and path handling in Rust; the frontend should call commands and render returned structured data.

## Working notes

- Release builds hide the Windows console window via the crate attribute in `src-tauri/src/main.rs`; keep debug builds console-friendly.
- Generated directories such as `node_modules/`, `dist/`, and `src-tauri/target/` are not source; avoid searching or editing them unless specifically required.
- The repository currently contains a long Chinese development plan in `PLAN.md`; use it for product intent, but verify current implementation in code before relying on planned features.
- The sample comic repository under `示例漫画结构/` is useful for manual scanning/reader checks.
