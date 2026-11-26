<script lang="ts">
  import {
    CircleArrowUp,
    CircleCheck,
    CircleExclamationPoint,
    Download,
    LoaderCircle,
    RefreshCw,
    RotateCw,
    X,
  } from "@lucide/svelte";
  import type { AppUpdateInfo, UpdateProgress } from "../types";

  export type UpdateModalStatus =
    | "idle"
    | "checking"
    | "available"
    | "up-to-date"
    | "downloading"
    | "ready-to-restart"
    | "error";

  interface Props {
    open: boolean;
    status: UpdateModalStatus;
    currentVersion: string;
    updateInfo: AppUpdateInfo | null;
    progress: UpdateProgress | null;
    errorMessage: string | null;
    onCheckAgain: () => void;
    onDownloadAndInstall: () => void;
    onRelaunch: () => void;
    onClose: () => void;
  }

  let {
    open,
    status,
    currentVersion,
    updateInfo,
    progress,
    errorMessage,
    onCheckAgain,
    onDownloadAndInstall,
    onRelaunch,
    onClose,
  }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  const isDownloading = $derived(status === "downloading");

  const titleText = $derived.by(() => {
    switch (status) {
      case "checking":
        return "Checking for Updates";
      case "available":
        return "Update Available";
      case "up-to-date":
        return "Up to Date";
      case "downloading":
        return "Installing Update";
      case "ready-to-restart":
        return "Ready to Restart";
      case "error":
        return "Update Check Failed";
      default:
        return "Software Update";
    }
  });

  const displayCurrentVersion = $derived(
    updateInfo?.currentVersion || currentVersion
  );

  const formattedDate = $derived.by(() => {
    const raw = updateInfo?.date;
    if (!raw) return null;
    const ms = Date.parse(raw);
    if (Number.isNaN(ms)) return null;
    try {
      return new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(
        new Date(ms)
      );
    } catch {
      return null;
    }
  });

  const releaseNotesText = $derived(
    updateInfo?.body?.trim() || "New release available via GitHub Releases."
  );

  const progressDetailsText = $derived.by(() => {
    if (!progress) return "Connecting to release server...";
    if (progress.downloadedBytes === 0 && progress.contentLength === null) {
      return "Connecting to release server...";
    }
    const downloaded = formatBytes(progress.downloadedBytes);
    if (progress.contentLength && progress.contentLength > 0) {
      return `${downloaded} / ${formatBytes(progress.contentLength)}`;
    }
    return `${downloaded} downloaded`;
  });

  // Sync native showModal() / close() with `open` prop
  $effect(() => {
    if (!dialogEl) return;

    if (open && !dialogEl.open) {
      dialogEl.showModal();
    } else if (!open && dialogEl.open) {
      dialogEl.close();
    }
  });

  // Fallback light-dismiss only for browsers lacking native <dialog closedby> (e.g. WebKit)
  $effect(() => {
    const el = dialogEl;
    if (!el || "closedBy" in HTMLDialogElement.prototype) return;

    let pressedOutside = false;
    const isOutside = (e: PointerEvent) => {
      const r = el.getBoundingClientRect();
      return (
        e.clientX < r.left ||
        e.clientX > r.right ||
        e.clientY < r.top ||
        e.clientY > r.bottom
      );
    };

    const onPointerDown = (e: PointerEvent) => {
      pressedOutside = !isDownloading && isOutside(e);
    };
    const onPointerUp = (e: PointerEvent) => {
      if (!isDownloading && pressedOutside && isOutside(e)) {
        el.close();
      }
      pressedOutside = false;
    };

    el.addEventListener("pointerdown", onPointerDown);
    el.addEventListener("pointerup", onPointerUp);
    return () => {
      el.removeEventListener("pointerdown", onPointerDown);
      el.removeEventListener("pointerup", onPointerUp);
    };
  });

  function formatBytes(bytes: number): string {
    if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB"];
    let value = bytes;
    let unitIdx = 0;
    while (value >= 1024 && unitIdx < units.length - 1) {
      value /= 1024;
      unitIdx++;
    }
    return `${value >= 10 || unitIdx === 0 ? Math.round(value) : value.toFixed(1)} ${units[unitIdx]}`;
  }

  function handleDialogCancel(event: Event) {
    if (isDownloading) {
      event.preventDefault();
    }
  }

  function handleCloseClick() {
    if (isDownloading) return;
    dialogEl?.close("close");
  }

  function handleDialogClose() {
    if (!dialogEl) return;
    dialogEl.returnValue = "";
    if (open) {
      onClose();
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  class="bex-modal"
  closedby={isDownloading ? "none" : "any"}
  aria-labelledby="update-dialog-title"
  oncancel={handleDialogCancel}
  onclose={handleDialogClose}
>
  <div class="modal-surface">
    <header class="modal-header">
      <span
        class="modal-icon"
        class:success={status === "up-to-date" || status === "ready-to-restart"}
        class:danger={status === "error"}
        aria-hidden="true"
      >
        {#if status === "checking"}
          <RefreshCw size={18} />
        {:else if status === "available"}
          <CircleArrowUp size={18} />
        {:else if status === "up-to-date" || status === "ready-to-restart"}
          <CircleCheck size={18} />
        {:else if status === "downloading"}
          <Download size={18} />
        {:else if status === "error"}
          <CircleExclamationPoint size={18} />
        {:else}
          <RefreshCw size={18} />
        {/if}
      </span>

      <div class="modal-titles">
        <h1 id="update-dialog-title" class="modal-title">{titleText}</h1>
        <p class="modal-subtitle">
          Bex LaTeX Studio <code>v{displayCurrentVersion}</code>
        </p>
      </div>

      <button
        type="button"
        class="close-btn"
        aria-label="Close update dialog"
        disabled={isDownloading}
        onclick={handleCloseClick}
      >
        <X size={16} aria-hidden="true" />
      </button>
    </header>

    <div class="modal-body">
      {#if status === "checking"}
        <div class="status-card" role="status" aria-live="polite">
          <LoaderCircle size={18} class="spin-icon status-spinner" aria-hidden="true" />
          <div class="status-copy">
            <p class="status-primary">Checking GitHub Releases for updates...</p>
            <p class="status-secondary">
              Installed version: <code>v{displayCurrentVersion}</code>
            </p>
          </div>
        </div>
      {:else if status === "up-to-date"}
        <div class="status-card status-success" role="status">
          <CircleCheck size={18} class="status-icon-success" aria-hidden="true" />
          <p class="status-primary">
            Bex LaTeX Studio <code>v{displayCurrentVersion}</code> is up to date.
          </p>
        </div>
      {:else if status === "available" && updateInfo}
        <div class="version-banner">
          <div class="version-flow">
            <span class="version-chip">v{displayCurrentVersion}</span>
            <span class="version-arrow" aria-hidden="true">→</span>
            <span class="version-chip version-new">v{updateInfo.version}</span>
          </div>
          {#if formattedDate}
            <span class="release-date">Released {formattedDate}</span>
          {/if}
        </div>

        <div class="notes-section">
          <span class="notes-label" id="update-notes-label">Release Notes</span>
          <div
            class="release-notes scrollable-pane"
            role="region"
            aria-labelledby="update-notes-label"
          >
            {releaseNotesText}
          </div>
        </div>
      {:else if status === "downloading"}
        <div class="progress-card">
          <div class="progress-header">
            <span id="update-progress-label" class="progress-title">
              {progress?.percent === 100
                ? "Finalizing installation..."
                : `Downloading v${updateInfo?.version ?? "update"}...`}
            </span>
            {#if progress?.percent !== null && progress?.percent !== undefined}
              <span class="progress-percent">{progress.percent}%</span>
            {/if}
          </div>

          <progress
            class="update-progress"
            aria-labelledby="update-progress-label"
            max={100}
            value={progress?.percent ?? undefined}
          ></progress>

          <div class="progress-meta" role="status" aria-live="polite">
            <span>{progressDetailsText}</span>
          </div>
        </div>

        <p class="install-hint">
          Please keep Bex LaTeX Studio open until installation finishes.
        </p>
      {:else if status === "ready-to-restart"}
        <div class="status-card status-success" role="status">
          <CircleCheck size={18} class="status-icon-success" aria-hidden="true" />
          <div class="status-copy">
            <p class="status-primary">
              Update installed! Restart Bex LaTeX Studio to switch to the new version.
            </p>
            {#if updateInfo?.version}
              <p class="status-secondary">
                Ready to launch <code>v{updateInfo.version}</code>
              </p>
            {/if}
          </div>
        </div>
      {:else if status === "error"}
        <div class="error-banner" role="alert">
          <CircleExclamationPoint size={17} class="error-icon" aria-hidden="true" />
          <div class="error-copy">
            <p class="error-title">Could not complete the update check.</p>
            <p class="error-detail">
              {errorMessage || "GitHub Releases could not be reached right now."}
            </p>
          </div>
        </div>
      {:else}
        <p class="idle-copy">
          Check GitHub Releases to see if a newer version of Bex LaTeX Studio is available.
        </p>
      {/if}
    </div>

    <footer class="modal-actions">
      {#if status === "checking"}
        <!-- svelte-ignore a11y_autofocus -->
        <button
          type="button"
          class="btn btn-secondary"
          autofocus
          onclick={handleCloseClick}
        >
          Close
        </button>
      {:else if status === "up-to-date"}
        <button type="button" class="btn btn-secondary" onclick={onCheckAgain}>
          Check Again
        </button>
        <!-- svelte-ignore a11y_autofocus -->
        <button
          type="button"
          class="btn btn-primary"
          autofocus
          onclick={handleCloseClick}
        >
          Close
        </button>
      {:else if status === "available"}
        <button type="button" class="btn btn-secondary" onclick={handleCloseClick}>
          Later
        </button>
        <!-- svelte-ignore a11y_autofocus -->
        <button
          type="button"
          class="btn btn-primary"
          autofocus
          onclick={onDownloadAndInstall}
        >
          <Download size={14} aria-hidden="true" />
          <span>Download &amp; Install</span>
        </button>
      {:else if status === "downloading"}
        <button type="button" class="btn btn-secondary" disabled>
          <LoaderCircle size={14} class="spin-icon" aria-hidden="true" />
          <span>Installing...</span>
        </button>
      {:else if status === "ready-to-restart"}
        <button type="button" class="btn btn-secondary" onclick={handleCloseClick}>
          Later
        </button>
        <!-- svelte-ignore a11y_autofocus -->
        <button
          type="button"
          class="btn btn-primary"
          autofocus
          onclick={onRelaunch}
        >
          <RotateCw size={14} aria-hidden="true" />
          <span>Restart Now</span>
        </button>
      {:else if status === "error"}
        <button type="button" class="btn btn-secondary" onclick={handleCloseClick}>
          Close
        </button>
        <!-- svelte-ignore a11y_autofocus -->
        <button
          type="button"
          class="btn btn-primary"
          autofocus
          onclick={onCheckAgain}
        >
          <RefreshCw size={14} aria-hidden="true" />
          <span>Retry</span>
        </button>
      {:else}
        <button type="button" class="btn btn-secondary" onclick={handleCloseClick}>
          Close
        </button>
        <!-- svelte-ignore a11y_autofocus -->
        <button
          type="button"
          class="btn btn-primary"
          autofocus
          onclick={onCheckAgain}
        >
          Check for Updates
        </button>
      {/if}
    </footer>
  </div>
</dialog>

<style>
  .bex-modal {
    padding: 0;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: var(--shadow-dialog);
    inline-size: min(468px, calc(100% - 32px));
    max-block-size: calc(100dvb - 32px);
  }

  .bex-modal:not([open]) {
    display: none;
  }

  .modal-surface {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-header {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .modal-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 36px;
    block-size: 36px;
    border-radius: var(--radius-md);
    background: var(--accent-soft);
    color: var(--accent);
    flex-shrink: 0;
  }

  .modal-icon.success {
    background: var(--success-soft);
    color: var(--success);
  }

  .modal-icon.danger {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .modal-titles {
    min-inline-size: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .modal-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    line-height: 1.3;
    color: var(--text-primary);
  }

  .modal-subtitle {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .modal-subtitle code,
  .status-primary code,
  .status-secondary code {
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--text-secondary);
  }

  .close-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 28px;
    block-size: 28px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .close-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  @media (hover: hover) {
    .close-btn:not(:disabled):hover {
      background: var(--bg-hover);
      color: var(--text-primary);
    }
  }

  .modal-body {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-inline-size: 0;
  }

  .status-card {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }

  .status-card.status-success {
    border-color: color-mix(in oklab, var(--success) 35%, var(--border-subtle));
    background: var(--success-soft);
  }

  :global(.status-spinner) {
    color: var(--success);
    flex-shrink: 0;
    margin-top: 1px;
  }

  :global(.status-icon-success) {
    color: var(--success);
    flex-shrink: 0;
    margin-top: 1px;
  }

  .status-copy {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-inline-size: 0;
  }

  .status-primary {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.45;
  }

  .status-secondary {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .version-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }

  .version-flow {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }

  .version-chip {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 500;
  }

  .version-chip.version-new {
    border-color: transparent;
    background: var(--success-soft);
    color: var(--success);
    font-weight: 600;
  }

  .version-arrow {
    color: var(--text-muted);
    font-size: 12px;
  }

  .release-date {
    font-size: 11.5px;
    color: var(--text-muted);
  }

  .notes-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-inline-size: 0;
  }

  .notes-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .release-notes {
    max-block-size: 170px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 12.5px;
    line-height: 1.55;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .progress-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }

  .progress-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .progress-title {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .progress-percent {
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--success);
  }

  .update-progress {
    appearance: none;
    inline-size: 100%;
    block-size: 8px;
    border: 0 solid;
    overflow: clip;
    container-type: inline-size;
    --progress-bar-radius: 1e5px;
    --progress-bar-indeterminate-size: 30%;
    --progress-bar-indeterminate-animation-duration: 1.5s;
    border-radius: var(--progress-bar-radius);
    background: var(--bg-muted);
    box-shadow: inset 0 1px 2px rgb(0 0 0 / 0.1);
  }

  @media (prefers-reduced-motion: reduce) {
    .update-progress {
      --progress-bar-indeterminate-animation-duration: 10s;
    }
  }

  .update-progress:dir(rtl) {
    --progress-bar-indeterminate-animation: bex-progress-indeterminate
      var(--progress-bar-indeterminate-animation-duration) infinite linear reverse;
  }

  .update-progress:indeterminate {
    --progress-bar-animation: var(
      --progress-bar-indeterminate-animation,
      bex-progress-indeterminate var(--progress-bar-indeterminate-animation-duration)
        infinite linear
    );
  }

  .update-progress::-webkit-progress-bar {
    background: transparent;
  }

  .update-progress::-webkit-progress-value {
    background: var(--accent);
    border-radius: var(--progress-bar-radius);
    transition: inline-size 0.3s;
    animation: var(--progress-bar-animation, revert-rule);
  }

  .update-progress::-moz-progress-bar {
    background: var(--accent);
    border-radius: var(--progress-bar-radius);
    transition: inline-size 0.3s;
    animation: var(--progress-bar-animation, revert-rule);
  }

  @keyframes bex-progress-indeterminate {
    from,
    to {
      inline-size: var(--progress-bar-indeterminate-size);
      margin-right: auto;
    }
    from {
      translate: -100%;
    }
    to {
      translate: 100cqi;
    }
  }

  .progress-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    color: var(--text-muted);
  }

  .install-hint,
  .idle-copy {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .error-banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px;
    border-radius: var(--radius-md);
    border: 1px solid color-mix(in oklab, var(--danger) 35%, var(--border-subtle));
    background: var(--danger-soft);
  }

  :global(.error-icon) {
    color: var(--danger);
    flex-shrink: 0;
    margin-top: 1px;
  }

  .error-copy {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-inline-size: 0;
  }

  .error-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--danger);
  }

  .error-detail {
    margin: 0;
    font-size: 12px;
    color: var(--text-primary);
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  .modal-actions {
    margin-top: 2px;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-block-size: 32px;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background-color 120ms ease, border-color 120ms ease;
  }

  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--bg-muted);
    border-color: var(--border-subtle);
    color: var(--text-primary);
  }

  @media (hover: hover) {
    .btn-secondary:not(:disabled):hover {
      background: var(--bg-hover);
      border-color: var(--border-strong);
    }
  }

  .btn-primary {
    background: var(--accent);
    color: var(--accent-fg);
  }

  @media (hover: hover) {
    .btn-primary:not(:disabled):hover {
      background: var(--accent-hover);
    }
  }
</style>
