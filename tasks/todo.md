# Bex LaTeX Studio — Implementation Checklist

## Step 1: Project Foundation & Dependencies
- [x] Initialize `tasks/todo.md`
- [x] Scaffold Svelte 5 + TypeScript + Vite config (`package.json`, `vite.config.ts`, `tsconfig.json`, `svelte.config.js`, `index.html`, `src/main.ts`)
- [x] Scaffold Tauri v2 configuration (`src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, app icons)
- [x] Install npm dependencies and verify package compatibility

## Step 2: Parallel Implementation via Subagents
- [x] **Subagent A (Rust Backend)**:
  - [x] Workspace management & file CRUD commands (`src-tauri/src/workspace.rs`)
  - [x] Multi-engine LaTeX compiler (`tectonic` Rust engine primary + `pdflatex`/`xelatex`/`lualatex` fallback) & log parser (`src-tauri/src/compiler.rs`)
  - [x] Tauri v2 command registration & app entry (`src-tauri/src/lib.rs`, `src-tauri/src/main.rs`)
  - [x] Rust unit tests for workspace tree, log parsing, and Tectonic compilation
- [x] **Subagent B (Editor & PDF Preview)**:
  - [x] Shared IPC TypeScript interfaces (`src/lib/types.ts`) and Tauri API bridge (`src/lib/api.ts`)
  - [x] CodeMirror 6 LaTeX editor with `Mod-s` save-and-compile, formatting toolbar, autocomplete, error decorations (`src/lib/components/EditorPane.svelte`)
  - [x] PDF.js multi-page HiDPI canvas viewer with zoom, scroll preservation, and clickable diagnostics drawer (`src/lib/components/PdfPreview.svelte`)
- [x] **Subagent C (Overleaf Layout, Menu, Sidebar & Theme)**:
  - [x] Modern CSS design tokens, `color-scheme: light dark`, `dialog` backdrop/overscroll rules (`src/app.css`)
  - [x] Top menu bar with project actions, compiler selector, auto-compile on save toggle, theme switch (`src/lib/components/TopBar.svelte`)
  - [x] Collapsible file & folder tree sidebar (`src/lib/components/Sidebar.svelte`)
  - [x] Native `<dialog>` modal component (`src/lib/components/ModalDialog.svelte`)
  - [x] Root 3-pane Overleaf layout & state wiring (`src/App.svelte`)

## Step 3: Integration & Verification
- [x] Verify IPC types and command signatures match end-to-end
- [x] Guard `Cmd/Ctrl+S` keydown propagation to prevent duplicate save/compile triggers
- [x] Run TypeScript/Svelte checks (`npm run check`) and Vite production build (`npm run build`)
- [x] Run `cargo check --all-targets` and `cargo test -- --nocapture` in `src-tauri`
- [x] Verify live save-triggers-compile flow with Tectonic
- [x] Document review and results in `tasks/todo.md`

---

## Review & Verification Summary

- **LaTeX Compilation Architecture (`src-tauri/src/compiler.rs`, `src-tauri/Cargo.toml`, `src-tauri/.cargo/config.toml`)**:
  - Embeds **`tectonic` (`0.17`)** directly in-process inside the Rust backend via `ProcessingSessionBuilder` on `tokio::task::spawn_blocking` serialized with `EMBEDDED_TECTONIC_LOCK` and backed by `BufferStatusBackend` (`StatusBackend`).
  - Automatically configures Homebrew `icu4c` pkg-config search paths in `src-tauri/.cargo/config.toml` and retains optional fallback support for external `pdflatex`, `xelatex`, and `lualatex` binaries.
  - Compiles into an isolated `.bex-build` folder inside the workspace, base64-encodes the generated PDF for fast IPC transfer, and parses TeX/Tectonic logs into structured `LatexDiagnostic` items.
- **Beck's-Inspired Beer Bottle Branding (`src-tauri/icons/*`, `public/bex-bottle.svg`, `index.html`, `src/lib/components/TopBar.svelte`)**:
  - Generated cross-platform desktop icons (`32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico`, etc.) plus SVG favicon and TopBar inline logo depicting an emerald beer bottle with silver foil neck, red collar, and red-bordered white oval crest label.
- **Verified Checks (after final edits)**:
  - `npm run check`: `svelte-check` reported **0 errors and 0 warnings**, `tsc --noEmit` exited `0`.
  - `npm run build`: `vite build` succeeded (`dist/index.html`, `dist/bex-bottle.svg`, `dist/assets/index-BNHaxTCn.css`, `dist/assets/index-BtgclU2M.js`, `dist/assets/pdf.worker.min-iDqQPrd3.mjs`).
  - `cd src-tauri && cargo check --all-targets`: succeeded with 0 errors.
  - `cd src-tauri && cargo test -- --nocapture`: **8 passed, 0 failed** (including embedded Tectonic compilation of the seeded multi-file project, `BufferStatusBackend` log capture, and line-3 diagnostic reporting).

---

## Step 4: GitHub Releases & Tauri v2 Auto-Updater
- [x] Generate Ed25519 signing keypair (`~/.tauri/bex-latex-studio.key` + `.pub`) for Tauri v2 updater signatures
- [x] Install `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process` npm packages
- [x] **Subagent 1 (Backend Updater, Tauri Config & GitHub Actions Release Pipeline)**:
  - [x] Add `tauri-plugin-process` and desktop `tauri-plugin-updater` + `[package.metadata.vcpkg]` in `src-tauri/Cargo.toml`
  - [x] Register updater & process plugins in `src-tauri/src/lib.rs`
  - [x] Enable `createUpdaterArtifacts: true` and configure `plugins.updater` in `src-tauri/tauri.conf.json`
  - [x] Add `updater:default` and `process:default` permissions in `src-tauri/capabilities/default.json`
  - [x] Create cross-platform GitHub Release workflow (`.github/workflows/release.yml`) for macOS, Linux, and Windows
  - [x] Document GitHub Releases & auto-update setup in `README.md`
- [x] **Subagent 2 (Frontend Updater API, UpdateModal & TopBar Integration)**:
  - [x] Extend `src/lib/types.ts` with `AppUpdateInfo` and `UpdateProgress`
  - [x] Implement updater helpers (`checkForAppUpdate`, `downloadAndInstallAppUpdate`, `relaunchApp`) in `src/lib/api.ts`
  - [x] Create `src/lib/components/UpdateModal.svelte` native `<dialog>` updater modal
  - [x] Add "Check for Updates..." and update availability badge in `src/lib/components/TopBar.svelte`
  - [x] Wire startup background check, manual update check, and `UpdateModal` in `src/App.svelte`
- [x] Verify `npm run check`, `npm run build`, `cargo check --all-targets`, and `cargo test -- --nocapture` and document results

### GitHub Releases & Updater Verification Summary
- **Updater Plugins & Config (`src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`)**:
  - Integrated `tauri-plugin-updater` (`2.14.1`) and `tauri-plugin-process` (`2.4.0`) with `@tauri-apps/plugin-updater` (`^2.14.1`) and `@tauri-apps/plugin-process` (`^2.4.0`).
  - Enabled `bundle.createUpdaterArtifacts: true` and `plugins.updater` with Ed25519 minisign `pubkey` (`~/.tauri/bex-latex-studio.key.pub`), GitHub Releases endpoint (`https://github.com/Vinello28/bex/releases/latest/download/latest.json`), and `"windows": { "installMode": "passive" }`.
- **Cross-Platform CI & Docs (`.github/workflows/release.yml`, `README.md`)**:
  - Added automated GitHub Release workflow for `macos-latest` (`--target aarch64-apple-darwin`), `ubuntu-22.04`, and `windows-latest` (`vcpkg` triplet `x64-windows-static-md`) using `tauri-apps/tauri-action@v0` to produce installers, `.sig` files, and `latest.json`.
- **Verified Commands**:
  - `npm run check`: `svelte-check` **0 errors and 0 warnings**, `tsc --noEmit` exited `0`.
  - `npm run build`: `vite build` succeeded (`dist/index.html`, `dist/assets/index-CD_X4XxE.js`, `dist/assets/index-De5mRQRQ.css`, `dist/assets/pdf.worker.min-iDqQPrd3.mjs`).
  - `cargo check --all-targets` & `cargo test -- --nocapture`: exited `0`, **8 passed; 0 failed** in `bex_latex_studio_lib`.

---

## Step 5: Rebranding (Homebrew-Style Beer Icon with Big Eyes), True Black/Neutral Theme & `Vinello28/bex` Repository
- [x] Record UI palette & icon design feedback in `tasks/lessons.md`
- [x] Configure `Vinello28/bex` repository metadata & updater endpoint (`src-tauri/tauri.conf.json`, `package.json`, `src-tauri/Cargo.toml`, `README.md`)
- [x] **Subagent 1 (Mascot & Brand Icon Designer)**:
  - [x] Redesign `public/bex-bottle.svg` as a large, chubby, Homebrew-inspired golden-amber beer mug/bottle with creamy overflowing foam and big expressive cartoon eyes ("occhioni") — zero green
  - [x] Update inline header SVG and bump `.brand-icon` to `28x28` in `src/lib/components/TopBar.svelte`
  - [x] Regenerate all cross-platform Tauri icons (`src-tauri/icons/*`) via `npx tauri icon public/bex-bottle.svg`
- [x] **Subagent 2 (True Black / Neutral Theme Engineer)**:
  - [x] Replace dark/light palettes in `src/app.css` with true black/pure neutral grayscale surfaces and warm amber accents (zero bluish slate)
  - [x] Purge hardcoded slate/blue fallbacks and update syntax/editor chrome in `src/lib/components/EditorPane.svelte` and `src/lib/components/PdfPreview.svelte`
  - [x] Audit remaining components (`Sidebar.svelte`, `ModalDialog.svelte`, `UpdateModal.svelte`, `TopBar.svelte`) for neutral contrast & `in oklab` color mixing
- [x] Run `npm run check`, `npm run build`, `cargo check --all-targets`, `cargo test -- --nocapture`, and palette verification checks

### Step 5 Verification Summary
- **Repository Configuration**: Replaced placeholder owner with **`Vinello28/bex`** across `src-tauri/tauri.conf.json` (`endpoints`), `package.json` (`repository`), `src-tauri/Cargo.toml` (`repository`), and `README.md`; verified zero remaining `gabrielevianello` occurrences outside `node_modules`/`target`/`dist`.
- **Homebrew-Style Beer Mascot Icon**: Replaced green bottle SVG with a large golden-amber frothy beer bottle with creamy foam and big expressive cartoon eyes ("occhioni") in `public/bex-bottle.svg` and `src/lib/components/TopBar.svelte` (`28x28`), and regenerated `src-tauri/icons/*` via `npx tauri icon public/bex-bottle.svg`.
- **True Black Dark Mode & Neutral Light Mode**: Updated `src/app.css` (`#000000`/`#0a0a0a`/`#121212` dark, `#ffffff`/`#fafafa`/`#ececec` light, warm amber accents `#f59e0b`/`#b45309`) along with `EditorPane.svelte`, `PdfPreview.svelte`, `Sidebar.svelte`, `ModalDialog.svelte`, `UpdateModal.svelte`, and `TopBar.svelte`.
- **Verified Checks (after final code edits)**:
  - Stale `gabrielevianello` and Slate/bluish hex + `color-mix(in srgb` audits returned **0 matches**.
  - `npm run check`: **0 errors and 0 warnings**.
  - `npm run build`: `vite build` succeeded (`dist/index.html`, `dist/assets/index-CbEUi1fZ.css`, `dist/assets/index-CbTYhP9R.js`, `dist/assets/pdf.worker.min-iDqQPrd3.mjs`).
  - `cargo check --all-targets` & `cargo test -- --nocapture`: exited `0`, **8 passed; 0 failed** in `bex_latex_studio_lib`.

---

## Step 6: Declutter Interface & Unified Settings Dialog
- [x] Move `ThemeMode` into `src/lib/types.ts`
- [x] Create `src/lib/components/SettingsDialog.svelte` unifying compilation settings, theme selector, software updates, and app info
- [x] Declutter `src/lib/components/TopBar.svelte` and add a gear Settings button (with subtle update indicator dot)
- [x] Wire `SettingsDialog.svelte`, preference persistence (`localStorage`), and `Cmd/Ctrl+,` shortcut in `src/App.svelte`
- [x] Remove obsolete `src/lib/components/UpdateModal.svelte`
- [x] Verify palette/code audits, `npm run check`, and `npm run build`, and document results

### Step 6 Verification Summary
- **Unified Settings Dialog (`src/lib/components/SettingsDialog.svelte`, `src/lib/types.ts`)**:
  - Consolidated Main `.tex` file selector, LaTeX compiler engine selector (with detected engine status pills), `Auto-compile on Save` toggle, 3-state theme selector (`System` with live OS state, `Light`, `Dark`), the full GitHub Releases auto-updater flow (`idle`, `checking`, `available`, `downloading`, `ready-to-restart`, `up-to-date`, `error`), and app metadata (`About Bex`) into a single native `<dialog>` modal.
- **Decluttered Header (`src/lib/components/TopBar.svelte`, `src/App.svelte`)**:
  - Removed inline entry file, compiler engine, auto-compile, theme cycle, and update pill controls from the top bar and removed `Check for Updates...` from the `File` menu, replacing them with a single gear Settings button (`Cmd/Ctrl+,`) displaying a subtle update dot when an update is available.
  - Added `localStorage` persistence for `selectedEngine` (`bex-compiler-engine`) and `autoCompileOnSave` (`bex-auto-compile`) alongside `bex-theme`, and deleted `src/lib/components/UpdateModal.svelte`.
- **Verified Checks (after final code edits)**:
  - Grep audit across `src/` for `UpdateModal`, Slate hex colors (`#0f141c`, `#161c26`, `#0f172a`, `#1e293b`, `#f1f5f9`), and `color-mix(in srgb`: **0 matches**.
  - `npm run check`: `svelte-check` reported **0 errors and 0 warnings**, `tsc --noEmit` exited `0`.
  - `npm run build`: `vite build` succeeded (`dist/index.html`, `dist/assets/index-rRhYJIyW.css`, `dist/assets/index-BF6c1hoz.js`, `dist/assets/pdf.worker.min-iDqQPrd3.mjs`).

---

## Step 7: JetBrains Islands Home Screen, Theme & MCP AI Agent
- [x] **Phase 0 (Coordinator Setup & Shared Contracts)**:
  - [x] Initialize `/swarm` with lightweight workers and write `plan.json`
  - [x] Add shared Home, Recent Projects/Files, AI & MCP types in `src/lib/types.ts`
  - [x] Add `reqwest` (`rustls-tls`, `json`) to `src-tauri/Cargo.toml`
  - [x] Add synchronous inline theme FOUC-prevention script in `index.html`
- [x] **Worker 1 (`rust-mcp-ai`)**:
  - [x] Expose `pub(crate) fn write_utf8_atomic` and add `create_new_project` in `src-tauri/src/workspace.rs`
  - [x] Implement MCP JSON-RPC 2.0 server (`src-tauri/src/mcp.rs`) with 8 LaTeX workspace tools, path traversal guards, and unit tests
  - [x] Implement multi-provider AI agent (`src-tauri/src/ai_agent.rs`) supporting Google AI Studio (`.env` fallback), Ollama, LM Studio, and vLLM + unit tests
  - [x] Register new modules & commands in `src-tauri/src/lib.rs` and verify `cargo check` / `cargo test`
- [x] **Worker 2 (`theme-islands`)**:
  - [x] Refresh `src/app.css` with neutral JetBrains Islands Dark & Light palettes and island geometry tokens
  - [x] Style `TopBar.svelte`, `Sidebar.svelte`, `EditorPane.svelte`, `PdfPreview.svelte`, and `ModalDialog.svelte` as floating islands
- [x] **Worker 3 (`home-screen`)**:
  - [x] Create `src/lib/recent.ts` for recent projects/files persistence and formatting helpers
  - [x] Create `src/lib/components/HomeView.svelte` JetBrains-style Welcome screen
- [x] **Worker 4 (`ai-sidebar-settings`)**:
  - [x] Create `src/lib/components/AiSidebar.svelte` collapsible AI assistant sidebar with MCP tool traces and file attachments
  - [x] Add AI Agent & MCP configuration + section navigation to `src/lib/components/SettingsDialog.svelte`
- [x] **Phase 2 & 3 (Coordinator Integration & Verification)**:
  - [x] Add new IPC wrappers & browser mocks in `src/lib/api.ts`
  - [x] Wire Home startup, recent items, Islands layout, AI Sidebar, and MCP sync in `src/App.svelte`
  - [x] Terminate finished swarm workers and run `npm run check`, `npm run build`, `cargo check --all-targets`, `cargo test`, and security/palette audits

### Step 7 Verification Summary
- **JetBrains Islands Theme & Home Screen (`src/app.css`, `src/lib/recent.ts`, `src/lib/components/HomeView.svelte`, `src/App.svelte`, `index.html`)**:
  - App starts on `HomeView.svelte` showing Recent Projects, Recent Files, search filter, New Project / Open Directory / Sample Project actions, Customize tab, Software Updates status, and Settings (`Cmd/Ctrl+,`).
  - Updated theme tokens to neutral JetBrains Islands Dark (`#101011` canvas, `#1a1a1c` islands) and Islands Light (`#e6e6e7` canvas, `#ffffff` islands) with `--radius-island: 10px` and `--island-gap: 7px`.
- **MCP JSON-RPC 2.0 Server & Multi-Provider AI Sidebar (`src-tauri/src/mcp.rs`, `src-tauri/src/ai_agent.rs`, `src/lib/components/AiSidebar.svelte`, `src/lib/components/SettingsDialog.svelte`)**:
  - Implemented 8 workspace-scoped MCP tools (`get_editor_context`, `list_project_files`, `read_file`, `write_file`, `insert_text`, `replace_in_file`, `create_file`, `compile_project`) with strict canonical path-traversal guards and live CodeMirror/PDF sync.
  - Added multi-provider AI support (Google AI Studio with `GOOGLE_AI_STUDIO` `.env` fallback when user key is empty, Ollama, LM Studio, and vLLM/OpenAI-compatible endpoints) configurable in Settings.
- **Verified Checks (after final code edits)**:
  - `npm run check`: `svelte-check` **0 errors and 0 warnings**, `tsc --noEmit` exited `0`.
  - `npm run build`: `vite build` succeeded (`dist/index.html`, `dist/assets/index-DqKb8LzT.css`, `dist/assets/index-jVVgmv1r.js`, `dist/assets/pdf.worker.min-iDqQPrd3.mjs`).
  - `cd src-tauri && cargo check --all-targets && cargo test -- --nocapture`: **16 passed; 0 failed**.
  - Secret leak & palette audits: `SECRET_AUDIT_CLEAN` (zero `.env` key occurrences in `src/`, `src-tauri/src/`, `dist/`) and zero bluish Slate hexes / `color-mix(in srgb`.




