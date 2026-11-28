# Bex LaTeX Studio

**Website & downloads:** https://vinello28.github.io/bex/

A cross-platform live LaTeX editor built with **Tauri v2**, **Svelte 5**, and **Rust**, featuring an embedded **Tectonic** typesetting engine alongside optional external `pdflatex`, `xelatex`, and `lualatex` support.

## Local Development Prerequisites

Because Bex LaTeX Studio embeds the Rust `tectonic` crate directly in-process, native C/C++ typography libraries are required when compiling the backend.

### All Platforms
- [Node.js](https://nodejs.org/) (v20 or v22 recommended)
- [Rust toolchain](https://rustup.rs/) (`stable`)

### macOS
Install the required libraries via Homebrew:
```bash
brew install pkg-config icu4c harfbuzz freetype graphite2 fontconfig openssl@3
```
Ensure `PKG_CONFIG_PATH` includes Homebrew's keg-only packages if not using the bundled `src-tauri/.cargo/config.toml` defaults:
```bash
export PKG_CONFIG_PATH="/opt/homebrew/opt/icu4c/lib/pkgconfig:/opt/homebrew/opt/openssl@3/lib/pkgconfig:/opt/homebrew/lib/pkgconfig:${PKG_CONFIG_PATH}"
```

### Linux (Ubuntu / Debian)
```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  pkg-config \
  libicu-dev \
  libharfbuzz-dev \
  libharfbuzz-icu0 \
  libfreetype6-dev \
  libgraphite2-dev \
  libfontconfig1-dev \
  libssl-dev
```

### Windows
Install the native Tectonic dependencies using `cargo-vcpkg` (configured via `[package.metadata.vcpkg]` in `src-tauri/Cargo.toml`):
```powershell
cargo install cargo-vcpkg
cd src-tauri
cargo vcpkg build
$env:TECTONIC_DEP_BACKEND = "vcpkg"
$env:VCPKG_ROOT = "$PWD\target\vcpkg"
$env:VCPKGRS_TRIPLET = "x64-windows-static-md"
```

## Running Locally

```bash
npm install
npm run tauri dev
```

To build production installers locally:
```bash
npm run tauri build
```

## Installing on macOS (Unsigned / Ad-hoc Signed Releases)

Pre-built macOS `.dmg` releases are ad-hoc signed (`signingIdentity: "-"`) and are **not** notarized with an Apple Developer ID certificate. Because browsers attach `com.apple.quarantine` to downloaded installers, macOS Gatekeeper on Apple Silicon may show *"Bex LaTeX Studio is damaged and can't be opened"* when launching the downloaded app.

After dragging **Bex LaTeX Studio.app** from the `.dmg` into `/Applications`, remove the quarantine attribute once in Terminal:
```bash
xattr -dr com.apple.quarantine "/Applications/Bex LaTeX Studio.app"
```

Note: All required Homebrew dynamic libraries (`icu4c`, `freetype`, `graphite2`, `libpng`, `brotli`) are bundled inside `Bex LaTeX Studio.app/Contents/Frameworks`, so end users do **not** need Homebrew installed to run the `.dmg` release.

## GitHub Releases & Auto-Updater Setup

Bex LaTeX Studio uses **`tauri-plugin-updater`** and **`tauri-plugin-process`** (registered in `src-tauri/src/lib.rs`, configured in `src-tauri/tauri.conf.json`, and permitted in `src-tauri/capabilities/default.json`) to check for signed updates via GitHub Releases (`latest.json`).

### How It Works
1. `src-tauri/tauri.conf.json` sets `"bundle.createUpdaterArtifacts": true`, `"plugins.updater.pubkey"`, and `"plugins.updater.endpoints"` pointing to:
   `https://github.com/Vinello28/bex/releases/latest/download/latest.json`
2. `.github/workflows/release.yml` runs on `v*` tag pushes (and manual `workflow_dispatch`) across:
   - **macOS Apple Silicon** (`macos-latest`, `--target aarch64-apple-darwin`): `.dmg` + `.app.tar.gz` + `.sig`
   - **Linux** (`ubuntu-22.04`): `.deb`, `.rpm`, `.AppImage` + `.sig`
   - **Windows** (`windows-latest`): `.msi`, NSIS `-setup.exe` + `.sig`
3. `tauri-apps/tauri-action@v0` signs the updater artifacts and uploads `latest.json` alongside all platform installers.

> **Important:** The updater endpoint in `src-tauri/tauri.conf.json` is configured for the **`Vinello28/bex`** GitHub repository. If you fork or rename the repository, update `plugins.updater.endpoints` to match `https://github.com/<OWNER>/<REPO>/releases/latest/download/latest.json`.

### Signing Secrets Setup
Before running the release workflow, configure the signing secrets in your GitHub repository (**Settings → Secrets and variables → Actions**):

1. Set **`TAURI_SIGNING_PRIVATE_KEY`** from `~/.tauri/bex-latex-studio.key` (recommended via GitHub CLI to avoid copy/paste newline issues):
   ```bash
   gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/bex-latex-studio.key
   ```
2. If your key is encrypted with a passphrase, also add **`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`**.
3. If you prefer to generate a fresh keypair:
   ```bash
   npx tauri signer generate -w ~/.tauri/bex-latex-studio.key
   ```
   Then replace `plugins.updater.pubkey` in `src-tauri/tauri.conf.json` with the newly generated public key and update `TAURI_SIGNING_PRIVATE_KEY` in GitHub Secrets.

### Cutting a Release
The release version is derived **solely** from the `v*` git tag — you do not need to bump `package.json`, `src-tauri/tauri.conf.json`, or `src-tauri/Cargo.toml` manually.

1. Ensure your changes are committed and pushed to your branch:
   ```bash
   git push
   ```
2. Create and push a SemVer `v*` tag:
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```
   Pushing a `v*` tag triggers `.github/workflows/release.yml`, which:
   - Fails fast upfront if `TAURI_SIGNING_PRIVATE_KEY` is missing or malformed, or if the tag is not a valid `vX.Y.Z` SemVer tag.
   - Automatically syncs the tag version (without the `v` prefix) into `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml` right before building.
   - Publishes the GitHub Release (`v<TAG_VERSION>`) with platform installers, `.sig` files, and `latest.json` (marking `-prerelease` tags as GitHub prereleases automatically). You can also trigger a draft or prerelease build manually via **Run workflow** (`workflow_dispatch`) from a `v*` tag.
