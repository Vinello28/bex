# Bex GitHub Pages Site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A one-page, "LaTeX paper"-styled, responsive download/usage site for Bex LaTeX Studio, deployed to `https://vinello28.github.io/bex/`.

**Architecture:** Static `site/` folder (HTML + CSS + one ES-module JS file + self-hosted fonts + screenshots), no build step. `download.js` turns the GitHub "latest release" API response into download links through pure functions, which are tested with `node --test`. A dedicated Actions workflow publishes `site/` to Pages.

**Tech Stack:** HTML5, CSS (custom properties, counters), vanilla ES modules, Node 24 `node:test`, fontTools (`pyftsubset`, scratchpad venv only), GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-11-28-github-pages-site-design.md`

## Global Constraints
- English copy. No architecture/tech-stack talk (no Tauri/Svelte/Rust/MCP/WebView). Tectonic may be named as the built-in TeX engine.
- No blue hue anywhere: links, focus, selection, greys. Light: bg `#ffffff`, ink `#111111`, secondary `#555555`, hairline `#d6d6d6`, code bg `#f5f5f4`, accent `#7a3e0c`. Dark: bg `#000000`, ink `#e8e8e8`, secondary `#9a9a9a`, hairline `#2a2a2a`, code bg `#121212`, accent `#e0a040`.
- Static only. Do not add npm dependencies. Do not change `src/`, `src-tauri/`, `public/`, `package.json`.
- All internal URLs are relative, because the site is served under `/bex/`. The only third-party request is the GitHub API.
- No gradients, pill buttons, shadows, icons (other than the mascot), emoji or animations.
- Must not scroll horizontally at 320 px.
- Fallback URL everywhere: `https://github.com/Vinello28/bex/releases/latest`.
- No commits. The user decides about committing at the end.

## Shared contracts (all tasks)
**Fonts** (`site/assets/fonts/`): `lm-roman-regular.woff2`, `lm-roman-italic.woff2`, `lm-roman-bold.woff2`, `lm-roman-bolditalic.woff2`, `lm-roman-caps.woff2`, `lm-mono-regular.woff2`, `GUST-FONT-LICENSE.txt`. CSS families: `"LM Roman"` (400/700, normal/italic), `"LM Roman Caps"`, `"LM Mono"`.

**Images** (`site/assets/img/`): `{editor,errors,assistant,home}-{light,dark}.webp` (1440×900) and `…@2x.webp` (2880×1800), `og-image.png` (1200×630), `bex-bottle.svg` (copied from `public/`).

**Download DOM contract** (HTML provides it, `download.js` fills it in):
- `AssetKey` = `"mac-dmg" | "win-exe" | "win-msi" | "linux-appimage" | "linux-deb" | "linux-rpm"`
- `<a data-asset="KEY" href="RELEASES_URL">` is the download link in each table row.
- `<span data-asset-size="KEY">–</span>` is the size cell.
- `<span data-release="version">latest release</span>` and `<span data-release="date"></span>` sit in the title-block date line.
- `<a id="primary-download" class="button" href="RELEASES_URL"><span data-primary-label>Download</span><span data-primary-note>Latest release on GitHub</span></a>`
- `<div class="code" data-copy><pre><code>…</code></pre></div>`: JS appends `<button type="button" class="copy">Copy</button>` only if `navigator.clipboard` exists.

## Review Focus
1. If the GitHub API is rate-limited (403) or offline, every download link and the primary button must still go to the releases page. Pinned by `model: null release → fallbacks` in Task 3.
2. A future release may lack an asset or add an unrecognised one (e.g. `_x64.dmg` for Intel, `.sig`). Missing rows keep the fallback link, and unknown files are never shown. Pinned in Task 3.
3. Android user agents contain "Linux" and iPadOS reports "Macintosh". Both must be classed as `mobile`, never offered a desktop installer. Pinned in Task 3.
4. A 320 px phone must have no page-level horizontal scroll, even with the long `xattr` command and the 4-column tables. Checked in Task 6.
5. The site is served under the `/bex/` sub-path, where root-absolute URLs (`/assets/...`) break. Checked in Task 6 by serving the parent directory.

---

### Task 1: Real-app screenshots *(already running in background)*
**Files:** Create the images listed in "Images" (except `bex-bottle.svg`).
**Produces:** image files, plus a report with one-line descriptions used for captions and alt text.
- [ ] Captured from installed v0.3.9 over WebView2 CDP, with demo data, real compile, real error, and a real LM Studio `openai/gpt-oss-20b` chat
- [ ] User localStorage restored and verified by hash; demo folders removed; app closed
- [ ] Every image inspected visually

