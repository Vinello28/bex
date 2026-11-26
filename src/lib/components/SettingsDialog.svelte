<script module lang="ts">
  export type { UpdateStatus } from "../types";
</script>

<script lang="ts">
  import {
    Bot,
    CircleArrowUp,
    CircleCheck,
    CircleExclamationPoint,
    Cpu,
    Download,
    Eye,
    EyeOff,
    FolderGit2,
    Info,
    LoaderCircle,
    Minus,
    Monitor,
    Moon,
    Palette,
    Plus,
    RefreshCw,
    RotateCcw,
    RotateCw,
    Settings,
    Sparkles,
    Sun,
    Trash2,
    X,
  } from "@lucide/svelte";
  import {
    formatUiZoomPercent,
    UI_ZOOM_DEFAULT,
    UI_ZOOM_MAX,
    UI_ZOOM_MIN,
  } from "../uiZoom";
  import type {
    AiEnvStatus,
    AiProviderId,
    AiSettings,
    AppUpdateInfo,
    CompileCacheInfo,
    CompilerInfo,
    SettingsSectionId,
    ThemeMode,
    UpdateProgress,
    UpdateStatus,
  } from "../types";

  interface Props {
    open: boolean;
    workspaceName: string | null;
    workspaceRoot: string | null;
    texFiles: string[];
    entryFile: string | null;
    onChangeEntryFile: (path: string) => void;
    compilers: CompilerInfo[];
    selectedEngine: string;
    onChangeEngine: (engine: string) => void;
    autoCompileOnSave: boolean;
    onToggleAutoCompile: (enabled: boolean) => void;
    theme: ThemeMode;
    onChangeTheme: (theme: ThemeMode) => void;
    uiZoom?: number;
    onZoomIn?: () => void;
    onZoomOut?: () => void;
    onResetZoom?: () => void;
    appVersion: string;
    updateStatus: UpdateStatus;
    updateInfo: AppUpdateInfo | null;
    updateProgress: UpdateProgress | null;
    updateError: string | null;
    onCheckUpdates: () => void;
    onDownloadAndInstall: () => void;
    onRelaunch: () => void;
    onClose: () => void;
    // Compilation cache props
    compileCacheInfo?: CompileCacheInfo | null;
    isLoadingCacheInfo?: boolean;
    clearingCacheScope?: "project" | "tectonic" | "all" | null;
    cacheNotice?: string | null;
    onRefreshCacheInfo?: () => void;
    onClearCompileCache?: (scope: "project" | "tectonic" | "all") => void;
    // AI and MCP section props
    initialSection?: SettingsSectionId | null;
    aiSettings?: AiSettings;
    aiEnvStatus?: AiEnvStatus;
    onChangeAiSettings?: (next: AiSettings) => void;
    onTestAiConnection?: (
      settings: AiSettings
    ) => Promise<{ ok: boolean; models: string[]; error?: string }>;
  }

  const defaultAiSettings: AiSettings = {
    provider: "google",
    baseUrl: "https://generativelanguage.googleapis.com/v1beta",
    apiKey: "",
    model: "gemini-2.5-flash",
    autoCompileAfterAiEdit: true,
  };

  const defaultAiEnvStatus: AiEnvStatus = {
    hasGoogleEnvKey: false,
    defaultGoogleModel: "gemini-2.5-flash",
  };

  let {
    open,
    workspaceName,
    workspaceRoot,
    texFiles,
    entryFile,
    onChangeEntryFile,
    compilers,
    selectedEngine,
    onChangeEngine,
    autoCompileOnSave,
    onToggleAutoCompile,
    theme,
    onChangeTheme,
    uiZoom = UI_ZOOM_DEFAULT,
    onZoomIn,
    onZoomOut,
    onResetZoom,
    appVersion,
    updateStatus,
    updateInfo,
    updateProgress,
    updateError,
    onCheckUpdates,
    onDownloadAndInstall,
    onRelaunch,
    onClose,
    compileCacheInfo = null,
    isLoadingCacheInfo = false,
    clearingCacheScope = null,
    cacheNotice = null,
    onRefreshCacheInfo,
    onClearCompileCache,
    initialSection = null,
    aiSettings = defaultAiSettings,
    aiEnvStatus = defaultAiEnvStatus,
    onChangeAiSettings = () => {},
    onTestAiConnection,
  }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let systemDark = $state(false);
  let showApiKey = $state(false);
  let isTestingConnection = $state(false);
  let testResult = $state<{
    ok: boolean;
    models: string[];
    error?: string;
  } | null>(null);

  const isMac =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/i.test(navigator.platform || navigator.userAgent);
  const modKey = isMac ? "⌘" : "Ctrl+";

  const isDownloading = $derived(updateStatus === "downloading");
  const displayCurrentVersion = $derived(
    updateInfo?.currentVersion || appVersion
  );

  const providerDefaults: Record<
    AiProviderId,
    { baseUrl: string; model: string }
  > = {
    google: {
      baseUrl: "https://generativelanguage.googleapis.com/v1beta",
      model: "gemini-2.5-flash",
    },
    ollama: {
      baseUrl: "http://localhost:11434/v1",
      model: "llama3.2",
    },
    lmstudio: {
      baseUrl: "http://localhost:1234/v1",
      model: "local-model",
    },
    vllm: {
      baseUrl: "http://localhost:8000/v1",
      model: "default",
    },
  };

  const engineOptions = $derived.by(() => {
    const map = new Map(compilers.map((c) => [c.id, c]));
    const defs = [
      { id: "auto", baseLabel: "Auto (Embedded Tectonic Rust)" },
      { id: "tectonic", baseLabel: "Tectonic (Embedded Rust)" },
      { id: "pdflatex", baseLabel: "pdfLaTeX" },
      { id: "xelatex", baseLabel: "XeLaTeX" },
      { id: "lualatex", baseLabel: "LuaLaTeX" },
    ];
    return defs.map((d) => {
      if (d.id === "auto") {
        const tectonic = map.get("tectonic");
        const suffix =
          tectonic && !tectonic.available ? " [fallback]" : "";
        return {
          id: d.id,
          label: `${d.baseLabel}${suffix}`,
          available: true,
        };
      }
      const info = map.get(d.id);
      const available = info ? info.available : true;
      const suffix = info
        ? info.available
          ? ""
          : " (missing)"
        : "";
      return {
        id: d.id,
        label: `${d.baseLabel}${suffix}`,
        available,
      };
    });
  });

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
    if (!updateProgress) return "Connecting to release server...";
    if (
      updateProgress.downloadedBytes === 0 &&
      updateProgress.contentLength === null
    ) {
      return "Connecting to release server...";
    }
    const downloaded = formatBytes(updateProgress.downloadedBytes);
    if (updateProgress.contentLength && updateProgress.contentLength > 0) {
      return `${downloaded} / ${formatBytes(updateProgress.contentLength)}`;
    }
    return `${downloaded} downloaded`;
  });

  // Track OS color scheme so the System theme option reflects current resolved state
  $effect(() => {
    if (typeof window === "undefined") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    systemDark = mq.matches;
    const handleMediaChange = (e: MediaQueryListEvent) => {
      systemDark = e.matches;
    };
    mq.addEventListener("change", handleMediaChange);
    return () => mq.removeEventListener("change", handleMediaChange);
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

  // When `open` becomes true and `initialSection` is set, scroll matching section into view cleanly
  $effect(() => {
    if (open && initialSection && dialogEl) {
      requestAnimationFrame(() => {
        const target = dialogEl?.querySelector(
          `#settings-section-${initialSection}`
        );
        if (target) {
          target.scrollIntoView({ behavior: "smooth", block: "start" });
        }
      });
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

  function formatRelativeTex(fullPath: string): string {
    if (!workspaceRoot) return fullPath;
    const normRoot = workspaceRoot.replace(/[\\/]+$/, "");
    if (
      fullPath.startsWith(normRoot + "/") ||
      fullPath.startsWith(normRoot + "\\")
    ) {
      return fullPath.slice(normRoot.length + 1);
    }
    return fullPath;
  }

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

  function formatFileCount(count: number): string {
    return `${count} ${count === 1 ? "file" : "files"}`;
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

  function handleProviderChange(provider: AiProviderId) {
    const defaults = providerDefaults[provider];
    onChangeAiSettings({
      ...aiSettings,
      provider,
      baseUrl: defaults.baseUrl,
      model: defaults.model,
    });
    testResult = null;
  }

  async function handleTestAiConnection() {
    if (!onTestAiConnection) return;
    isTestingConnection = true;
    testResult = null;
    try {
      testResult = await onTestAiConnection(aiSettings);
    } catch (err) {
      testResult = {
        ok: false,
        models: [],
        error: err instanceof Error ? err.message : String(err),
      };
    } finally {
      isTestingConnection = false;
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  class="bex-settings-dialog"
  closedby={isDownloading ? "none" : "any"}
  aria-labelledby="settings-dialog-title"
  oncancel={handleDialogCancel}
  onclose={handleDialogClose}
>
  <header class="settings-header">
    <span class="header-icon" aria-hidden="true">
      <Settings size={18} />
    </span>

    <div class="header-titles">
      <h1 id="settings-dialog-title" class="dialog-title">Settings</h1>
      <p class="dialog-subtitle">Preferences, updates, and app information</p>
    </div>

    <!-- svelte-ignore a11y_autofocus -->
    <button
      type="button"
      class="close-btn"
      aria-label="Close settings"
      autofocus
      disabled={isDownloading}
      onclick={handleCloseClick}
    >
      <X size={16} aria-hidden="true" />
    </button>
  </header>

  <div class="settings-body scrollable-pane">
    <!-- Compilation Section -->
    <section
      id="settings-section-compilation"
      class="settings-section"
      aria-labelledby="settings-section-compilation-title"
    >
      <div class="section-heading">
        <Cpu size={14} aria-hidden="true" />
        <h2 id="settings-section-compilation-title">Compilation</h2>
      </div>

      <div class="section-card">
        <div class="field-row">
          <div class="field-info">
            <label class="field-label" for="settings-entry-file">Main TeX file</label>
            <p class="field-hint">Root document passed to the LaTeX compiler</p>
          </div>
          <select
            id="settings-entry-file"
            class="field-select mono-select"
            value={entryFile ?? ""}
            disabled={texFiles.length === 0}
            onchange={(e) =>
              onChangeEntryFile((e.currentTarget as HTMLSelectElement).value)}
          >
            {#if texFiles.length === 0}
              <option value="">No .tex files</option>
            {:else}
              {#each texFiles as texPath (texPath)}
                <option value={texPath}>{formatRelativeTex(texPath)}</option>
              {/each}
            {/if}
          </select>
        </div>

        <hr class="row-sep" />

        <div class="field-col">
          <div class="field-row">
            <div class="field-info">
              <label class="field-label" for="settings-compiler-engine">
                LaTeX compiler engine
              </label>
              <p class="field-hint">
                Select embedded Tectonic or an installed TeX distribution
              </p>
            </div>
            <select
              id="settings-compiler-engine"
              class="field-select"
              value={selectedEngine}
              onchange={(e) =>
                onChangeEngine((e.currentTarget as HTMLSelectElement).value)}
            >
              {#each engineOptions as opt (opt.id)}
                <option
                  value={opt.id}
                  disabled={!opt.available && opt.id !== selectedEngine}
                >
                  {opt.label}
                </option>
              {/each}
            </select>
          </div>

          {#if compilers.length > 0}
            <ul class="compiler-pills" aria-label="Detected compilers">
              {#each compilers as comp (comp.id)}
                <li
                  class="compiler-pill"
                  class:available={comp.available}
                  title={comp.executablePath ?? comp.name}
                >
                  <span class="compiler-dot" aria-hidden="true"></span>
                  <span class="compiler-name">{comp.name}</span>
                  {#if comp.version}
                    <span class="compiler-ver">{comp.version}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </div>

        <hr class="row-sep" />

        <label class="toggle-row" for="settings-auto-compile">
          <div class="field-info">
            <span class="field-label">Auto-compile on Save</span>
            <p class="field-hint">
              Automatically recompile PDF when saving files ({modKey}S)
            </p>
          </div>
          <input
            id="settings-auto-compile"
            type="checkbox"
            class="native-checkbox"
            checked={autoCompileOnSave}
            onchange={(e) =>
              onToggleAutoCompile((e.currentTarget as HTMLInputElement).checked)}
          />
        </label>

        <hr class="row-sep" />

        <div class="cache-section">
          <div class="cache-header">
            <div class="cache-header-left">
              <span class="field-label">Compilation Cache</span>
              <span class="cache-total-badge">
                {isLoadingCacheInfo
                  ? "..."
                  : formatBytes(compileCacheInfo?.totalBytes ?? 0)}
              </span>
            </div>
            <button
              type="button"
              class="btn btn-secondary cache-refresh-btn"
              disabled={isLoadingCacheInfo || clearingCacheScope !== null}
              aria-label="Refresh compilation cache info"
              title="Refresh cache sizes"
              onclick={() => onRefreshCacheInfo?.()}
            >
              <RefreshCw
                size={12}
                class={isLoadingCacheInfo ? "spin-icon" : undefined}
                aria-hidden="true"
              />
              <span>Refresh</span>
            </button>
          </div>

          <div class="cache-row">
            <div class="cache-row-info">
              <div class="cache-row-title-line">
                <span class="cache-row-title">
                  Project build cache <code>.bex-build</code>
                </span>
                <span
                  class="cache-meta"
                  title={compileCacheInfo?.projectCache.path ?? undefined}
                >
                  {#if !workspaceRoot}
                    No project open
                  {:else if isLoadingCacheInfo && !compileCacheInfo}
                    ...
                  {:else}
                    {formatBytes(compileCacheInfo?.projectCache.bytes ?? 0)} · {formatFileCount(compileCacheInfo?.projectCache.fileCount ?? 0)}
                  {/if}
                </span>
              </div>
              <p class="field-hint">
                Incremental <code>.aux</code>, <code>.toc</code>, <code>.bbl</code> &amp; SyncTeX cache for faster builds
              </p>
            </div>
            <button
              type="button"
              class="btn btn-secondary cache-clear-btn"
              disabled={!workspaceRoot ||
                !compileCacheInfo?.projectCache.exists ||
                compileCacheInfo.projectCache.bytes === 0 ||
                clearingCacheScope !== null}
              onclick={() => onClearCompileCache?.("project")}
            >
              {#if clearingCacheScope === "project" || clearingCacheScope === "all"}
                <LoaderCircle size={12} class="spin-icon" aria-hidden="true" />
              {:else}
                <Trash2 size={12} aria-hidden="true" />
              {/if}
              <span>Clear</span>
            </button>
          </div>

          <div class="cache-row">
            <div class="cache-row-info">
              <div class="cache-row-title-line">
                <span class="cache-row-title">Tectonic engine cache</span>
                <span
                  class="cache-meta"
                  title={compileCacheInfo?.tectonicCache.path ?? undefined}
                >
                  {#if isLoadingCacheInfo && !compileCacheInfo}
                    ...
                  {:else}
                    {formatBytes(compileCacheInfo?.tectonicCache.bytes ?? 0)} · {formatFileCount(compileCacheInfo?.tectonicCache.fileCount ?? 0)}
                  {/if}
                </span>
              </div>
              <p class="field-hint">
                Cached LaTeX formats &amp; packages (re-downloaded if needed on next compile)
              </p>
            </div>
            <button
              type="button"
              class="btn btn-secondary cache-clear-btn"
              disabled={!compileCacheInfo?.tectonicCache.exists ||
                compileCacheInfo.tectonicCache.bytes === 0 ||
                clearingCacheScope !== null}
              onclick={() => onClearCompileCache?.("tectonic")}
            >
              {#if clearingCacheScope === "tectonic" || clearingCacheScope === "all"}
                <LoaderCircle size={12} class="spin-icon" aria-hidden="true" />
              {:else}
                <Trash2 size={12} aria-hidden="true" />
              {/if}
              <span>Clear</span>
            </button>
          </div>

          {#if cacheNotice}
            <div class="status-banner status-compact" role="status" aria-live="polite">
              <Info size={14} class="status-icon-info" aria-hidden="true" />
              <p class="status-primary">{cacheNotice}</p>
            </div>
          {/if}
        </div>
      </div>
    </section>

    <!-- Appearance Section -->
    <section
      id="settings-section-appearance"
      class="settings-section"
      aria-labelledby="settings-section-appearance-title"
    >
      <div class="section-heading">
        <Palette size={14} aria-hidden="true" />
        <h2 id="settings-section-appearance-title">Appearance</h2>
      </div>

      <div class="section-card">
        <fieldset class="theme-fieldset">
          <legend class="sr-only">Color theme</legend>
          <div class="theme-grid">
            <label
              class="theme-card"
              class:selected={theme === "system"}
            >
              <input
                type="radio"
                class="sr-only"
                name="bex-theme"
                value="system"
                checked={theme === "system"}
                onchange={() => onChangeTheme("system")}
              />
              <Monitor size={15} aria-hidden="true" />
              <span class="theme-name">System</span>
              <span class="theme-sub">
                Currently {systemDark ? "Dark" : "Light"}
              </span>
            </label>

            <label
              class="theme-card"
              class:selected={theme === "light"}
            >
              <input
                type="radio"
                class="sr-only"
                name="bex-theme"
                value="light"
                checked={theme === "light"}
                onchange={() => onChangeTheme("light")}
              />
              <Sun size={15} aria-hidden="true" />
              <span class="theme-name">Light</span>
              <span class="theme-sub">Islands Light</span>
            </label>

            <label
              class="theme-card"
              class:selected={theme === "dark"}
            >
              <input
                type="radio"
                class="sr-only"
                name="bex-theme"
                value="dark"
                checked={theme === "dark"}
                onchange={() => onChangeTheme("dark")}
              />
              <Moon size={15} aria-hidden="true" />
              <span class="theme-name">Dark</span>
              <span class="theme-sub">Islands Dark</span>
            </label>
          </div>
        </fieldset>

        <hr class="row-sep" />

        <div class="field-row">
          <div class="field-info">
            <span id="settings-ui-zoom-label" class="field-label">Interface Scale</span>
            <p class="field-hint">
              Zoom the entire interface ({modKey}+ / {modKey}- / {modKey}0)
            </p>
          </div>
          <div
            class="zoom-controls"
            role="group"
            aria-labelledby="settings-ui-zoom-label"
          >
            <button
              type="button"
              class="zoom-step-btn"
              aria-label="Zoom out interface"
              title={`Zoom Out (${modKey}-)`}
              disabled={uiZoom <= UI_ZOOM_MIN}
              onclick={() => onZoomOut?.()}
            >
              <Minus size={14} aria-hidden="true" />
            </button>
            <span class="zoom-value">{formatUiZoomPercent(uiZoom)}</span>
            <button
              type="button"
              class="zoom-step-btn"
              aria-label="Zoom in interface"
              title={`Zoom In (${modKey}+)`}
              disabled={uiZoom >= UI_ZOOM_MAX}
              onclick={() => onZoomIn?.()}
            >
              <Plus size={14} aria-hidden="true" />
            </button>
            <button
              type="button"
              class="zoom-reset-btn"
              aria-label="Reset interface scale to 100%"
              title={`Reset Zoom (${modKey}0)`}
              disabled={Math.abs(uiZoom - UI_ZOOM_DEFAULT) < 0.001}
              onclick={() => onResetZoom?.()}
            >
              <RotateCcw size={12} aria-hidden="true" />
              <span>Reset</span>
            </button>
          </div>
        </div>
      </div>
    </section>

    <!-- AI Agent & MCP Section -->
    <section
      id="settings-section-ai"
      class="settings-section"
      aria-labelledby="settings-section-ai-title"
    >
      <div class="section-heading">
        <Sparkles size={14} aria-hidden="true" />
        <h2 id="settings-section-ai-title">AI Agent &amp; MCP</h2>
      </div>

      <div class="section-card">
        <!-- Provider Selector -->
        <div class="field-row">
          <div class="field-info">
            <label class="field-label" for="settings-ai-provider">
              AI Provider
            </label>
            <p class="field-hint">
              Select Google AI Studio (Gemini) or a local inference server
            </p>
          </div>
          <select
            id="settings-ai-provider"
            class="field-select"
            value={aiSettings.provider}
            onchange={(e) =>
              handleProviderChange(
                (e.currentTarget as HTMLSelectElement).value as AiProviderId
              )}
          >
            <option value="google">Google AI Studio (Gemini)</option>
            <option value="ollama">Ollama (Local)</option>
            <option value="lmstudio">LM Studio (Local)</option>
            <option value="vllm">vLLM / OpenAI-Compatible</option>
          </select>
        </div>

        <hr class="row-sep" />

        <!-- Endpoint Base URL -->
        <div class="field-row">
          <div class="field-info">
            <label class="field-label" for="settings-ai-baseurl">
              Endpoint Base URL
            </label>
            <p class="field-hint">
              API endpoint used for completions and model discovery
            </p>
          </div>
          <input
            id="settings-ai-baseurl"
            type="url"
            class="field-input mono-input"
            value={aiSettings.baseUrl}
            placeholder="https://..."
            oninput={(e) =>
              onChangeAiSettings({
                ...aiSettings,
                baseUrl: (e.currentTarget as HTMLInputElement).value,
              })}
          />
        </div>

        <hr class="row-sep" />

        <!-- API Key with show/hide toggle -->
        <div class="field-col">
          <div class="field-row">
            <div class="field-info">
              <label class="field-label" for="settings-ai-apikey">
                API Key
              </label>
              <p class="field-hint">
                {#if aiSettings.provider === "google"}
                  Google AI Studio key. Stored locally in settings.
                {:else}
                  Optional for local Ollama / LM Studio / vLLM instances
                {/if}
              </p>
            </div>
            <div class="input-with-button">
              <input
                id="settings-ai-apikey"
                type={showApiKey ? "text" : "password"}
                class="field-input mono-input"
                value={aiSettings.apiKey}
                placeholder={aiSettings.provider === "google"
                  ? "Enter AI Studio API Key..."
                  : "Optional API Key"}
                oninput={(e) =>
                  onChangeAiSettings({
                    ...aiSettings,
                    apiKey: (e.currentTarget as HTMLInputElement).value,
                  })}
              />
              <button
                type="button"
                class="input-action-btn"
                onclick={() => (showApiKey = !showApiKey)}
                title={showApiKey ? "Hide key" : "Show key"}
                aria-label={showApiKey ? "Hide API key" : "Show API key"}
              >
                {#if showApiKey}
                  <EyeOff size={14} aria-hidden="true" />
                {:else}
                  <Eye size={14} aria-hidden="true" />
                {/if}
              </button>
            </div>
          </div>

          <!-- Environment Key Banner for Google -->
          {#if aiSettings.provider === "google" && !aiSettings.apiKey.trim() && aiEnvStatus.hasGoogleEnvKey}
            <div class="status-banner status-success status-compact" role="status">
              <CircleCheck size={14} class="status-icon-success" aria-hidden="true" />
              <p class="status-primary">
                Using <code>GOOGLE_AI_STUDIO</code> from local <code>.env</code> (enter a key to override)
              </p>
            </div>
          {/if}
        </div>

        <hr class="row-sep" />

        <!-- Model Name + Test & Fetch Models Button -->
        <div class="field-col">
          <div class="field-row">
            <div class="field-info">
              <label class="field-label" for="settings-ai-model">Model</label>
              <p class="field-hint">
                Model identifier passed to the AI provider
              </p>
            </div>
            <div class="input-with-button model-input-group">
              <input
                id="settings-ai-model"
                type="text"
                class="field-input mono-input"
                value={aiSettings.model}
                placeholder="e.g. gemini-2.5-flash"
                oninput={(e) =>
                  onChangeAiSettings({
                    ...aiSettings,
                    model: (e.currentTarget as HTMLInputElement).value,
                  })}
              />
              <button
                type="button"
                class="btn btn-secondary test-btn"
                disabled={isTestingConnection}
                onclick={handleTestAiConnection}
              >
                {#if isTestingConnection}
                  <LoaderCircle
                    size={13}
                    class="spin-icon"
                    aria-hidden="true"
                  />
                  <span>Testing...</span>
                {:else}
                  <RefreshCw size={12} aria-hidden="true" />
                  <span>Test &amp; Fetch</span>
                {/if}
              </button>
            </div>
          </div>

          <!-- Test Connection Feedback & Model Chips -->
          {#if testResult}
            {#if testResult.ok}
              <div class="status-banner status-success status-compact" role="status">
                <CircleCheck size={14} class="status-icon-success" aria-hidden="true" />
                <p class="status-primary">
                  Connected successfully!
                  {#if testResult.models.length > 0}
                    Found {testResult.models.length} model{testResult.models.length === 1 ? "" : "s"}.
                  {/if}
                </p>
              </div>

              {#if testResult.models.length > 0}
                <div class="models-picker">
                  <span class="models-picker-label">Available Models (click to select):</span>
                  <div class="model-chips-list">
                    {#each testResult.models as modelId (modelId)}
                      <button
                        type="button"
                        class="model-chip-btn"
                        class:selected={aiSettings.model === modelId}
                        onclick={() =>
                          onChangeAiSettings({ ...aiSettings, model: modelId })}
                      >
                        {modelId}
                      </button>
                    {/each}
                  </div>
                </div>
              {/if}
            {:else}
              <div class="error-banner status-compact" role="alert">
                <CircleExclamationPoint size={14} class="error-icon" aria-hidden="true" />
                <div class="error-copy">
                  <p class="error-title">Connection failed</p>
                  <p class="error-detail">
                    {testResult.error || "Could not reach endpoint or authenticate."}
                  </p>
                </div>
              </div>
            {/if}
          {/if}
        </div>

        <hr class="row-sep" />

        <!-- Auto-compile after AI MCP edits -->
        <label class="toggle-row" for="settings-ai-autocompile">
          <div class="field-info">
            <span class="field-label">
              Auto-compile LaTeX after AI MCP edits
            </span>
            <p class="field-hint">
              Automatically trigger PDF compilation whenever Bex AI writes or edits files
            </p>
          </div>
          <input
            id="settings-ai-autocompile"
            type="checkbox"
            class="native-checkbox"
            checked={aiSettings.autoCompileAfterAiEdit}
            onchange={(e) =>
              onChangeAiSettings({
                ...aiSettings,
                autoCompileAfterAiEdit: (e.currentTarget as HTMLInputElement).checked,
              })}
          />
        </label>
      </div>
    </section>

    <!-- Software Updates Section -->
    <section
      id="settings-section-updates"
      class="settings-section"
      aria-labelledby="settings-section-updates-title"
    >
      <div class="section-heading-row">
        <div class="section-heading">
          <CircleArrowUp size={14} aria-hidden="true" />
          <h2 id="settings-section-updates-title">Software Updates</h2>
        </div>
        <span class="version-pill">v{displayCurrentVersion}</span>
      </div>

      <div class="section-card update-card">
        {#if updateStatus === "checking"}
          <div class="status-row" role="status" aria-live="polite">
            <LoaderCircle
              size={17}
              class="spin-icon status-spinner"
              aria-hidden="true"
            />
            <div class="status-copy">
              <p class="status-primary">Checking GitHub Releases for updates...</p>
              <p class="status-secondary">
                Installed version: <code>v{displayCurrentVersion}</code>
              </p>
            </div>
          </div>
        {:else if updateStatus === "up-to-date"}
          <div class="status-banner status-success" role="status">
            <CircleCheck size={17} class="status-icon-success" aria-hidden="true" />
            <p class="status-primary">
              Bex LaTeX Studio <code>v{displayCurrentVersion}</code> is up to date.
            </p>
          </div>
          <div class="update-actions">
            <button
              type="button"
              class="btn btn-secondary"
              onclick={onCheckUpdates}
            >
              <RefreshCw size={13} aria-hidden="true" />
              <span>Check Again</span>
            </button>
          </div>
        {:else if updateStatus === "available" && updateInfo}
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
            <span class="notes-label" id="settings-update-notes-label">
              Release Notes
            </span>
            <div
              class="release-notes scrollable-pane"
              role="region"
              aria-labelledby="settings-update-notes-label"
            >
              {releaseNotesText}
            </div>
          </div>

          <div class="update-actions">
            <button
              type="button"
              class="btn btn-secondary"
              onclick={onCheckUpdates}
            >
              <RefreshCw size={13} aria-hidden="true" />
              <span>Check Again</span>
            </button>
            <button
              type="button"
              class="btn btn-primary"
              onclick={onDownloadAndInstall}
            >
              <Download size={14} aria-hidden="true" />
              <span>Download &amp; Install</span>
            </button>
          </div>
        {:else if updateStatus === "downloading"}
          <div class="progress-block">
            <div class="progress-header">
              <span id="settings-update-progress-label" class="progress-title">
                {updateProgress?.percent === 100
                  ? "Finalizing installation..."
                  : `Downloading v${updateInfo?.version ?? "update"}...`}
              </span>
              {#if updateProgress?.percent !== null && updateProgress?.percent !== undefined}
                <span class="progress-percent">{updateProgress.percent}%</span>
              {/if}
            </div>

            <progress
              class="update-progress"
              aria-labelledby="settings-update-progress-label"
              max={100}
              value={updateProgress?.percent ?? undefined}
            ></progress>

            <div class="progress-meta" role="status" aria-live="polite">
              <span>{progressDetailsText}</span>
            </div>
          </div>

          <p class="install-hint">
            Please keep Bex LaTeX Studio open until installation finishes.
          </p>
        {:else if updateStatus === "ready-to-restart"}
          <div class="status-banner status-success" role="status">
            <CircleCheck size={17} class="status-icon-success" aria-hidden="true" />
            <div class="status-copy">
              <p class="status-primary">
                Update installed! Restart Bex LaTeX Studio to switch to
                {#if updateInfo?.version}
                  <code>v{updateInfo.version}</code>.
                {:else}
                  the new version.
                {/if}
              </p>
            </div>
          </div>
          <div class="update-actions">
            <button type="button" class="btn btn-primary" onclick={onRelaunch}>
              <RotateCw size={14} aria-hidden="true" />
              <span>Restart Now</span>
            </button>
          </div>
        {:else if updateStatus === "error"}
          <div class="error-banner" role="alert">
            <CircleExclamationPoint size={16} class="error-icon" aria-hidden="true" />
            <div class="error-copy">
              <p class="error-title">Could not complete the update check.</p>
              <p class="error-detail">
                {updateError || "GitHub Releases could not be reached right now."}
              </p>
            </div>
          </div>
          <div class="update-actions">
            <button
              type="button"
              class="btn btn-primary"
              onclick={onCheckUpdates}
            >
              <RefreshCw size={13} aria-hidden="true" />
              <span>Retry</span>
            </button>
          </div>
        {:else}
          <div class="idle-row">
            <p class="idle-copy">
              Check GitHub Releases to see if a newer version of Bex LaTeX Studio
              is available.
            </p>
            <button
              type="button"
              class="btn btn-secondary"
              onclick={onCheckUpdates}
            >
              <RefreshCw size={13} aria-hidden="true" />
              <span>Check for Updates</span>
            </button>
          </div>
        {/if}
      </div>
    </section>

    <!-- About Bex Section -->
    <section
      id="settings-section-about"
      class="settings-section"
      aria-labelledby="settings-section-about-title"
    >
      <div class="section-heading">
        <Info size={14} aria-hidden="true" />
        <h2 id="settings-section-about-title">About Bex</h2>
      </div>

      <div class="section-card about-card">
        <div class="about-brand">
          <img
            src="/bex-bottle.svg"
            alt=""
            width="44"
            height="44"
            class="about-logo"
          />
          <div class="about-identity">
            <div class="about-title-row">
              <span class="about-name">Bex LaTeX Studio</span>
              <span class="version-pill">v{displayCurrentVersion}</span>
            </div>
            <p class="about-desc">
              Cross-platform Live LaTeX Studio powered by Tauri v2, Svelte 5, and
              embedded Rust Tectonic.
            </p>
          </div>
        </div>

        <dl class="about-meta">
          <div class="meta-item">
            <dt>Workspace</dt>
            <dd title={workspaceRoot ?? undefined}>
              <span class="meta-main">{workspaceName ?? "No project open"}</span>
              {#if workspaceRoot}
                <code class="meta-path">{workspaceRoot}</code>
              {/if}
            </dd>
          </div>

          <div class="meta-item">
            <dt>Repository</dt>
            <dd class="meta-repo">
              <FolderGit2 size={13} aria-hidden="true" />
              <code>https://github.com/Vinello28/bex</code>
            </dd>
          </div>

          <div class="meta-item">
            <dt>Shortcuts</dt>
            <dd class="meta-shortcuts">
              <span><kbd>{modKey}S</kbd> Save &amp; Compile</span>
              <span aria-hidden="true">·</span>
              <span><kbd>{modKey}B</kbd> Sidebar</span>
              <span aria-hidden="true">·</span>
              <span><kbd>{isMac ? "⌘⇧A" : "Ctrl+Shift+A"}</kbd> AI Sidebar</span>
              <span aria-hidden="true">·</span>
              <span><kbd>{isMac ? "⌘⇧J" : "Ctrl+Shift+J"}</kbd> Sync to PDF</span>
              <span aria-hidden="true">·</span>
              <span><kbd>Double-click PDF</kbd> Jump to source</span>
              <span aria-hidden="true">·</span>
              <span><kbd>{modKey}ù</kbd> / <kbd>{modKey}/</kbd> Toggle Comment</span>
              <span aria-hidden="true">·</span>
              <span><kbd>{modKey}+</kbd> / <kbd>{modKey}-</kbd> UI Zoom</span>
              <span aria-hidden="true">·</span>
              <span><kbd>{modKey}0</kbd> Reset Zoom</span>
              <span aria-hidden="true">·</span>
              <span><kbd>{modKey},</kbd> Settings</span>
            </dd>
          </div>
        </dl>
      </div>
    </section>
  </div>

  <footer class="settings-footer">
    <button
      type="button"
      class="btn btn-primary"
      disabled={isDownloading}
      onclick={handleCloseClick}
    >
      Done
    </button>
  </footer>
</dialog>

<style>
  .bex-settings-dialog {
    padding: 0;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    background: var(--bg-elevated);
    color: var(--text-primary);
    box-shadow: var(--shadow-dialog);
    inline-size: min(560px, calc(100% - 32px));
    max-block-size: min(720px, calc(100dvb - 32px));
  }

  .bex-settings-dialog[open] {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .bex-settings-dialog:not([open]) {
    display: none;
  }

  .settings-header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }

  .header-icon {
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

  .header-titles {
    min-inline-size: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .dialog-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    line-height: 1.3;
    color: var(--text-primary);
  }

  .dialog-subtitle {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
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

  .settings-body {
    flex: 1;
    min-block-size: 0;
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section-heading-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .section-heading {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
  }

  .section-heading h2 {
    margin: 0;
    font-size: 11.5px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .section-card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }

  .field-col {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .field-row,
  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }

  .toggle-row {
    cursor: pointer;
  }

  .field-info {
    min-inline-size: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .field-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .field-hint {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .field-select {
    inline-size: 215px;
    max-inline-size: 52%;
    block-size: 30px;
    padding: 0 9px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .field-select:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .field-input {
    inline-size: 215px;
    max-inline-size: 52%;
    block-size: 30px;
    padding: 0 9px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-primary);
    font-size: 12px;
    flex-shrink: 0;
    outline: none;
    transition: border-color 120ms ease;
  }

  .field-input:focus {
    border-color: var(--accent);
  }

  .mono-input,
  .mono-select {
    font-family: var(--font-mono);
  }

  .input-with-button {
    display: flex;
    align-items: center;
    gap: 6px;
    max-inline-size: 60%;
  }

  .input-with-button .field-input {
    inline-size: 100%;
    max-inline-size: 100%;
    flex: 1;
  }

  .input-action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 30px;
    block-size: 30px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-muted);
    cursor: pointer;
    flex-shrink: 0;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .input-action-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .test-btn {
    flex-shrink: 0;
    block-size: 30px;
    padding: 0 9px;
    font-size: 11px;
    gap: 4px;
    white-space: nowrap;
  }

  .models-picker {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 4px;
  }

  .models-picker-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .model-chips-list {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    max-block-size: 100px;
    overflow-y: auto;
  }

  .model-chip-btn {
    padding: 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 10.5px;
    cursor: pointer;
    transition: background-color 100ms ease, border-color 100ms ease, color 100ms ease;
  }

  .model-chip-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--border-strong);
  }

  .model-chip-btn.selected {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }

  .row-sep {
    inline-size: 100%;
    block-size: 1px;
    margin: 0;
    border: none;
    background: var(--border-subtle);
  }

  .compiler-pills {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .compiler-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-muted);
    font-size: 11px;
  }

  .compiler-pill.available {
    color: var(--text-secondary);
  }

  .compiler-dot {
    inline-size: 6px;
    block-size: 6px;
    border-radius: 50%;
    background: var(--border-strong);
    flex-shrink: 0;
  }

  .compiler-pill.available .compiler-dot {
    background: var(--success);
  }

  .compiler-ver {
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--text-muted);
  }

  .native-checkbox {
    inline-size: 16px;
    block-size: 16px;
    margin: 0;
    accent-color: var(--accent);
    cursor: pointer;
    flex-shrink: 0;
  }

  .theme-fieldset {
    margin: 0;
    padding: 0;
    border: none;
  }

  .theme-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
  }

  .theme-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    padding: 10px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-secondary);
    cursor: pointer;
    text-align: center;
    transition: background-color 120ms ease, border-color 120ms ease, color 120ms ease;
  }

  @media (hover: hover) {
    .theme-card:not(.selected):hover {
      background: var(--bg-hover);
      border-color: var(--border-strong);
      color: var(--text-primary);
    }
  }

  .theme-card.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--text-primary);
  }

  .theme-card:has(:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .theme-name {
    font-size: 12.5px;
    font-weight: 600;
  }

  .theme-sub {
    font-size: 11px;
    color: var(--text-muted);
  }

  .theme-card.selected .theme-sub {
    color: var(--text-secondary);
  }

  .zoom-controls {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .zoom-step-btn,
  .zoom-reset-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-inline-size: 28px;
    min-block-size: 28px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-primary);
    font-size: 11.5px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 120ms ease, border-color 120ms ease;
  }

  .zoom-step-btn {
    inline-size: 28px;
    padding: 0;
  }

  .zoom-reset-btn {
    gap: 5px;
    padding: 0 9px;
  }

  .zoom-step-btn:disabled,
  .zoom-reset-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  @media (hover: hover) {
    .zoom-step-btn:not(:disabled):hover,
    .zoom-reset-btn:not(:disabled):hover {
      background: var(--bg-hover);
      border-color: var(--border-strong);
    }
  }

  .zoom-value {
    min-inline-size: 46px;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--text-primary);
  }

  .version-pill {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 500;
  }

  .status-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .status-banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
  }

  .status-banner.status-compact {
    padding: 6px 10px;
  }

  .status-banner.status-success {
    border-color: var(--border-subtle);
    background: var(--success-soft);
  }

  :global(.status-spinner) {
    color: var(--accent);
    flex-shrink: 0;
    margin-top: 2px;
  }

  :global(.status-icon-success) {
    color: var(--success);
    flex-shrink: 0;
    margin-top: 2px;
  }

  .status-copy {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-inline-size: 0;
  }

  .status-primary {
    margin: 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    line-height: 1.45;
  }

  .status-secondary {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-muted);
  }

  .status-primary code,
  .status-secondary code {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .version-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
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
    font-size: 11.5px;
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
    gap: 5px;
    min-inline-size: 0;
  }

  .notes-label {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .release-notes {
    max-block-size: 130px;
    padding: 9px 11px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .progress-block {
    display: flex;
    flex-direction: column;
    gap: 7px;
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
    color: var(--accent);
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
    --progress-bar-indeterminate-animation: bex-settings-progress-indeterminate
      var(--progress-bar-indeterminate-animation-duration) infinite linear reverse;
  }

  .update-progress:indeterminate {
    --progress-bar-animation: var(
      --progress-bar-indeterminate-animation,
      bex-settings-progress-indeterminate
        var(--progress-bar-indeterminate-animation-duration) infinite linear
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

  @keyframes bex-settings-progress-indeterminate {
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

  .install-hint {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .error-banner {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--danger-soft);
  }

  .error-banner.status-compact {
    padding: 6px 10px;
  }

  :global(.error-icon) {
    color: var(--danger);
    flex-shrink: 0;
    margin-top: 2px;
  }

  .error-copy {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-inline-size: 0;
  }

  .error-title {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--danger);
  }

  .error-detail {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-primary);
    line-height: 1.45;
    overflow-wrap: anywhere;
  }

  .idle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .idle-copy {
    margin: 0;
    flex: 1 1 240px;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .update-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }

  .about-brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .about-logo {
    inline-size: 44px;
    block-size: 44px;
    flex-shrink: 0;
    border-radius: var(--radius-sm);
    display: block;
  }

  .about-identity {
    min-inline-size: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .about-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .about-name {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .about-desc {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .about-meta {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--border-subtle);
  }

  .meta-item {
    display: grid;
    grid-template-columns: 88px minmax(0, 1fr);
    align-items: baseline;
    gap: 10px;
    font-size: 12px;
  }

  .meta-item dt {
    color: var(--text-muted);
    font-weight: 500;
  }

  .meta-item dd {
    margin: 0;
    min-inline-size: 0;
    color: var(--text-primary);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .meta-main {
    font-weight: 500;
  }

  .meta-path {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta-item dd.meta-repo {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
  }

  .meta-repo code {
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--text-secondary);
    overflow-wrap: anywhere;
  }

  .meta-item dd.meta-shortcuts {
    flex-direction: row;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    color: var(--text-secondary);
    font-size: 11.5px;
  }

  .meta-shortcuts kbd {
    display: inline-block;
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 10.5px;
  }

  .settings-footer {
    flex-shrink: 0;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }

  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-block-size: 30px;
    padding: 5px 13px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background-color 120ms ease, border-color 120ms ease;
  }

  .btn:disabled {
    opacity: 0.6;
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
    font-weight: 600;
  }

  @media (hover: hover) {
    .btn-primary:not(:disabled):hover {
      background: var(--accent-hover);
    }
  }

  :global(.spin-icon) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .cache-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .cache-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .cache-header-left {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-inline-size: 0;
  }

  .cache-total-badge {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  .cache-refresh-btn,
  .cache-clear-btn {
    min-block-size: 26px;
    padding: 3px 9px;
    font-size: 11px;
    gap: 5px;
    flex-shrink: 0;
  }

  .cache-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 11px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
  }

  .cache-row-info {
    min-inline-size: 0;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .cache-row-title-line {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 6px;
  }

  .cache-row-title {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .cache-row-title code,
  .cache-row-info .field-hint code {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .cache-meta {
    font-family: var(--font-mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }

  @media (hover: hover) {
    .cache-clear-btn:not(:disabled):hover {
      border-color: var(--danger);
      background: var(--danger-soft);
      color: var(--danger);
    }
  }

  :global(.status-icon-info) {
    color: var(--accent);
    flex-shrink: 0;
    margin-top: 2px;
  }
</style>
