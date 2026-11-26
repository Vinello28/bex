<script lang="ts">
  import {
    PanelLeftClose,
    PanelLeftOpen,
    FolderOpen,
    FilePlus,
    FolderPlus,
    Save,
    Download,
    Play,
    LoaderCircle,
    CircleCheck,
    CircleExclamationPoint,
    TriangleExclamationPoint,
    ChevronDown,
    Settings,
    Home,
    Sparkles,
  } from "@lucide/svelte";
  import type { LatexDiagnostic } from "../types";

  interface Props {
    projectName: string;
    sidebarOpen: boolean;
    onToggleSidebar: () => void;
    workspaceRoot: string | null;
    entryFile: string | null;
    isCompiling: boolean;
    compileSuccess: boolean | null;
    lastCompileMs: number | null;
    diagnostics: LatexDiagnostic[];
    isDirty: boolean;
    hasAnyDirty: boolean;
    hasPdf: boolean;
    availableUpdateVersion: string | null;
    aiSidebarOpen?: boolean;
    onToggleAiSidebar?: () => void;
    onGoHome?: () => void;
    onOpenFolder: () => void;
    onNewFile: () => void;
    onNewFolder: () => void;
    onSave: () => void;
    onRecompile: () => void;
    onExportPdf: () => void;
    onToggleLogs: () => void;
    onOpenSettings: () => void;
  }

  let {
    projectName,
    sidebarOpen,
    onToggleSidebar,
    workspaceRoot,
    entryFile,
    isCompiling,
    compileSuccess,
    lastCompileMs,
    diagnostics,
    isDirty,
    hasAnyDirty,
    hasPdf,
    availableUpdateVersion,
    aiSidebarOpen = false,
    onToggleAiSidebar,
    onGoHome,
    onOpenFolder,
    onNewFile,
    onNewFolder,
    onSave,
    onRecompile,
    onExportPdf,
    onToggleLogs,
    onOpenSettings,
  }: Props = $props();

  let fileMenuTriggerEl: HTMLButtonElement | undefined = $state();
  let fileMenuEl: HTMLDivElement | undefined = $state();
  let fileMenuOpen = $state(false);

  const isMac =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/i.test(navigator.platform || navigator.userAgent);
  const modKey = isMac ? "⌘" : "Ctrl+";

  const errorCount = $derived(
    diagnostics.filter((d) => d.severity === "error").length
  );
  const warningCount = $derived(
    diagnostics.filter((d) => d.severity === "warning").length
  );

  function closeFileMenu() {
    try {
      fileMenuEl?.hidePopover?.();
    } catch {
      // Ignore if already closed
    }
  }

  function handleFileMenuAction(action: () => void) {
    closeFileMenu();
    action();
  }

  function handleFileMenuToggle(event: ToggleEvent) {
    const isOpen = event.newState === "open";
    fileMenuOpen = isOpen;

    // Fallback coordinate positioning only when CSS anchor positioning is unavailable
    if (
      isOpen &&
      fileMenuEl &&
      fileMenuTriggerEl &&
      !CSS.supports("position-area", "block-end")
    ) {
      const t = fileMenuTriggerEl.getBoundingClientRect();
      const m = fileMenuEl.getBoundingClientRect();
      const below = t.bottom + 4 + m.height <= window.innerHeight;
      fileMenuEl.style.top = `${below ? t.bottom + 4 : Math.max(4, t.top - m.height - 4)}px`;
      fileMenuEl.style.left = `${Math.max(4, Math.min(t.left, window.innerWidth - m.width - 4))}px`;
    }
  }
</script>