### Task 2: Fonts
**Files:** Create the files listed in "Fonts".
**Produces:** the font files under the contract names.
- [ ] **Step 1:** Download Latin Modern OTFs from CTAN (`https://mirrors.ctan.org/fonts/lm/fonts/opentype/public/lm/`: `lmroman10-regular.otf`, `lmroman10-italic.otf`, `lmroman10-bold.otf`, `lmroman10-bolditalic.otf`, `lmromancaps10-regular.otf`, `lmmono10-regular.otf`) into a fresh scratchpad dir. Get the GUST Font License text from the same distribution (`https://mirrors.ctan.org/fonts/lm/doc/fonts/lm/GUST-FONT-LICENSE.txt`, or the `lm` package docs).
- [ ] **Step 2:** Create a scratchpad venv: `python -m venv <scratch>/venv` then `pip install fonttools brotli`.
- [ ] **Step 3:** Subset each font with `pyftsubset <otf> --flavor=woff2 --layout-features='kern,liga,onum,lnum,smcp' --unicodes="U+0020-007E,U+00A0-00FF,U+0131,U+0152-0153,U+2013-2014,U+2018-201D,U+2026,U+2022,U+2190-2193,U+2212,U+2318,U+21E7"` and write the output under the contract names.
- [ ] **Step 4:** Verify. Each file is < 90 KB and the total is < 450 KB. Print which of ⌘ (U+2318), ⇧ (U+21E7) and ↓ (U+2193) are present in `lm-mono-regular`'s cmap (fontTools `TTFont(...)['cmap']`). Report the missing ones so the CSS falls back.

### Task 3: `download.js` + tests
**Files:** Create `site/assets/js/download.js` and `tests/site/download.test.mjs`.
**Interfaces (exports of `download.js`):**
- `RELEASES_URL`, `API_URL = "https://api.github.com/repos/Vinello28/bex/releases/latest"`
- `classifyAsset(name: string): AssetKey | null`
- `detectPlatform(nav: {userAgent: string, userAgentData?: {platform?: string, mobile?: boolean}, maxTouchPoints?: number}): "mac"|"windows"|"linux"|"mobile"|"unknown"`
- `formatSize(bytes: number): string` (MiB, 1 decimal, `" MB"` suffix)
- `formatDate(iso: string): string` (UTC, `"26 November 2026"`, built from an English month table, no locale APIs)
- `buildDownloadModel(release: {tag_name, published_at, assets: {name, size, browser_download_url}[]} | null, platform): {version: string|null, date: string|null, rows: Partial<Record<AssetKey,{href,size}>>, primary: {href, label, note}}`
- `init(): Promise<void>` runs only when `typeof document !== "undefined"` (auto-called on `DOMContentLoaded`). It does a fetch with a 5 s `AbortController` timeout and a `sessionStorage` cache (10 min, wrapped in try/catch), applies the model to the DOM contract, and adds the copy buttons.

- [ ] **Step 1:** Write `tests/site/download.test.mjs` (`node:test` + `node:assert/strict`) using the real v0.3.9 asset list from the spec §3 (sizes: dmg 25645091, exe 16168827, msi 24358912, AppImage 89315832, deb 11981788, rpm 11987420):
  - `classifyAsset`: each of the 6 names → its key. `"…aarch64.app.tar.gz"`, `"….sig"`, `"latest.json"` and `"Bex.LaTeX.Studio_0.4.0_x64.dmg"` → `null`.
  - `detectPlatform`: Win10 Chrome UA → `windows`; macOS Safari UA with `maxTouchPoints: 0` → `mac`; the same UA with `maxTouchPoints: 5` (iPad) → `mobile`; Ubuntu Firefox UA → `linux`; Android Chrome UA → `mobile`; iPhone UA → `mobile`; `userAgentData.platform: "macOS"` wins over the UA; `""` → `unknown`.
  - `formatSize(25645091) === "24.5 MB"`, `formatSize(89315832) === "85.2 MB"`.
  - `formatDate("2026-11-26T18:07:55Z") === "26 November 2026"`.
  - Model with the full release and `mac`: `version === "0.3.9"`, `primary.label === "Download for macOS"`, `primary.note === "Apple Silicon · .dmg · 24.5 MB"`, `primary.href` = the dmg's `browser_download_url`, all 6 rows present.
  - `windows` → `primary.label "Download for Windows"`, note `"64-bit installer · .exe · 15.4 MB"`. `linux` → `"Download for Linux"`, note `"AppImage · 85.2 MB"`.
  - `mobile`/`unknown` → `{href: "#download", label: "See all downloads", note: "Bex is a desktop app for macOS, Windows and Linux."}`.
  - Model with `null` release and `mac` → version/date `null`, `rows` `{}`, `primary.href === RELEASES_URL`, label `"Download for macOS"`, note `"Latest release on GitHub"`.
  - A release lacking the `.rpm` → `rows["linux-rpm"]` undefined and the other rows present.
- [ ] **Step 2:** Run `node --test "tests/site/*.test.mjs"`. Expected: FAIL (module not found).
- [ ] **Step 3:** Implement `download.js` per the Interfaces. The DOM code stays a thin applier over `buildDownloadModel` and only touches the contract selectors.
- [ ] **Step 4:** Run `node --test "tests/site/*.test.mjs"`. Expected: all pass.

