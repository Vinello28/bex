export const UI_ZOOM_STORAGE_KEY = "bex-ui-zoom";
export const UI_ZOOM_DEFAULT = 1.0;
export const UI_ZOOM_MIN = 0.7;
export const UI_ZOOM_MAX = 1.8;
export const UI_ZOOM_STEP = 0.1;

export type UiZoomAction = "in" | "out" | "reset";

/**
 * Clamps a UI zoom scale factor to `[UI_ZOOM_MIN, UI_ZOOM_MAX]` and rounds to
 * two decimal places to prevent floating-point drift. Falls back to
 * `UI_ZOOM_DEFAULT` for non-finite inputs.
 */
export function clampUiZoom(value: number): number {
  if (!Number.isFinite(value)) {
    return UI_ZOOM_DEFAULT;
  }
  const clamped = Math.min(UI_ZOOM_MAX, Math.max(UI_ZOOM_MIN, value));
  return Math.round(clamped * 100) / 100;
}

/**
 * Increments or decrements `current` by `UI_ZOOM_STEP` and clamps the result.
 */
export function stepUiZoom(current: number, direction: "in" | "out"): number {
  const delta = direction === "in" ? UI_ZOOM_STEP : -UI_ZOOM_STEP;
  return clampUiZoom(current + delta);
}

/**
 * Formats a UI zoom scale factor as a rounded percentage string (e.g. `"110%"`).
 */
export function formatUiZoomPercent(scale: number): string {
  return `${Math.round(clampUiZoom(scale) * 100)}%`;
}

/**
 * Loads the persisted UI zoom scale factor from `localStorage`, returning
 * `UI_ZOOM_DEFAULT` when missing, invalid, or storage is unavailable.
 */
export function loadUiZoom(): number {
  try {
    if (typeof localStorage === "undefined") {
      return UI_ZOOM_DEFAULT;
    }
    const raw = localStorage.getItem(UI_ZOOM_STORAGE_KEY);
    if (raw === null) {
      return UI_ZOOM_DEFAULT;
    }
    return clampUiZoom(Number.parseFloat(raw));
  } catch {
    return UI_ZOOM_DEFAULT;
  }
}

/**
 * Persists a clamped UI zoom scale factor to `localStorage`, ignoring any
 * storage errors.
 */
export function saveUiZoom(scale: number): void {
  try {
    if (typeof localStorage === "undefined") {
      return;
    }
    localStorage.setItem(UI_ZOOM_STORAGE_KEY, String(clampUiZoom(scale)));
  } catch {
    // Ignore storage errors (e.g. disabled or full storage).
  }
}

/**
 * Matches Cmd/Ctrl zoom shortcuts across keyboard layouts (including Italian
 * unshifted `+`/`-`, US `=`/`+`/`-`, and Numpad keys) while ignoring IME
 * composition and AltGr combinations.
 */
export function matchUiZoomShortcut(
  event: Pick<
    KeyboardEvent,
    "metaKey" | "ctrlKey" | "altKey" | "shiftKey" | "key" | "code" | "isComposing"
  >
): UiZoomAction | null {
  if (event.isComposing || event.altKey || !(event.metaKey || event.ctrlKey)) {
    return null;
  }

  if (
    event.key === "+" ||
    event.code === "NumpadAdd" ||
    (event.key === "=" && (!event.shiftKey || event.code === "Equal"))
  ) {
    return "in";
  }

  if (
    event.key === "-" ||
    event.code === "NumpadSubtract" ||
    (event.key === "_" && event.code === "Minus")
  ) {
    return "out";
  }

  if ((event.key === "0" && !event.shiftKey) || event.code === "Numpad0") {
    return "reset";
  }

  return null;
}
