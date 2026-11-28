// Turns the GitHub "latest release" API response into OS-aware download links.
// Without JS (or on any failure) the static links to RELEASES_URL stay in place.

export const RELEASES_URL = "https://github.com/Vinello28/bex/releases/latest";
export const API_URL = "https://api.github.com/repos/Vinello28/bex/releases/latest";

const ASSET_PATTERNS = [
  ["mac-dmg", /aarch64\.dmg$/],
  ["win-exe", /-setup\.exe$/],
  ["win-msi", /\.msi$/],
  ["linux-appimage", /\.AppImage$/],
  ["linux-deb", /\.deb$/],
  ["linux-rpm", /\.rpm$/],
];

const PRIMARY = {
  mac: { key: "mac-dmg", label: "Download for macOS", note: "Apple Silicon only · .dmg · " },
  windows: { key: "win-exe", label: "Download for Windows", note: "64-bit installer · .exe · " },
  linux: { key: "linux-appimage", label: "Download for Linux", note: "AppImage · " },
};

const MOBILE_PRIMARY = {
  href: "#download",
  label: "See all downloads",
  note: "Bex is a desktop app for macOS, Windows and Linux.",
};

const MONTHS = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

const UA_DATA_PLATFORMS = {
  macOS: "mac",
  Windows: "windows",
  Linux: "linux",
  Android: "mobile",
  iOS: "mobile",
};

export function classifyAsset(name) {
  if (typeof name !== "string") return null;
  const match = ASSET_PATTERNS.find(([, pattern]) => pattern.test(name));
  return match ? match[0] : null;
}

export function detectPlatform(nav) {
  const data = nav.userAgentData;
  if (data?.mobile) return "mobile";
  if (Object.hasOwn(UA_DATA_PLATFORMS, data?.platform)) return UA_DATA_PLATFORMS[data.platform];

  const ua = nav.userAgent || "";
  if (/Android|iPhone|iPad|iPod/.test(ua)) return "mobile";
  if (/Windows/.test(ua)) return "windows";
  // iPadOS Safari reports a desktop Mac UA; only touch support tells it apart.
  if (/Macintosh/.test(ua)) return (nav.maxTouchPoints || 0) > 1 ? "mobile" : "mac";
  if (/Linux/.test(ua)) return "linux";
  return "unknown";
}

export function formatSize(bytes) {
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

export function formatDate(iso) {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return `${d.getUTCDate()} ${MONTHS[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
}

export function buildDownloadModel(release, platform) {
  const rows = {};
  for (const { name, size, browser_download_url: href } of release?.assets ?? []) {
    const key = classifyAsset(name);
    // A malformed asset keeps its row on the static fallback link.
    if (!key || rows[key] || typeof href !== "string" || !Number.isFinite(size)) continue;
    rows[key] = { href, size };
  }

  let primary = MOBILE_PRIMARY;
  if (Object.hasOwn(PRIMARY, platform)) {
    const target = PRIMARY[platform];
    const row = rows[target.key];
    primary = row
      ? { href: row.href, label: target.label, note: target.note + formatSize(row.size) }
      : { href: RELEASES_URL, label: target.label, note: "Latest release on GitHub" };
  }

  return {
    version: release?.tag_name ? String(release.tag_name).replace(/^v/, "") : null,
    date: (release?.published_at && formatDate(release.published_at)) || null,
    rows,
    primary,
  };
}

// ---- Browser only ----------------------------------------------------------

const CACHE_KEY = "bex-latest-release";
const CACHE_TTL_MS = 10 * 60 * 1000;
const FETCH_TIMEOUT_MS = 5000;

function readCache() {
  try {
    const cached = JSON.parse(sessionStorage.getItem(CACHE_KEY));
    if (cached && Date.now() - cached.at < CACHE_TTL_MS) return cached.release;
  } catch {}
  return null;
}

function writeCache(release) {
  try {
    sessionStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), release }));
  } catch {}
}

async function fetchRelease() {
  const cached = readCache();
  if (cached) return cached;

  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);
  try {
    const res = await fetch(API_URL, { signal: controller.signal });
    if (!res.ok) return null;
    const { tag_name, published_at, assets } = await res.json();
    const release = {
      tag_name,
      published_at,
      assets: (assets ?? []).map(({ name, size, browser_download_url }) => ({
        name,
        size,
        browser_download_url,
      })),
    };
    writeCache(release);
    return release;
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

function setText(selector, text) {
  for (const el of document.querySelectorAll(selector)) el.textContent = text;
}

function applyModel(model) {
  if (model.version) setText('[data-release="version"]', model.version);
  if (model.date) setText('[data-release="date"]', model.date);

  for (const [key, row] of Object.entries(model.rows)) {
    for (const a of document.querySelectorAll(`a[data-asset="${key}"]`)) a.href = row.href;
    setText(`[data-asset-size="${key}"]`, formatSize(row.size));
  }

  const button = document.getElementById("primary-download");
  if (!button) return;
  button.href = model.primary.href;
  const label = button.querySelector("[data-primary-label]");
  const note = button.querySelector("[data-primary-note]");
  if (label) label.textContent = model.primary.label;
  if (note) note.textContent = model.primary.note;
}

// One shared, visually hidden live region announces the result of a copy.
function copyStatusRegion() {
  let region = document.querySelector(".copy-status");
  if (!region) {
    region = document.createElement("div");
    region.className = "copy-status vh";
    region.setAttribute("role", "status");
    region.setAttribute("aria-live", "polite");
    document.body.append(region);
  }
  return region;
}

function addCopyButtons() {
  if (!navigator.clipboard) return;
  const blocks = document.querySelectorAll("[data-copy]");
  if (!blocks.length) return;
  const status = copyStatusRegion();
  for (const block of blocks) {
    const code = block.querySelector("code");
    if (!code || block.querySelector("button.copy")) continue;
    const button = document.createElement("button");
    button.type = "button";
    button.className = "copy";
    button.textContent = "Copy";
    button.setAttribute("aria-label", "Copy command");
    let timer;
    const flash = (text) => {
      button.textContent = text;
      status.textContent = text;
      clearTimeout(timer);
      timer = setTimeout(() => {
        button.textContent = "Copy";
        status.textContent = "";
      }, 1500);
    };
    button.addEventListener("click", () => {
      navigator.clipboard.writeText(code.textContent).then(
        () => flash("Copied"),
        () => flash("Copy failed"),
      );
    });
    block.append(button);
  }
}

export async function init() {
  if (typeof document === "undefined") return;
  addCopyButtons();
  const release = await fetchRelease();
  applyModel(buildDownloadModel(release, detectPlatform(navigator)));
}

if (typeof document !== "undefined") {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => init());
  } else {
    init();
  }
}
