# Bex LaTeX Studio — GitHub Pages Site (Design)

Date: 2026-11-28 · Status: draft for approval

## 1. Intent

A public one-page website for **Bex LaTeX Studio** (repo `Vinello28/bex`, public), served by GitHub Pages at
`https://vinello28.github.io/bex/`.

**Audience:** the author's colleagues (students/researchers who write papers and theses in LaTeX).
**Job of the page:** get a colleague from "what is this?" to "installed and compiling my first document" with no
friction. How the app is built (Tauri, Svelte, Rust, MCP…) is explicitly **not** of interest to them and must not appear.

**Visual direction (decided by the user): "Paper LaTeX"** — the page reads like an article typeset with LaTeX:
Latin Modern type, centered title block, an *Abstract*, numbered sections, booktabs-style tables, figures with
"Figure n:" captions. Academic and dry. **No bluish colours anywhere.** Responsive. **Language: English.**

## 2. Goals / non-goals / anti-goals

**Goals**
- One-click download of the right installer for the visitor's OS, plus a complete table of all installers.
- Clear, OS-specific install instructions, including the unsigned-app workarounds (macOS quarantine, Windows SmartScreen).
- Show what the app does, in user terms, with real screenshots.
- A "getting started" walkthrough and a keyboard-shortcut reference.
- Works and looks right from 320 px phones to wide desktop, in light and dark system themes.

**Non-goals**
- Architecture, tech stack, contributor/build-from-source docs (the README covers that).
- Multi-page docs site, blog, changelog page, i18n, custom domain, analytics, cookies, any tracker.
- Any change to the app itself.

**Anti-goals (failure even if it "works")**
- Looks like a SaaS landing page: gradients, pill buttons, hero blobs, feature-icon grids, emoji, testimonials, animations.
- Any blue/bluish tone: default link blue, blue focus rings, blue text selection, slate greys
  (`#0f172a`, `#1e293b`, `#64748b`, `#f1f5f9`, etc. — see `tasks/lessons.md`).
- Horizontal page scroll on mobile.
- Dead download links when the GitHub API is unreachable or rate-limited.
- Claims about features the app does not have.

## 3. Constraints

- Static files only, no build step, no framework, no npm dependency added to the repo.
- Must work under the `/bex/` sub-path → all internal URLs relative.
- No third-party requests at page load except the GitHub API call for release data (fonts are self-hosted).
- Must not affect the app build: nothing under `src/`, `src-tauri/`, `public/`, `package.json` changes.
- Release assets currently published (v0.3.9, tag-driven, names contain the version):
  ```
  Bex.LaTeX.Studio_0.3.9_aarch64.dmg          macOS Apple Silicon (no Intel build exists)
  Bex.LaTeX.Studio_0.3.9_x64-setup.exe        Windows (NSIS installer)
  Bex.LaTeX.Studio_0.3.9_x64_en-US.msi        Windows (MSI)
  Bex.LaTeX.Studio_0.3.9_amd64.AppImage       Linux
  Bex.LaTeX.Studio_0.3.9_amd64.deb            Linux (Debian/Ubuntu)
  Bex.LaTeX.Studio-0.3.9-1.x86_64.rpm         Linux (Fedora/RHEL)
  + *.sig, Bex.LaTeX.Studio_aarch64.app.tar.gz, latest.json   ← updater artifacts, never shown
  ```
  Because filenames embed the version, stable `releases/latest/download/<name>` links are impossible → resolve via API.

## 4. Approaches considered