### Task 4: Pages workflow + README link
**Files:** Create `.github/workflows/pages.yml`. Modify `README.md` by adding one line after the H1: `**Website & downloads:** https://vinello28.github.io/bex/`.
- [ ] **Step 1:** Write the workflow per spec §5.6 (triggers on push to `main` with paths `site/**` and `.github/workflows/pages.yml`, plus `workflow_dispatch`; permissions; concurrency `pages`, `cancel-in-progress: false`; job `deploy` with `environment: {name: github-pages, url: ${{ steps.deployment.outputs.page_url }}}`; `checkout@v4`, `configure-pages@v5`, `upload-pages-artifact@v3` with `path: site`, `deploy-pages@v4` with `id: deployment`).
- [ ] **Step 2:** Validate the YAML parses: `python -c "import yaml,sys;yaml.safe_load(open(sys.argv[1]))" .github/workflows/pages.yml` (install PyYAML into the scratchpad venv if needed). Expected: no error. Confirm `release.yml` is untouched (`git diff --stat`).

### Task 5: `index.html` + `style.css`
**Files:** Create `site/index.html`, `site/assets/css/style.css`, `site/.nojekyll`. Copy `public/bex-bottle.svg` → `site/assets/img/bex-bottle.svg`.
**Consumes:** font names, image names, the DOM contract (above). Script tag: `<script type="module" src="assets/js/download.js"></script>`.
- [ ] **Step 1:** Write `index.html` with the structure and facts of spec §5.2 (title block → abstract → primary button + "Other platforms ↓" → Contents → §1–§5 → References → footer) , the head/meta of §5.7 and the accessibility rules of §5.8 (skip link, landmarks, `<th scope>`). `og:image` = `https://vinello28.github.io/bex/assets/img/og-image.png`. Use ids `download`, `installation`, `getting-started`, `features`, `shortcuts`, `references`. Each figure is a `<figure>` with `<picture>`: a `<source media="(prefers-color-scheme: dark)" srcset="…-dark.webp 1x, …-dark@2x.webp 2x">` plus an `<img src="…-light.webp" srcset="…-light.webp 1x, …-light@2x.webp 2x" width="1440" height="900" alt="…">`, with `loading="lazy"` on all but Figure 1, and a `<figcaption>`. Captions and alt text are provisional, finalised in Task 6. Every factual claim comes from spec §5.2. Verify any feature wording you add against `src/` before writing it.
- [ ] **Step 2:** Write `style.css` with the tokens from Global Constraints (`:root` plus the `@media (prefers-color-scheme: dark)` override), `@font-face` for the contract fonts with `font-display: swap`, the `kbd`/`code` stack `"LM Mono", ui-monospace, "Cascadia Mono", "Segoe UI Symbol", monospace` (covers ⌘ ⇧), `::selection` and `:focus-visible` in the accent colour, CSS-counter section numbering ("1", "2.1"), booktabs tables (top/bottom rule 1.5px ink, mid rule 0.75px ink, no vertical rules), `.button` per spec §5.3, measure ~37em, figures breaking out to `min(60em, 100vw - 32px)`, justified + `hyphens:auto` from `min-width: 44em`, and tables and code wrapped in `overflow-x: auto`.
- [ ] **Step 3:** Check it. `python -m http.server 8080 -d site`, then open `http://localhost:8080/`. The page renders with fonts loaded and no console errors (missing images are expected if Task 1 hasn't finished). Grep `site/` for blue: `grep -RniE '#(0f172a|1e293b|334155|475569|64748b|94a3b8|cbd5e1|e2e8f0|f1f5f9)|\bblue\b|rgb\(0, ?0, ?255' site/`. Expected: no matches.

### Task 6: Integration & verification *(after Tasks 1–5)*
**Files:** Modify `site/index.html` (captions and alt text from the Task 1 report). Modify `tasks/todo.md` (review section).
- [ ] **Step 1:** Rewrite the figure captions and alt text from the Task 1 report (the actual content of each shot, including the real AI prompt and what it did).
- [ ] **Step 2:** Sub-path test: serve the repo root with `python -m http.server 8090` and open `http://localhost:8090/site/`. All fonts, images, CSS and JS return 200 (check the network panel). The download table and primary button are filled from the API.
- [ ] **Step 3:** Responsive and theme pass in Chrome at 320, 375, 768, 1280 and 1920 px, in light and dark (emulate `prefers-color-scheme`). Check `document.documentElement.scrollWidth <= innerWidth` at each width. Screenshot each for the record.
- [ ] **Step 4:** API failure test: block `api.github.com` (DevTools request blocking, or a `fetch` override before load). All `[data-asset]` hrefs and `#primary-download` point to `RELEASES_URL`.
- [ ] **Step 5:** Re-run `node --test "tests/site/*.test.mjs"` and the blue grep. Check every `href="#…"` matches an id.
- [ ] **Step 6:** Run a whole-change code review (fresh reviewer subagent), fix the findings, and write the review summary into `tasks/todo.md` (Step 12).
