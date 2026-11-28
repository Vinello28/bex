import { test } from "node:test";
import assert from "node:assert/strict";

// Import with a fetch spy installed, to prove the module never touches the
// network at import time (init() must only run when a DOM exists).
let fetchCalls = 0;
const realFetch = globalThis.fetch;
globalThis.fetch = () => {
  fetchCalls++;
  return Promise.reject(new Error("network is disabled in tests"));
};
const {
  RELEASES_URL,
  API_URL,
  classifyAsset,
  detectPlatform,
  formatSize,
  formatDate,
  buildDownloadModel,
} = await import("../../site/assets/js/download.js");
globalThis.fetch = realFetch;

const DL = "https://github.com/Vinello28/bex/releases/download/v0.3.9/";

const ASSETS = {
  "mac-dmg": { name: "Bex.LaTeX.Studio_0.3.9_aarch64.dmg", size: 25645091 },
  "win-exe": { name: "Bex.LaTeX.Studio_0.3.9_x64-setup.exe", size: 16168827 },
  "win-msi": { name: "Bex.LaTeX.Studio_0.3.9_x64_en-US.msi", size: 24358912 },
  "linux-appimage": { name: "Bex.LaTeX.Studio_0.3.9_amd64.AppImage", size: 89315832 },
  "linux-deb": { name: "Bex.LaTeX.Studio_0.3.9_amd64.deb", size: 11981788 },
  "linux-rpm": { name: "Bex.LaTeX.Studio-0.3.9-1.x86_64.rpm", size: 11987420 },
};

const UPDATER_ASSETS = [
  { name: "Bex.LaTeX.Studio_aarch64.app.tar.gz", size: 25000000 },
  { name: "Bex.LaTeX.Studio_aarch64.app.tar.gz.sig", size: 420 },
  { name: "Bex.LaTeX.Studio_0.3.9_x64-setup.exe.sig", size: 420 },
  { name: "latest.json", size: 2048 },
];

const asset = ({ name, size }) => ({ name, size, browser_download_url: DL + name });

const RELEASE = {
  tag_name: "v0.3.9",
  published_at: "2026-11-26T18:07:55Z",
  assets: [...Object.values(ASSETS), ...UPDATER_ASSETS].map(asset),
};

const UA = {
  windowsChrome:
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36",
  macSafari:
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.6 Safari/605.1.15",
  ubuntuFirefox: "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:145.0) Gecko/20100101 Firefox/145.0",
  androidChrome:
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Mobile Safari/537.36",
  iPhoneSafari:
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_6 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.6 Mobile/15E148 Safari/604.1",
};

const MOBILE_PRIMARY = {
  href: "#download",
  label: "See all downloads",
  note: "Bex is a desktop app for macOS, Windows and Linux.",
};

test("importing in Node makes no network call", () => {
  assert.equal(fetchCalls, 0);
});

test("URLs", () => {
  assert.equal(RELEASES_URL, "https://github.com/Vinello28/bex/releases/latest");
  assert.equal(API_URL, "https://api.github.com/repos/Vinello28/bex/releases/latest");
});

test("classifyAsset maps each published installer to its key", () => {
  for (const [key, { name }] of Object.entries(ASSETS)) {
    assert.equal(classifyAsset(name), key, name);
  }
});

test("classifyAsset ignores updater artifacts and unsupported builds", () => {
  for (const name of [
    "Bex.LaTeX.Studio_aarch64.app.tar.gz",
    "Bex.LaTeX.Studio_0.3.9_aarch64.dmg.sig",
    "Bex.LaTeX.Studio_0.3.9_x64-setup.exe.sig",
    "latest.json",
    "Bex.LaTeX.Studio_0.4.0_x64.dmg",
  ]) {
    assert.equal(classifyAsset(name), null, name);
  }
});

test("detectPlatform", () => {
  assert.equal(detectPlatform({ userAgent: UA.windowsChrome }), "windows");
  assert.equal(detectPlatform({ userAgent: UA.macSafari, maxTouchPoints: 0 }), "mac");
  assert.equal(detectPlatform({ userAgent: UA.macSafari, maxTouchPoints: 5 }), "mobile"); // iPad
  assert.equal(detectPlatform({ userAgent: UA.ubuntuFirefox }), "linux");
  assert.equal(detectPlatform({ userAgent: UA.androidChrome }), "mobile");
  assert.equal(detectPlatform({ userAgent: UA.iPhoneSafari }), "mobile");
  assert.equal(
    detectPlatform({ userAgent: UA.windowsChrome, userAgentData: { platform: "macOS", mobile: false } }),
    "mac",
  );
  assert.equal(detectPlatform({ userAgent: "" }), "unknown");
});