<header class="topbar">
  <div class="topbar-left">
    <button
      type="button"
      class="icon-btn"
      aria-label={sidebarOpen ? "Collapse project sidebar" : "Expand project sidebar"}
      aria-pressed={sidebarOpen}
      title={`${sidebarOpen ? "Hide" : "Show"} Sidebar (${modKey}B)`}
      onclick={onToggleSidebar}
    >
      {#if sidebarOpen}
        <PanelLeftClose size={16} aria-hidden="true" />
      {:else}
        <PanelLeftOpen size={16} aria-hidden="true" />
      {/if}
    </button>

    <button
      type="button"
      class="brand-btn"
      title="Welcome Screen / Projects"
      aria-label="Welcome Screen / Projects"
      onclick={() => onGoHome?.()}
    >
      <svg
        class="brand-icon"
        width="28"
        height="28"
        viewBox="0 0 64 64"
        aria-hidden="true"
      >
        <defs>
          <linearGradient id="bex-brand-bg" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#1b1815" />
            <stop offset="55%" stop-color="#120f0c" />
            <stop offset="100%" stop-color="#0b0a09" />
          </linearGradient>
          <radialGradient id="bex-brand-glow" cx="50%" cy="54%" r="48%">
            <stop offset="0%" stop-color="#f59e0b" stop-opacity="0.32" />
            <stop offset="56%" stop-color="#b45309" stop-opacity="0.14" />
            <stop offset="100%" stop-color="#0b0a09" stop-opacity="0" />
          </radialGradient>
          <linearGradient id="bex-brand-amber" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stop-color="#78350f" />
            <stop offset="14%" stop-color="#d97706" />
            <stop offset="35%" stop-color="#fbbf24" />
            <stop offset="56%" stop-color="#f59e0b" />
            <stop offset="84%" stop-color="#b45309" />
            <stop offset="100%" stop-color="#6c2708" />
          </linearGradient>
          <linearGradient id="bex-brand-beer-glow" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stop-color="#fef08a" stop-opacity="0.58" />
            <stop offset="36%" stop-color="#fbbf24" stop-opacity="0.20" />
            <stop offset="78%" stop-color="#78350f" stop-opacity="0.10" />
            <stop offset="100%" stop-color="#451a03" stop-opacity="0.64" />
          </linearGradient>
          <linearGradient id="bex-brand-base-shade" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stop-color="#451a03" stop-opacity="0" />
            <stop offset="55%" stop-color="#451a03" stop-opacity="0.52" />
            <stop offset="100%" stop-color="#2a0e02" stop-opacity="0.88" />
          </linearGradient>
          <linearGradient id="bex-brand-lip" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stop-color="#78350f" />
            <stop offset="22%" stop-color="#f59e0b" />
            <stop offset="48%" stop-color="#fcd34d" />
            <stop offset="78%" stop-color="#d97706" />
            <stop offset="100%" stop-color="#6c2708" />
          </linearGradient>
          <linearGradient id="bex-brand-shine" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stop-color="#fffbeb" stop-opacity="0.72" />
            <stop offset="50%" stop-color="#fef08a" stop-opacity="0.38" />
            <stop offset="100%" stop-color="#fde68a" stop-opacity="0.10" />
          </linearGradient>
          <linearGradient id="bex-brand-foam" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stop-color="#ffffff" />
            <stop offset="58%" stop-color="#fffbeb" />
            <stop offset="88%" stop-color="#fef3c7" />
            <stop offset="100%" stop-color="#fde68a" />
          </linearGradient>
          <linearGradient id="bex-brand-foam-under" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stop-color="#fef3c7" stop-opacity="0" />
            <stop offset="65%" stop-color="#fde68a" stop-opacity="0.75" />
            <stop offset="100%" stop-color="#f59e0b" stop-opacity="0.45" />
          </linearGradient>
          <linearGradient id="bex-brand-sclera" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stop-color="#ffffff" />
            <stop offset="72%" stop-color="#ffffff" />
            <stop offset="100%" stop-color="#fef3c7" />
          </linearGradient>
          <linearGradient id="bex-brand-iris" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stop-color="#2a1205" />
            <stop offset="60%" stop-color="#1c0a02" />
            <stop offset="100%" stop-color="#451a03" />
          </linearGradient>
          <clipPath id="bex-brand-bottle-clip">
            <path d="M 24.6 14.6 L 24.2 18.6 C 23.2 23.8, 11.2 27.6, 11.1 34.4 C 11.0 40.5, 11.0 46.2, 11.5 52.0 C 11.8 56.2, 14.0 58.4, 18.4 58.6 Q 32.0 59.3, 45.6 58.6 C 50.0 58.4, 52.2 56.2, 52.5 52.0 C 53.0 46.2, 53.0 40.5, 52.9 34.4 C 52.8 27.6, 40.8 23.8, 39.8 18.6 L 39.4 14.6 Z" />
          </clipPath>
        </defs>
        <rect x="2" y="2" width="60" height="60" rx="14" fill="url(#bex-brand-bg)" />
        <rect x="2" y="2" width="60" height="60" rx="14" fill="url(#bex-brand-glow)" />
        <rect x="2.5" y="2.5" width="59" height="59" rx="13.5" fill="none" stroke="#f59e0b" stroke-opacity="0.35" stroke-width="1" />
        <ellipse cx="32" cy="59.1" rx="16.5" ry="1.6" fill="#050302" fill-opacity="0.72" />
        <ellipse cx="32" cy="12.2" rx="8.8" ry="1.9" fill="#78350f" stroke="#d97706" stroke-opacity="0.58" stroke-width="0.6" />
        <g clip-path="url(#bex-brand-bottle-clip)">
          <rect x="10" y="14" width="44" height="46" fill="url(#bex-brand-amber)" />
          <rect x="10" y="14" width="44" height="46" fill="url(#bex-brand-beer-glow)" />
          <rect x="10" y="53.4" width="44" height="6.6" fill="url(#bex-brand-base-shade)" />
          <path d="M 21.0 57.6 Q 32 55.2 43.0 57.6" fill="#2a0e02" fill-opacity="0.45" />
          <path d="M 16.5 55.6 Q 32 58.2 47.5 55.6" fill="none" stroke="#fbbf24" stroke-opacity="0.34" stroke-width="1.05" stroke-linecap="round" />
          <path d="M 23.2 14.8 Q 32 16.2 40.8 14.8 L 40.8 17.0 Q 32 18.4 23.2 17.0 Z" fill="#78350f" fill-opacity="0.45" />
          <path d="M 25.7 18.8 C 24.8 24.0, 13.5 27.8, 13.4 34.5 L 13.5 51.0 C 13.5 52.1, 15.6 52.1, 15.6 51.0 L 15.5 34.5 C 15.6 29.0, 26.0 24.8, 27.0 18.8 Z" fill="url(#bex-brand-shine)" />
          <path d="M 26.2 19.3 C 25.3 24.3, 14.3 28.0, 14.2 34.5 L 14.3 47.6" fill="none" stroke="#ffffff" stroke-opacity="0.56" stroke-width="0.9" stroke-linecap="round" />
          <path d="M 50.8 34.6 L 50.6 51.2" fill="none" stroke="#fbbf24" stroke-opacity="0.42" stroke-width="1.05" stroke-linecap="round" />
          <circle cx="15.6" cy="51.0" r="1.35" fill="#fef9c3" fill-opacity="0.68" />
          <circle cx="18.8" cy="54.3" r="0.95" fill="#fef08a" fill-opacity="0.58" />
          <circle cx="14.3" cy="47.4" r="0.8" fill="#ffffff" fill-opacity="0.48" />
          <circle cx="27.2" cy="53.5" r="1.15" fill="#fef9c3" fill-opacity="0.62" />
          <circle cx="35.6" cy="52.0" r="1.4" fill="#fef08a" fill-opacity="0.66" />
          <circle cx="31.5" cy="55.1" r="0.85" fill="#ffffff" fill-opacity="0.52" />
          <circle cx="47.3" cy="50.6" r="1.25" fill="#fef9c3" fill-opacity="0.65" />
          <circle cx="44.2" cy="54.4" r="0.95" fill="#fef08a" fill-opacity="0.56" />
          <circle cx="48.9" cy="46.8" r="0.8" fill="#ffffff" fill-opacity="0.45" />
          <path d="M 23.6 14.6 C 24.6 18.6, 25.0 20.6, 26.5 20.6 C 28.1 20.6, 28.6 17.6, 29.6 17.4 C 30.6 17.4, 30.6 24.0, 32.5 24.0 C 34.4 24.0, 34.7 17.8, 35.7 17.6 C 36.7 17.6, 36.7 21.5, 38.3 21.5 C 40.0 21.5, 40.0 18.0, 40.8 14.6 Z" fill="#78350f" fill-opacity="0.34" />
        </g>
        <path
          d="M 24.6 14.6 L 24.2 18.6 C 23.2 23.8, 11.2 27.6, 11.1 34.4 C 11.0 40.5, 11.0 46.2, 11.5 52.0 C 11.8 56.2, 14.0 58.4, 18.4 58.6 Q 32.0 59.3, 45.6 58.6 C 50.0 58.4, 52.2 56.2, 52.5 52.0 C 53.0 46.2, 53.0 40.5, 52.9 34.4 C 52.8 27.6, 40.8 23.8, 39.8 18.6 L 39.4 14.6 Z"
          fill="none"
          stroke="#fbbf24"
          stroke-opacity="0.45"
          stroke-width="0.9"
        />
        <path
          d="M 23.2 11.9 L 40.8 11.9 C 42.1 11.9, 42.4 15.0, 40.6 15.1 Q 32.0 15.9, 23.4 15.1 C 21.6 15.0, 21.9 11.9, 23.2 11.9 Z"
          fill="url(#bex-brand-lip)"
          stroke="#451a03"
          stroke-width="0.85"
        />
        <path d="M 23.1 12.9 L 26.0 12.9" fill="none" stroke="#fef08a" stroke-opacity="0.72" stroke-width="0.8" stroke-linecap="round" />
        <path d="M 38.2 12.9 L 40.9 12.9" fill="none" stroke="#fbbf24" stroke-opacity="0.48" stroke-width="0.7" stroke-linecap="round" />
        <path
          d="M 22.0 12.9 C 18.8 12.3, 18.4 8.2, 21.9 7.6 C 22.9 4.9, 27.1 4.6, 29.0 6.2 C 30.8 3.6, 36.2 3.8, 37.5 6.8 C 41.2 6.0, 44.8 8.7, 43.4 11.9 C 43.8 13.1, 42.2 13.8, 40.2 13.7 C 39.4 15.9, 39.8 19.3, 38.1 20.1 C 36.4 20.8, 36.2 17.5, 35.6 16.3 C 35.0 15.5, 34.2 22.3, 32.3 22.5 C 30.4 22.7, 30.4 17.1, 29.5 16.1 C 28.6 15.3, 27.6 19.3, 26.0 19.1 C 24.3 18.9, 24.6 14.7, 23.6 13.7 C 22.7 13.7, 22.1 13.4, 22.0 12.9 Z"
          fill="url(#bex-brand-foam)"
          stroke="#b45309"
          stroke-opacity="0.38"
          stroke-width="0.7"
          stroke-linejoin="round"
        />
        <path
          d="M 23.6 13.7 C 24.6 14.7, 24.3 18.9, 26.0 19.1 C 27.6 19.3, 28.6 15.3, 29.5 16.1 C 30.4 17.1, 30.4 22.7, 32.3 22.5 C 34.2 22.3, 35.0 15.5, 35.6 16.3 C 36.2 17.5, 36.4 20.8, 38.1 20.1 C 39.8 19.3, 39.4 15.9, 40.2 13.7 L 23.6 13.7 Z"
          fill="url(#bex-brand-foam-under)"
        />
        <path d="M 23.0 8.0 C 24.1 6.2, 26.4 5.8, 28.0 6.6" fill="none" stroke="#ffffff" stroke-width="1.05" stroke-linecap="round" />
        <path d="M 30.8 5.6 C 32.4 4.6, 34.7 5.0, 36.0 6.5" fill="none" stroke="#ffffff" stroke-width="1.15" stroke-linecap="round" />
        <circle cx="20.7" cy="6.1" r="1.0" fill="#fffbeb" />
        <circle cx="42.7" cy="5.8" r="1.15" fill="#fffbeb" />
        <circle cx="44.9" cy="8.2" r="0.75" fill="#fef3c7" />
        <ellipse cx="18.4" cy="43.3" rx="2.9" ry="1.5" fill="#ea580c" fill-opacity="0.38" />
        <ellipse cx="18.0" cy="42.9" rx="1.15" ry="0.55" fill="#fef08a" fill-opacity="0.34" />
        <ellipse cx="45.6" cy="43.3" rx="2.9" ry="1.5" fill="#ea580c" fill-opacity="0.38" />
        <ellipse cx="45.2" cy="42.9" rx="1.15" ry="0.55" fill="#fef08a" fill-opacity="0.34" />
        <path d="M 27.8 44.3 Q 32.0 48.3 36.2 44.3" fill="none" stroke="#2a1205" stroke-width="1.8" stroke-linecap="round" />
        <ellipse cx="24.2" cy="35.9" rx="5.8" ry="7.3" fill="#78350f" fill-opacity="0.34" />
        <ellipse cx="39.8" cy="35.9" rx="5.8" ry="7.3" fill="#78350f" fill-opacity="0.34" />
        <ellipse cx="24.2" cy="35.2" rx="5.8" ry="7.3" fill="url(#bex-brand-sclera)" stroke="#2a1205" stroke-width="1.35" />
        <ellipse cx="24.9" cy="35.7" rx="3.9" ry="5.1" fill="url(#bex-brand-iris)" />
        <ellipse cx="24.9" cy="38.0" rx="2.7" ry="1.8" fill="#d97706" fill-opacity="0.48" />
        <ellipse cx="24.9" cy="35.5" rx="2.35" ry="3.0" fill="#140601" />
        <circle cx="23.1" cy="32.9" r="1.85" fill="#ffffff" />
        <circle cx="26.5" cy="38.1" r="0.82" fill="#ffffff" fill-opacity="0.92" />
        <ellipse cx="39.8" cy="35.2" rx="5.8" ry="7.3" fill="url(#bex-brand-sclera)" stroke="#2a1205" stroke-width="1.35" />
        <ellipse cx="39.1" cy="35.7" rx="3.9" ry="5.1" fill="url(#bex-brand-iris)" />
        <ellipse cx="39.1" cy="38.0" rx="2.7" ry="1.8" fill="#d97706" fill-opacity="0.48" />
        <ellipse cx="39.1" cy="35.5" rx="2.35" ry="3.0" fill="#140601" />
        <circle cx="37.3" cy="32.9" r="1.85" fill="#ffffff" />
        <circle cx="40.7" cy="38.1" r="0.82" fill="#ffffff" fill-opacity="0.92" />
      </svg>
      <span class="brand-badge" aria-hidden="true">Bex</span>
    </button>

    <div class="project-pill" title={workspaceRoot ?? projectName}>
      <span class="project-name">
        {projectName}
      </span>
    </div>

    <div class="file-menu-wrap">
      <button
        bind:this={fileMenuTriggerEl}
        type="button"
        class="menu-trigger"
        popovertarget="bex-file-menu"
        aria-expanded={fileMenuOpen}
      >
        <span>File</span>
        <ChevronDown size={13} aria-hidden="true" />
      </button>

      <div
        bind:this={fileMenuEl}
        id="bex-file-menu"
        popover="auto"
        class="file-popover"
        ontoggle={handleFileMenuToggle}
      >
        <button
          type="button"
          class="popover-item"
          onclick={() => handleFileMenuAction(() => onGoHome?.())}
        >
          <Home size={14} aria-hidden="true" />
          <span>Welcome Screen</span>
        </button>
        <button
          type="button"
          class="popover-item"
          onclick={() => handleFileMenuAction(onOpenFolder)}
        >
          <FolderOpen size={14} aria-hidden="true" />
          <span>Open Project Folder...</span>
        </button>
        <button
          type="button"
          class="popover-item"
          disabled={!workspaceRoot}
          onclick={() => handleFileMenuAction(onNewFile)}
        >
          <FilePlus size={14} aria-hidden="true" />
          <span>New File...</span>
        </button>
        <button
          type="button"
          class="popover-item"
          disabled={!workspaceRoot}
          onclick={() => handleFileMenuAction(onNewFolder)}
        >
          <FolderPlus size={14} aria-hidden="true" />
          <span>New Folder...</span>
        </button>
        <hr class="popover-sep" />
        <button
          type="button"
          class="popover-item"
          disabled={!workspaceRoot}
          onclick={() => handleFileMenuAction(onSave)}
        >
          <Save size={14} aria-hidden="true" />
          <span>Save</span>
          <span class="shortcut">{modKey}S</span>
        </button>
        <button
          type="button"
          class="popover-item"
          disabled={!hasPdf}
          onclick={() => handleFileMenuAction(onExportPdf)}
        >
          <Download size={14} aria-hidden="true" />
          <span>Export PDF...</span>
        </button>
      </div>
    </div>
  </div>

  <div class="topbar-right">
    {#if isDirty || hasAnyDirty}
      <button
        type="button"
        class="save-btn"
        title={`Save changes (${modKey}S)`}
        onclick={onSave}
      >
        <Save size={14} aria-hidden="true" />
        <span>Save</span>
      </button>
    {/if}

    <button
      type="button"
      class="compile-btn"
      disabled={isCompiling || !entryFile}
      title={entryFile
        ? `Recompile ${entryFile} (${modKey}S)`
        : "Select a .tex entry file in Settings first"}
      onclick={onRecompile}
    >
      {#if isCompiling}
        <LoaderCircle size={14} class="spin-icon" aria-hidden="true" />
        <span>Compiling...</span>
      {:else}
        <Play size={14} aria-hidden="true" />
        <span>Recompile</span>
      {/if}
    </button>

    {#if compileSuccess !== null || errorCount > 0 || warningCount > 0}
      <button
        type="button"
        class="status-pill"
        class:status-ok={compileSuccess === true && errorCount === 0}
        class:status-warn={compileSuccess === true && errorCount === 0 && warningCount > 0}
        class:status-error={compileSuccess === false || errorCount > 0}
        title="Click to toggle compiler logs and diagnostics"
        onclick={onToggleLogs}
      >
        {#if compileSuccess === false || errorCount > 0}
          <CircleExclamationPoint size={13} aria-hidden="true" />
          <span>{errorCount > 0 ? `${errorCount} err` : "Failed"}</span>
        {:else if warningCount > 0}
          <TriangleExclamationPoint size={13} aria-hidden="true" />
          <span>{warningCount} warn</span>
        {:else}
          <CircleCheck size={13} aria-hidden="true" />
          <span>OK</span>
        {/if}
        {#if lastCompileMs !== null}
          <span class="compile-ms">{lastCompileMs} ms</span>
        {/if}
      </button>
    {/if}

    <button
      type="button"
      class="ai-toggle-btn"
      class:active={aiSidebarOpen}
      aria-pressed={aiSidebarOpen}
      title={`AI Assistant (${modKey}⇧A)`}
      onclick={onToggleAiSidebar}
    >
      <Sparkles size={14} aria-hidden="true" />
      <span class="ai-label">AI</span>
    </button>

    <button
      type="button"
      class="icon-btn settings-btn"
      aria-label={availableUpdateVersion
        ? `Settings (update v${availableUpdateVersion} available)`
        : "Settings"}
      title={availableUpdateVersion
        ? `Settings (${modKey},) — Update v${availableUpdateVersion} available`
        : `Settings (${modKey},)`}
      onclick={onOpenSettings}
    >
      <Settings size={16} aria-hidden="true" />
      {#if availableUpdateVersion}
        <span class="update-dot" aria-hidden="true"></span>
      {/if}
    </button>
  </div>
</header>

<style>
  .topbar {
    block-size: 46px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding-inline: 10px;
    background: var(--bg-app);
    border-bottom: none;
    user-select: none;
  }

  .topbar-left,
  .topbar-right {
    display: flex;
    align-items: center;
    gap: 8px;
    min-inline-size: 0;
  }

  .topbar-right {
    flex-shrink: 0;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-inline-size: 30px;
    min-block-size: 30px;
    padding: 4px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  @media (hover: hover) {
    .icon-btn:hover {
      background: var(--bg-hover);
      color: var(--text-primary);
    }
  }

  .brand-btn {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 3px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: transparent;
    cursor: pointer;
    transition: background-color 120ms ease;
  }

  @media (hover: hover) {
    .brand-btn:hover {
      background: var(--bg-hover);
    }
  }

  .brand-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .brand-icon {
    inline-size: 28px;
    block-size: 28px;
    flex-shrink: 0;
    display: block;
  }

  .brand-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px 7px;
    border-radius: var(--radius-xs);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.03em;
    flex-shrink: 0;
  }

  .project-pill {
    display: inline-flex;
    align-items: center;
    min-block-size: 26px;
    padding: 0 8px;
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    max-inline-size: 200px;
  }

  .project-name {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ai-toggle-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-block-size: 28px;
    padding: 0 9px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 120ms ease, border-color 120ms ease, color 120ms ease;
  }

  @media (hover: hover) {
    .ai-toggle-btn:hover {
      background: var(--bg-hover);
      color: var(--text-primary);
      border-color: var(--border-strong);
    }
  }

  .ai-toggle-btn.active {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }

  .ai-label {
    letter-spacing: 0.02em;
  }

  .file-menu-wrap {
    position: relative;
  }

  .menu-trigger {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-block-size: 28px;
    padding: 4px 9px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }

  @media (hover: hover) {
    .menu-trigger:hover {
      background: var(--bg-hover);
      border-color: var(--border-strong);
    }
  }

  .file-popover {
    padding: 5px;
    min-inline-size: 210px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: var(--shadow-popover);
  }

  @supports (position-area: block-end) {
    .file-popover {
      inset: auto;
      margin: 4px 0 0 0;
      position-area: block-end span-inline-end;
      position-try-fallbacks: flip-block, flip-inline, flip-block flip-inline;
    }
  }

  @supports not (position-area: block-end) {
    .file-popover {
      position: fixed;
      inset: auto;
      margin: 0;
    }
  }

  .file-popover:is(:popover-open, :global(.\:popover-open)) {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .popover-item {
    display: flex;
    align-items: center;
    gap: 8px;
    inline-size: 100%;
    padding: 7px 10px;
    border: none;
    border-radius: var(--radius-xs);
    background: transparent;
    color: var(--text-primary);
    font-size: 12.5px;
    text-align: start;
    cursor: pointer;
  }

  .popover-item:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  @media (hover: hover) {
    .popover-item:not(:disabled):hover {
      background: var(--bg-hover);
    }
  }

  .popover-item .shortcut {
    margin-inline-start: auto;
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .popover-sep {
    inline-size: 100%;
    block-size: 1px;
    margin: 4px 0;
    border: none;
    background: var(--border-subtle);
  }

  .save-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    block-size: 28px;
    padding: 0 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-strong);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
  }

  @media (hover: hover) {
    .save-btn:hover {
      background: var(--bg-hover);
    }
  }

  .compile-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    block-size: 30px;
    padding: 0 13px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background-color 120ms ease;
  }

  .compile-btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  @media (hover: hover) {
    .compile-btn:not(:disabled):hover {
      background: var(--accent-hover);
    }
  }

  :global(.spin-icon) {
    animation: bex-spin 0.9s linear infinite;
  }

  @keyframes bex-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    block-size: 28px;
    padding: 0 9px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-secondary);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }

  .status-pill.status-ok {
    background: var(--success-soft);
    color: var(--success);
    border-color: transparent;
  }

  .status-pill.status-warn {
    background: var(--warning-soft);
    color: var(--warning);
    border-color: transparent;
  }

  .status-pill.status-error {
    background: var(--danger-soft);
    color: var(--danger);
    border-color: transparent;
  }

  .compile-ms {
    opacity: 0.85;
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .settings-btn {
    position: relative;
  }

  .update-dot {
    position: absolute;
    top: 4px;
    right: 4px;
    inline-size: 7px;
    block-size: 7px;
    border-radius: 50%;
    background: var(--success);
    box-shadow: 0 0 0 2px var(--bg-surface);
    pointer-events: none;
  }
</style>