| Approach | Trade-off | Verdict |
|---|---|---|
| **A. Hand-written static HTML + CSS + one small JS file in `site/`, deployed by a dedicated GitHub Actions Pages workflow** | Zero tooling, full control of typography, trivially reviewable | **Chosen** (builder's call, stated to user) |
| B. Jekyll (native GitHub Pages) | Markdown authoring, but themes fight the custom "paper" look; extra moving parts for one page | Rejected |
| C. Separate Vite/Svelte project | Reuses app stack but adds a build step and deps for a single static page | Rejected |

Deploy source: `site/` via Actions (not "deploy from `/docs`"), because `docs/` holds internal specs that must not be published.

## 5. Design

### 5.1 File layout
```
site/
  index.html
  assets/css/style.css
  assets/js/download.js
  assets/fonts/*.woff2  + GUST-FONT-LICENSE.txt
  assets/img/           screenshots (light + dark variants), bex-bottle.svg (favicon/logo, copied from public/)
.github/workflows/pages.yml
README.md                (one added line linking to the site)
```

### 5.2 Page structure (top to bottom)
1. **Title block** (centered, like `\maketitle`): small mascot logo; title "Bex LaTeX Studio"; subtitle
   "A desktop LaTeX editor with the TeX engine built in"; a "date line" `Version 0.3.9 · 26 November 2026 · macOS · Windows · Linux`
   (filled from the API; static fallback text "Latest release" with no stale numbers).
2. **Abstract** (indented, smaller, run-in bold "Abstract."): 3–4 sentences — write papers and theses; no TeX
   distribution to install; live PDF preview; free.
3. **Primary download**: one rectangular, bordered button "Download for {OS}" + secondary link "Other platforms ↓" (to §1).
4. **Contents** (like `\tableofcontents`): numbered list of the sections below.
5. **§1 Download** — booktabs table: *Platform · Package · Size · Link*. Rows in this order: macOS (Apple Silicon) `.dmg`;
   Windows installer `.exe` (marked recommended); Windows `.msi`; Linux `.AppImage`; Linux `.deb`; Linux `.rpm`.
   Footnote: Apple Silicon only, no Intel Mac build. Link to "all releases" page.
6. **§2 Installation** — 2.1 macOS (drag to Applications, then run
   `xattr -dr com.apple.quarantine "/Applications/Bex LaTeX Studio.app"`, explained in one sentence: the app is not
   notarized by Apple); 2.2 Windows (SmartScreen: *More info → Run anyway*); 2.3 Linux (`chmod +x` AppImage;
   `sudo apt install ./….deb`; `sudo dnf install ./….rpm`); 2.4 Updates (the app checks for updates itself — no need to come back).
   Code blocks have a copy button.
7. **§3 Getting started** — numbered steps: launch → Home screen → *New Project* / *Open Directory* / *Sample Project* →
   write → `Ctrl/⌘+S` saves and compiles, PDF updates on the right → double-click in the PDF jumps to the source line.
   Note: the first compile downloads the LaTeX packages it needs, so it requires an internet connection and takes longer.
8. **§4 Features** — short paragraphs, user language, each tied to a figure where useful:
   built-in TeX engine (Tectonic; pdfLaTeX/XeLaTeX/LuaLaTeX also usable if already installed); live PDF preview;
   source↔PDF sync both ways; errors listed and marked in the editor, click to jump; optional AI assistant
   (bring your own Google AI Studio key, or a local model via Ollama / LM Studio / vLLM) that can read and edit project files;
   home screen with recent projects; light/dark theme and interface zoom; automatic updates.
   Figures: (1) editor + PDF preview, (2) error diagnostics, (3) AI assistant sidebar, (4) home screen.
9. **§5 Keyboard shortcuts** — booktabs table *Action · macOS · Windows/Linux*, taken from the app's Settings → About:
   Save & compile `⌘S`/`Ctrl+S`; Toggle file sidebar `⌘B`/`Ctrl+B`; AI sidebar `⌘⇧A`/`Ctrl+Shift+A`;
   Sync editor → PDF `⌘⇧J`/`Ctrl+Shift+J`; PDF → source: double-click (or `⌘`/`Ctrl`+click); Toggle comment `⌘ù` or `⌘/` / `Ctrl+ù` or `Ctrl+/`;
   Interface zoom `⌘+`/`⌘−`, reset `⌘0` (Ctrl equivalents); Settings `⌘,`/`Ctrl+,`.
10. **References** (styled like a bibliography, `[1]` labels): source code on GitHub; all releases; issue tracker; Tectonic project.
11. **Footer**: one line — "Bex LaTeX Studio · source on GitHub · report an issue". No licence line (repo has no LICENSE).

All copy must be checked against the codebase; nothing is claimed that the app doesn't do.

### 5.3 Visual system
- **Type**: Latin Modern Roman (regular, italic, bold, bold italic), Latin Modern Roman Caps for small caps,
  Latin Modern Mono for code/kbd. Self-hosted WOFF2 subset to Latin + needed symbols (⌘ ⇧ → ↓ · – — ’ “ ”),
  GUST Font License text shipped alongside. Fallback stack: `"Latin Modern Roman", "Computer Modern", Georgia, serif`.
  Body ~19 px desktop / 17 px mobile, line-height ~1.45.
- **Colour tokens** (CSS custom properties on `:root`, dark set under `prefers-color-scheme: dark`):
  - light: background `#ffffff`, ink `#111111`, secondary `#555555`, rules `#111111` (booktabs) / `#d6d6d6` (hairlines),
    code bg `#f5f5f4`, accent (links, focus, selection) warm brown `#7a3e0c`.
  - dark: background `#000000`, ink `#e8e8e8`, secondary `#9a9a9a`, rules `#e8e8e8` / `#2a2a2a`, code bg `#121212`,
    accent amber `#e0a040`.
  - Exact values may be tuned by the builder, but must stay neutral/warm, pass WCAG AA, and contain no blue hue.
- **Layout**: single centered column, measure ≈ 36–38 em; figures may break out wider (up to ~60 em) on large screens.
  Text justified with `hyphens: auto` from ~700 px up; left-aligned below. Gutters ≥ 16 px. Section numbers via CSS
  counters ("1  Download", "2.1  macOS"). Tables: `overflow-x: auto` wrapper or stacked rows on narrow screens —
  never page-level horizontal scroll at 320 px.
- **Figures**: screenshot with 1 px hairline border, caption below, centered, "Figure 1: …" (LaTeX article style).
  `<picture>` with a dark-theme screenshot source under `prefers-color-scheme: dark`. WebP with `srcset` 1x (1440×900)
  / 2x (2880×1800), lazy-loaded except Figure 1, explicit `width="1440" height="900"` to avoid layout shift.
- **Button**: rectangular, 1 px ink border, small caps label, no radius beyond 2 px, no shadow; hover = inverted colours.
- No animations, no icons other than the mascot logo.

### 5.4 Download logic (`download.js`)
- No-JS baseline: every link in the HTML points to `https://github.com/Vinello28/bex/releases/latest`; page is fully usable.
- On load: `fetch("https://api.github.com/repos/Vinello28/bex/releases/latest")` (unauthenticated, 60 req/h/IP).
  Map assets by filename pattern: `aarch64\.dmg$` → macOS; `-setup\.exe$` → Windows exe; `\.msi$` → Windows msi;
  `\.AppImage$`, `\.deb$`, `\.rpm$` → Linux. Ignore `.sig`, `.app.tar.gz`, `latest.json`.
  Fill links, sizes (MB, one decimal), version and release date (title block). On any failure: leave the fallback links.
- OS detection (`navigator.userAgentData?.platform` then `navigator.userAgent`): Mac → `.dmg` (button sub-label "Apple Silicon");
  Windows → `-setup.exe`; Linux (not Android) → `.AppImage`; iOS/Android/unknown → button becomes "See all downloads"
  (anchor to §1) with a note that Bex is a desktop app.
- Asset matching and OS detection are pure functions, exported so they can be unit-tested with Node.
- Copy-to-clipboard buttons on code blocks (Clipboard API; hidden when unavailable).

### 5.5 Screenshots (decided: real app, captured by Claude)
- Captured from the **real installed app (v0.3.9, Windows)**, launched with
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333` and driven over the Chrome DevTools Protocol by a
  throwaway script kept **outside the repo** (scratchpad). Browser preview mode is rejected: its PDF, diagnostics and AI
  replies are mocks.
- Demo data only: a multi-file demo paper created under `~/Documents/Papers/`, demo entries in recent projects. The user's
  localStorage (recent projects, AI settings) is backed up silently and restored afterwards; demo folders deleted.
- Real Tectonic compile, real diagnostics (a deliberately introduced error, reverted), **real AI conversation with the
  user's local LM Studio running `openai/gpt-oss-20b`**, including a visible file-editing tool call.
- Viewport 1440×900, captured at DPR 1 and 2 as WebP (served with `srcset`), light and dark app themes.
  Files in `site/assets/img/`: `{editor,errors,assistant,home}-{light,dark}[@2x].webp`, plus `og-image.png` (1200×630).

### 5.6 Deployment
- `.github/workflows/pages.yml`: on `push` to `main` with `paths: [site/**, .github/workflows/pages.yml]`, plus
  `workflow_dispatch`. Permissions `contents: read, pages: write, id-token: write`; concurrency group `pages`.
  Steps: `actions/checkout@v4` → `actions/configure-pages@v5` → `actions/upload-pages-artifact@v3` (`path: site`) →
  `actions/deploy-pages@v4`.
- **One-time manual step for the user:** repo Settings → Pages → Source: **GitHub Actions**.
- `README.md`: add one line under the title linking to the site.

### 5.7 Head / meta
`lang="en"`, title "Bex LaTeX Studio — a LaTeX editor with the TeX engine built in", meta description, Open Graph +
Twitter card (absolute `og:image` = Figure 1 light), `theme-color` for light/dark, SVG favicon (mascot),
`color-scheme: light dark`.

### 5.8 Accessibility
Semantic landmarks and heading order, skip link, `<th scope>` in tables, descriptive alt text, visible focus ring in
accent colour, AA contrast in both themes, honours `prefers-reduced-motion` (nothing animates anyway).

## 6. Verification
- Serve `site/` locally (`python -m http.server`) and check in Chrome at 320, 375, 768, 1280, 1920 px, light and dark.
- No horizontal scroll at 320 px; tables readable on mobile.
- Node unit test (scratchpad, not committed) of asset mapping + OS detection against the real v0.3.9 asset list.
- Simulate API failure (block request) → all links still resolve to the releases page.
- Palette audit: grep `site/` for blue-hue hex values / named blues.
- All internal links/anchors resolve; all images load under a `/bex/` sub-path (serve from a parent dir to mimic).
- Workflow YAML reviewed; real deploy happens after the user enables Pages and pushes.

## 7. Left to the builder
Exact copy wording (within §5.2 facts), exact colour values within the stated constraints, font subsetting method,
screenshot capture tooling, whether the Contents block becomes a sticky margin TOC on very wide screens,
mobile table strategy (scroll vs. stacked).

## 8. Decisions log
- **Screenshots:** from the real app, produced by Claude (user: "fai partire l'app e falli tu gli screenshot").
- **AI figure:** a real conversation with local LM Studio running gpt-oss-20b.
- **Builder's calls (Claude):** Windows `.exe` recommended over `.msi` (the updater supports both install kinds);
  screenshots show the Windows build; the page uses `srcset` 1x/2x instead of a single downscaled image.
