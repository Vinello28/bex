import type { RecentProject, RecentFile } from "./types";

const RECENT_PROJECTS_KEY = "bex-recent-projects";
const RECENT_FILES_KEY = "bex-recent-files";
const MAX_PROJECTS = 12;
const MAX_FILES = 16;

/**
 * Normalizes directory or file paths by removing trailing slashes/backslashes
 * and replacing backslashes with forward slashes for cross-platform consistency.
 */
function normalizePath(rawPath: string): string {
  if (!rawPath) return "";
  let clean = rawPath.trim().replace(/\\/g, "/");
  while (clean.length > 1 && clean.endsWith("/")) {
    clean = clean.slice(0, -1);
  }
  return clean;
}

/**
 * Safely loads recent projects from localStorage.
 */
export function loadRecentProjects(): RecentProject[] {
  try {
    if (typeof window === "undefined" || !window.localStorage) {
      return [];
    }
    const raw = window.localStorage.getItem(RECENT_PROJECTS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];

    return parsed
      .filter((p): p is RecentProject => Boolean(p && typeof p.path === "string" && typeof p.name === "string"))
      .map((p) => ({
        path: normalizePath(p.path),
        name: p.name,
        lastOpened: typeof p.lastOpened === "number" ? p.lastOpened : Date.now(),
        entryFile: p.entryFile ?? null,
        lastActiveFile: p.lastActiveFile ?? null,
      }))
      .sort((a, b) => b.lastOpened - a.lastOpened)
      .slice(0, MAX_PROJECTS);
  } catch (err) {
    console.error("Failed to load recent projects:", err);
    return [];
  }
}

/**
 * Records a recent project, updating its lastOpened timestamp, entryFile, and lastActiveFile.
 * Deduplicates by normalized path, keeps newest first, up to 12 items.
 */
export function recordRecentProject(
  project: Omit<RecentProject, "lastOpened">
): RecentProject[] {
  try {
    const existing = loadRecentProjects();
    const normPath = normalizePath(project.path);
    const now = Date.now();

    const previousIdx = existing.findIndex(
      (p) => normalizePath(p.path).toLowerCase() === normPath.toLowerCase()
    );

    let updatedEntryFile = project.entryFile;
    let updatedLastActive = project.lastActiveFile;

    if (previousIdx >= 0) {
      const prev = existing[previousIdx];
      if (updatedEntryFile === undefined) {
        updatedEntryFile = prev.entryFile;
      }
      if (updatedLastActive === undefined) {
        updatedLastActive = prev.lastActiveFile;
      }
      existing.splice(previousIdx, 1);
    }

    const updatedProject: RecentProject = {
      path: project.path,
      name: project.name || normPath.split("/").pop() || "Project",
      lastOpened: now,
      entryFile: updatedEntryFile ?? null,
      lastActiveFile: updatedLastActive ?? null,
    };

    const nextList = [updatedProject, ...existing].slice(0, MAX_PROJECTS);

    if (typeof window !== "undefined" && window.localStorage) {
      window.localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(nextList));
    }
    return nextList;
  } catch (err) {
    console.error("Failed to record recent project:", err);
    return loadRecentProjects();
  }
}

/**
 * Removes a recent project by path from localStorage.
 */
export function removeRecentProject(path: string): RecentProject[] {
  try {
    const existing = loadRecentProjects();
    const normTarget = normalizePath(path).toLowerCase();
    const filtered = existing.filter(
      (p) => normalizePath(p.path).toLowerCase() !== normTarget
    );

    if (typeof window !== "undefined" && window.localStorage) {
      window.localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(filtered));
    }
    return filtered;
  } catch (err) {
    console.error("Failed to remove recent project:", err);
    return loadRecentProjects();
  }
}

/**
 * Safely loads recent files from localStorage.
 */
export function loadRecentFiles(): RecentFile[] {
  try {
    if (typeof window === "undefined" || !window.localStorage) {
      return [];
    }
    const raw = window.localStorage.getItem(RECENT_FILES_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];

    return parsed
      .filter((f): f is RecentFile =>
        Boolean(
          f &&
          typeof f.workspacePath === "string" &&
          typeof f.relPath === "string" &&
          typeof f.fileName === "string"
        )
      )
      .map((f) => ({
        workspacePath: normalizePath(f.workspacePath),
        workspaceName: f.workspaceName || "Project",
        relPath: normalizePath(f.relPath),
        fileName: f.fileName,
        lastOpened: typeof f.lastOpened === "number" ? f.lastOpened : Date.now(),
      }))
      .sort((a, b) => b.lastOpened - a.lastOpened)
      .slice(0, MAX_FILES);
  } catch (err) {
    console.error("Failed to load recent files:", err);
    return [];
  }
}

/**
 * Records a recent file, updating its lastOpened timestamp.
 * Deduplicates by normalized workspacePath + relPath, newest first, max 16.
 */