test("detectPlatform via userAgentData", () => {
  assert.equal(detectPlatform({ userAgent: "", userAgentData: { mobile: true } }), "mobile");
  assert.equal(detectPlatform({ userAgent: "", userAgentData: { platform: "Android" } }), "mobile");
  // Prototype keys must not be mistaken for known platforms.
  assert.equal(detectPlatform({ userAgent: "", userAgentData: { platform: "constructor" } }), "unknown");
});

test("formatSize uses MiB with one decimal", () => {
  assert.equal(formatSize(25645091), "24.5 MB");
  assert.equal(formatSize(89315832), "85.2 MB");
});

test("formatDate is UTC and English", () => {
  assert.equal(formatDate("2026-11-26T18:07:55Z"), "26 November 2026");
});

test("an unparseable date gives an empty string and a null model date", () => {
  assert.equal(formatDate("not a date"), "");
  assert.equal(buildDownloadModel({ ...RELEASE, published_at: "not a date" }, "mac").date, null);
});

test("model for macOS with the full release", () => {
  const m = buildDownloadModel(RELEASE, "mac");
  assert.equal(m.version, "0.3.9");
  assert.equal(m.date, "26 November 2026");
  assert.equal(m.primary.label, "Download for macOS");
  assert.equal(m.primary.note, "Apple Silicon only · .dmg · 24.5 MB");
  assert.equal(m.primary.href, DL + ASSETS["mac-dmg"].name);
  assert.deepEqual(Object.keys(m.rows).sort(), Object.keys(ASSETS).sort());
  for (const [key, { name, size }] of Object.entries(ASSETS)) {
    assert.deepEqual(m.rows[key], { href: DL + name, size }, key);
  }
});

test("model for Windows and Linux", () => {
  const win = buildDownloadModel(RELEASE, "windows");
  assert.equal(win.primary.label, "Download for Windows");
  assert.equal(win.primary.note, "64-bit installer · .exe · 15.4 MB");
  assert.equal(win.primary.href, DL + ASSETS["win-exe"].name);

  const linux = buildDownloadModel(RELEASE, "linux");
  assert.equal(linux.primary.label, "Download for Linux");
  assert.equal(linux.primary.note, "AppImage · 85.2 MB");
  assert.equal(linux.primary.href, DL + ASSETS["linux-appimage"].name);
});

test("model for mobile and unknown points at the downloads table", () => {
  assert.deepEqual(buildDownloadModel(RELEASE, "mobile").primary, MOBILE_PRIMARY);
  assert.deepEqual(buildDownloadModel(RELEASE, "unknown").primary, MOBILE_PRIMARY);
});

test("model without release data falls back to the releases page", () => {
  const m = buildDownloadModel(null, "mac");
  assert.equal(m.version, null);
  assert.equal(m.date, null);
  assert.deepEqual(m.rows, {});
  assert.equal(m.primary.href, RELEASES_URL);
  assert.equal(m.primary.label, "Download for macOS");
  assert.equal(m.primary.note, "Latest release on GitHub");
});

test("model omits rows for missing assets", () => {
  const release = {
    ...RELEASE,
    assets: RELEASE.assets.filter((a) => !a.name.endsWith(".rpm")),
  };
  const m = buildDownloadModel(release, "linux");
  assert.equal(m.rows["linux-rpm"], undefined);
  for (const key of ["mac-dmg", "win-exe", "win-msi", "linux-appimage", "linux-deb"]) {
    assert.ok(m.rows[key], key);
  }
});

test("model skips assets with a malformed URL or size", () => {
  const withoutUrl = {
    ...RELEASE,
    assets: RELEASE.assets.map((a) => {
      if (classifyAsset(a.name) !== "win-msi") return a;
      const { browser_download_url, ...rest } = a;
      return rest;
    }),
  };
  const m = buildDownloadModel(withoutUrl, "windows");
  assert.equal(m.rows["win-msi"], undefined);
  for (const key of ["mac-dmg", "win-exe", "linux-appimage", "linux-deb", "linux-rpm"]) {
    assert.ok(m.rows[key], key);
  }

  const badSize = {
    ...RELEASE,
    assets: RELEASE.assets.map((a) => (classifyAsset(a.name) === "mac-dmg" ? { ...a, size: "big" } : a)),
  };
  const mac = buildDownloadModel(badSize, "mac");
  assert.equal(mac.rows["mac-dmg"], undefined);
  assert.equal(mac.primary.href, RELEASES_URL);
  assert.equal(mac.primary.note, "Latest release on GitHub");
});
