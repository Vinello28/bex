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