export function recordRecentFile(
  file: Omit<RecentFile, "lastOpened">
): RecentFile[] {
  try {
    const existing = loadRecentFiles();
    const normWorkspace = normalizePath(file.workspacePath).toLowerCase();
    const normRel = normalizePath(file.relPath).toLowerCase();
    const key = `${normWorkspace}::${normRel}`;
    const now = Date.now();

    const filtered = existing.filter(
      (f) =>
        `${normalizePath(f.workspacePath).toLowerCase()}::${normalizePath(f.relPath).toLowerCase()}` !== key
    );

    const updatedFile: RecentFile = {
      workspacePath: file.workspacePath,
      workspaceName: file.workspaceName,
      relPath: file.relPath,
      fileName: file.fileName,
      lastOpened: now,
    };

    const nextList = [updatedFile, ...filtered].slice(0, MAX_FILES);

    if (typeof window !== "undefined" && window.localStorage) {
      window.localStorage.setItem(RECENT_FILES_KEY, JSON.stringify(nextList));
    }
    return nextList;
  } catch (err) {
    console.error("Failed to record recent file:", err);
    return loadRecentFiles();
  }
}

/**
 * Removes a recent file by workspacePath and relPath.
 */
export function removeRecentFile(workspacePath: string, relPath: string): RecentFile[] {
  try {
    const existing = loadRecentFiles();
    const normWorkspace = normalizePath(workspacePath).toLowerCase();
    const normRel = normalizePath(relPath).toLowerCase();
    const key = `${normWorkspace}::${normRel}`;

    const filtered = existing.filter(
      (f) =>
        `${normalizePath(f.workspacePath).toLowerCase()}::${normalizePath(f.relPath).toLowerCase()}` !== key
    );

    if (typeof window !== "undefined" && window.localStorage) {
      window.localStorage.setItem(RECENT_FILES_KEY, JSON.stringify(filtered));
    }
    return filtered;
  } catch (err) {
    console.error("Failed to remove recent file:", err);
    return loadRecentFiles();
  }
}

/**
 * Formats a timestamp into human-readable relative time:
 * "Just now", "5m ago", "2h ago", "Yesterday", "3d ago", or short date.
 */
export function formatRelativeTime(timestamp: number): string {
  if (!timestamp || isNaN(timestamp)) return "";
  const diffMs = Date.now() - timestamp;
  if (diffMs < 0) return "Just now";

  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffMin < 1) return "Just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay === 1) return "Yesterday";
  if (diffDay < 7) return `${diffDay}d ago`;

  const date = new Date(timestamp);
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}

/**
 * Generates a 2-letter uppercase initials monogram from a project name.
 */
export function getProjectMonogram(name: string): string {
  if (!name) return "PR";
  const cleaned = name.replace(/[^a-zA-Z0-9\s_-]/g, "").trim();
  if (!cleaned) return "PR";

  // Split on whitespace, underscores, hyphens, or camelCase boundaries
  const words = cleaned
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .split(/[\s_\-]+/)
    .filter(Boolean);

  if (words.length >= 2) {
    return (words[0][0] + words[1][0]).toUpperCase();
  }
  if (words[0].length >= 2) {
    return words[0].slice(0, 2).toUpperCase();
  }
  return (words[0][0] + "P").toUpperCase();
}

/**
 * Deterministic palette of 6 warm, neutral, botanical tones with strong contrast.
 */
export interface AvatarColor {
  bg: string;
  fg: string;
}

const AVATAR_PALETTE: AvatarColor[] = [
  // 1. Botanical Forest Emerald
  { bg: "#166534", fg: "#f0fdf4" },
  // 2. Warm Amber Copper
  { bg: "#b45309", fg: "#fffbeb" },
  // 3. Deep Mediterranean Indigo
  { bg: "#1e40af", fg: "#eff6ff" },
  // 4. Botanical Terracotta Wine
  { bg: "#991b1b", fg: "#fef2f2" },
  // 5. Rich Plum Aubergine
  { bg: "#6b21a8", fg: "#faf5ff" },
  // 6. Deep Olive Sage
  { bg: "#3f6212", fg: "#f7fee7" },
];

/**
 * Deterministically derives an avatar color pair from project name.
 */
export function getProjectAvatarColor(name: string): AvatarColor {
  const str = (name || "Project").toLowerCase();
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i);
    hash |= 0;
  }
  const index = Math.abs(hash) % AVATAR_PALETTE.length;
  return AVATAR_PALETTE[index];
}

/**
 * Shortens a full path replacing /Users/<name> or C:\Users\<name> with ~.
 */
export function shortenPath(fullPath: string): string {
  if (!fullPath) return "";
  let p = fullPath.replace(/\\/g, "/");

  // Unix/macOS: /Users/<username> or /home/<username>
  p = p.replace(/^\/(?:Users|home)\/[^/]+(?:\/|$)/, "~/");

  // Windows: C:/Users/<username>
  p = p.replace(/^[a-zA-Z]:\/Users\/[^/]+(?:\/|$)/i, "~/");

  // Clean trailing slash if not root ~ or /
  if (p.length > 2 && p.endsWith("/")) {
    p = p.slice(0, -1);
  }
  return p;
}
