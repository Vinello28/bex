import { toggleLineComment } from "@codemirror/commands";
import type { EditorState, Transaction } from "@codemirror/state";

/**
 * Matches keyboard shortcuts for toggling LaTeX `%` line comments:
 * - Italian keyboard layout: `Cmd/Ctrl + ù` (including CapsLock `Ù` and NFD-decomposed `u\u0300`, excluding `Shift+ù`)
 * - International layouts: `Cmd/Ctrl + /` (including `Shift+7` on Italian keyboards)
 *
 * Ignores IME composition and `Alt`/`AltGr` combinations.
 */
export function isCommentToggleShortcut(
  event: Pick<KeyboardEvent, "metaKey" | "ctrlKey" | "altKey" | "shiftKey" | "key" | "isComposing">
): boolean {
  if (event.isComposing || event.altKey || !(event.metaKey || event.ctrlKey)) {
    return false;
  }

  const normalized = event.key.normalize("NFC").toLowerCase();
  if (normalized === "ù" && !event.shiftKey) {
    return true;
  }

  if (event.key === "/") {
    return true;
  }

  return false;
}

/**
 * Toggles `% ` line comments on the current line or all selected lines using CodeMirror's
 * language `commentTokens` (`{ line: "%" }`). Returns `false` if the state is read-only.
 */
export function toggleLatexComment(target: {
  state: EditorState;
  dispatch: (tr: Transaction) => void;
}): boolean {
  if (target.state.readOnly) {
    return false;
  }
  return toggleLineComment(target);
}

function stripLineComment(line: string): string {
  let escaped = false;
  for (let i = 0; i < line.length; i++) {
    const ch = line[i];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (ch === "\\") {
      escaped = true;
    } else if (ch === "%") {
      return line.slice(0, i);
    }
  }
  return line;
}

/**
 * Strips unescaped `%...` comments from each line of a LaTeX source string while preserving `\%`.
 */
export function stripLatexComments(source: string): string {
  const parts = source.split(/(\r?\n)/);
  for (let i = 0; i < parts.length; i += 2) {
    parts[i] = stripLineComment(parts[i]);
  }
  return parts.join("");
}
