<script lang="ts">
  import {
    FolderKanban,
    FolderOpen,
    FolderPlus,
    Palette,
    Search,
    BookOpen,
    FileCode,
    FileText,
    X,
    Settings,
    RefreshCw,
    Laptop,
    Sun,
    Moon,
    Sparkles,
    Cpu,
    Check,
  } from "@lucide/svelte";
  import type {
    RecentProject,
    RecentFile,
    ThemeMode,
    CompilerInfo,
    AiSettings,
    AiEnvStatus,
    UpdateStatus,
    SettingsSectionId,
  } from "../types";
  import {
    formatRelativeTime,
    getProjectMonogram,
    getProjectAvatarColor,
    shortenPath,
  } from "../recent";

  interface Props {
    appVersion: string;
    recentProjects: RecentProject[];
    recentFiles: RecentFile[];
    theme: ThemeMode;
    compilers: CompilerInfo[];
    aiSettings: AiSettings;
    aiEnvStatus: AiEnvStatus;
    availableUpdateVersion: string | null;
    updateStatus: UpdateStatus;
    onOpenFolder: () => void;
    onNewProject: () => void;
    onOpenSampleProject: () => void;
    onOpenRecentProject: (project: RecentProject) => void;
    onRemoveRecentProject: (path: string) => void;
    onOpenRecentFile: (file: RecentFile) => void;
    onRemoveRecentFile: (workspacePath: string, relPath: string) => void;
    onChangeTheme: (theme: ThemeMode) => void;
    onOpenSettings: (section?: SettingsSectionId) => void;
    onCheckUpdates: () => void;
  }

  let {
    appVersion,
    recentProjects,
    recentFiles,
    theme,
    compilers,
    aiSettings,
    aiEnvStatus,
    availableUpdateVersion,
    updateStatus,
    onOpenFolder,
    onNewProject,
    onOpenSampleProject,
    onOpenRecentProject,
    onRemoveRecentProject,
    onOpenRecentFile,
    onRemoveRecentFile,
    onChangeTheme,
    onOpenSettings,
    onCheckUpdates,
  }: Props = $props();

  let activeTab = $state<"projects" | "customize">("projects");
  let searchQuery = $state("");

  const isMac = typeof navigator !== "undefined" && /Mac|iPod|iPhone|iPad/.test(navigator.platform);
  const modKey = isMac ? "⌘" : "Ctrl";

  const filteredProjects = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return recentProjects;
    return recentProjects.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.path.toLowerCase().includes(q) ||
        (p.entryFile && p.entryFile.toLowerCase().includes(q))
    );
  });

  const filteredFiles = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return recentFiles;
    return recentFiles.filter(
      (f) =>
        f.fileName.toLowerCase().includes(q) ||
        f.relPath.toLowerCase().includes(q) ||
        f.workspaceName.toLowerCase().includes(q) ||
        f.workspacePath.toLowerCase().includes(q)
    );
  });

  function getFileIcon(fileName: string) {
    const lower = fileName.toLowerCase();
    if (lower.endsWith(".tex") || lower.endsWith(".ltx") || lower.endsWith(".sty") || lower.endsWith(".cls")) {
      return FileCode;
    }
    if (lower.endsWith(".bib")) {
      return BookOpen;
    }
    return FileText;
  }

  function getProviderLabel(provider: string): string {
    switch (provider) {
      case "google":
        return "Google AI Studio (Gemini)";
      case "ollama":
        return "Ollama (Local)";
      case "lmstudio":
        return "LM Studio";
      case "vllm":
        return "vLLM";
      default:
        return provider;
    }
  }

  const aiStatusText = $derived.by(() => {
    if (aiSettings.provider === "google") {
      if (aiEnvStatus.hasGoogleEnvKey || aiSettings.apiKey.trim().length > 0) {
        return ".env key ready";
      }
      return "API key required";
    }
    if (aiSettings.baseUrl && aiSettings.baseUrl.trim().length > 0) {
      return "Configured";
    }
    return "Endpoint required";
  });
</script>

<div class="home-container">
  <!-- Left Navigation Rail Island -->
  <aside class="nav-island" aria-label="Welcome screen navigation">
    <div class="nav-top">
      <div class="brand-block">
        <img src="/bex-bottle.svg" alt="Bex Logo" width="40" height="40" class="brand-logo" />
        <div class="brand-info">
          <h1 class="brand-title">Bex LaTeX Studio</h1>
          <span class="brand-version">Version {appVersion}</span>
        </div>
      </div>

      <nav class="nav-items" aria-label="Main menu">
        <button
          type="button"
          class="nav-tab-btn"
          class:active={activeTab === "projects"}
          onclick={() => (activeTab = "projects")}
        >
          <FolderKanban size={18} class="tab-icon" />
          <span class="tab-label">Projects</span>
        </button>

        <button
          type="button"
          class="nav-tab-btn"
          class:active={activeTab === "customize"}
          onclick={() => (activeTab = "customize")}
        >
          <Palette size={18} class="tab-icon" />
          <span class="tab-label">Customize</span>
        </button>
      </nav>
    </div>

    <!-- Bottom Utility Dock -->
    <div class="nav-bottom">
      <div class="update-section">
        {#if availableUpdateVersion}
          <button
            type="button"
            class="update-pill-btn available"
            onclick={() => onOpenSettings("updates")}
            title="Update available - click to view in Settings"
          >
            <span class="pulse-dot"></span>
            <span class="update-label">Update v{availableUpdateVersion}</span>
          </button>
        {:else if updateStatus === "checking"}
          <div class="update-status-pill checking">
            <RefreshCw size={13} class="spin" />
            <span>Checking updates...</span>
          </div>
        {:else}
          <button
            type="button"
            class="update-check-btn"
            onclick={onCheckUpdates}
            title="Check for updates"
          >
            <RefreshCw size={14} />
            <span>Check for Updates</span>
          </button>
        {/if}
      </div>

      <button
        type="button"
        class="settings-btn"
        onclick={() => onOpenSettings()}
        title="Settings ({modKey},)"
      >
        <div class="settings-btn-main">
          <Settings size={16} />
          <span>Settings</span>
          {#if availableUpdateVersion}
            <span class="settings-update-dot" title="Update available"></span>
          {/if}
        </div>
        <kbd class="shortcut-kbd">{modKey},</kbd>
      </button>
    </div>
  </aside>

  <!-- Right Main Island -->
  <main class="main-island" aria-label="Main content">
    {#if activeTab === "projects"}
      <!-- Top Action Bar -->
      <header class="projects-topbar">
        <div class="search-box">
          <Search size={15} class="search-icon" />
          <input
            type="search"
            class="search-input"
            placeholder="Search recent projects and files..."
            bind:value={searchQuery}
            aria-label="Search recent projects and files"
          />
          {#if searchQuery}
            <button
              type="button"
              class="search-clear-btn"
              onclick={() => (searchQuery = "")}
              title="Clear search"
            >
              <X size={14} />
            </button>
          {/if}
        </div>

        <div class="topbar-actions">
          <button
            type="button"
            class="action-btn primary-cta"
            onclick={onNewProject}
            title="Create a new LaTeX project"
          >
            <FolderPlus size={16} />
            <span>New Project</span>
          </button>

          <button
            type="button"
            class="action-btn secondary-cta"
            onclick={onOpenFolder}
            title="Open an existing project directory"
          >
            <FolderOpen size={16} />
            <span>Open Directory</span>
          </button>

          <button
            type="button"
            class="action-btn secondary-cta"
            onclick={onOpenSampleProject}
            title="Explore sample starter document"
          >
            <BookOpen size={16} />
            <span>Sample Project</span>
          </button>
        </div>
      </header>

      <!-- Scrollable Projects Content -->
      <div class="scrollable-pane projects-content">
        {#if recentProjects.length === 0 && recentFiles.length === 0}
          <!-- Empty State Hero -->
          <div class="empty-hero">
            <div class="hero-mascot-wrapper">
              <img src="/bex-bottle.svg" alt="" width="80" height="80" class="hero-mascot" />
            </div>
            <h2 class="hero-title">Welcome to Bex LaTeX Studio</h2>
            <p class="hero-subtitle">
              High-performance desktop LaTeX environment powered by Rust, Tectonic, and AI assistance.
            </p>

            <div class="empty-cards-grid">
              <button type="button" class="empty-card" onclick={onNewProject}>
                <div class="empty-card-icon primary">
                  <FolderPlus size={24} />
                </div>
                <div class="empty-card-text">
                  <h3>New Project</h3>
                  <p>Start a clean LaTeX document from a template or scratch.</p>
                </div>
              </button>

              <button type="button" class="empty-card" onclick={onOpenFolder}>
                <div class="empty-card-icon">
                  <FolderOpen size={24} />
                </div>
                <div class="empty-card-text">
                  <h3>Open Directory</h3>
                  <p>Open an existing folder with TeX source files.</p>
                </div>
              </button>

              <button type="button" class="empty-card" onclick={onOpenSampleProject}>
                <div class="empty-card-icon">
                  <BookOpen size={24} />
                </div>
                <div class="empty-card-text">
                  <h3>Explore Sample Project</h3>
                  <p>Browse a ready-to-compile sample paper with figures and math.</p>
                </div>
              </button>
            </div>
          </div>
        {:else}
          <!-- Filterable Lists -->
          <div class="lists-wrapper">
            <!-- Recent Projects Section -->
            <section class="list-section" aria-labelledby="heading-projects">
              <div class="section-header">
                <h2 id="heading-projects" class="section-title">
                  Recent Projects
                  <span class="count-badge">{filteredProjects.length}</span>
                </h2>
              </div>

              {#if filteredProjects.length === 0}
                <div class="no-results-msg">
                  No projects matching "{searchQuery}"
                </div>
              {:else}
                <div class="recent-list" role="list">
                  {#each filteredProjects as proj (proj.path)}
                    {@const monogram = getProjectMonogram(proj.name)}
                    {@const avatarColors = getProjectAvatarColor(proj.name)}
                    <div class="recent-row" role="listitem">
                      <button
                        type="button"
                        class="recent-main"
                        onclick={() => onOpenRecentProject(proj)}
                        title={`Open ${proj.name} (${proj.path})`}
                      >
                        <div
                          class="project-avatar"
                          style="background-color: {avatarColors.bg}; color: {avatarColors.fg};"
                        >
                          {monogram}
                        </div>
                        <div class="project-details">
                          <div class="project-row-top">
                            <span class="project-name">{proj.name}</span>
                            {#if proj.entryFile}
                              <span class="entry-file-pill" title="Root entry file">
                                {proj.entryFile}
                              </span>
                            {/if}
                          </div>
                          <div class="project-row-bottom">
                            <span class="project-path" title={proj.path}>
                              {shortenPath(proj.path)}
                            </span>
                          </div>
                        </div>
                        <div class="project-meta">
                          <span class="relative-time">{formatRelativeTime(proj.lastOpened)}</span>
                        </div>
                      </button>

                      <button
                        type="button"
                        class="remove-btn"
                        onclick={(e) => {
                          e.stopPropagation();
                          onRemoveRecentProject(proj.path);
                        }}
                        title={`Remove ${proj.name} from recent projects`}
                        aria-label={`Remove ${proj.name} from recent projects`}
                      >
                        <X size={15} />
                      </button>
                    </div>
                  {/each}
                </div>
              {/if}
            </section>

            <!-- Recent Files Section -->
            {#if recentFiles.length > 0}
              <section class="list-section" aria-labelledby="heading-files">
                <div class="section-header">
                  <h2 id="heading-files" class="section-title">
                    Recent Files
                    <span class="count-badge">{filteredFiles.length}</span>
                  </h2>
                </div>

                {#if filteredFiles.length === 0}
                  <div class="no-results-msg">
                    No files matching "{searchQuery}"
                  </div>
                {:else}
                  <div class="recent-list" role="list">
                    {#each filteredFiles as file (`${file.workspacePath}::${file.relPath}`)}
                      {@const IconComp = getFileIcon(file.fileName)}
                      <div class="recent-row" role="listitem">
                        <button
                          type="button"
                          class="recent-main"
                          onclick={() => onOpenRecentFile(file)}
                          title={`Open ${file.fileName} in ${file.workspaceName}`}
                        >
                          <div class="file-icon-box">
                            <IconComp size={18} />
                          </div>
                          <div class="project-details">
                            <div class="project-row-top">
                              <span class="project-name">{file.fileName}</span>
                              <span class="workspace-pill" title="Workspace">
                                {file.workspaceName}
                              </span>
                            </div>
                            <div class="project-row-bottom">
                              <span class="project-path" title={`${file.workspacePath}/${file.relPath}`}>
                                {file.relPath}
                              </span>
                            </div>
                          </div>
                          <div class="project-meta">
                            <span class="relative-time">{formatRelativeTime(file.lastOpened)}</span>
                          </div>
                        </button>

                        <button
                          type="button"
                          class="remove-btn"
                          onclick={(e) => {
                            e.stopPropagation();
                            onRemoveRecentFile(file.workspacePath, file.relPath);
                          }}
                          title={`Remove ${file.fileName} from recent files`}
                          aria-label={`Remove ${file.fileName} from recent files`}
                        >
                          <X size={15} />
                        </button>
                      </div>
                    {/each}
                  </div>
                {/if}
              </section>
            {/if}
          </div>
        {/if}
      </div>
    {:else if activeTab === "customize"}
      <!-- Customize View -->
      <div class="scrollable-pane customize-content">
        <div class="customize-inner">
          <header class="customize-header">
            <h2 class="customize-title">Customization & Environment</h2>
            <p class="customize-subtitle">
              Manage appearance, AI model connections, and LaTeX compilation engines.
            </p>
          </header>

          <!-- Theme Selection Cards -->
          <section class="customize-section" aria-labelledby="theme-heading">
            <h3 id="theme-heading" class="customize-section-title">Color Theme</h3>
            <div class="theme-grid">
              <!-- System Theme -->
              <button
                type="button"
                class="theme-card"
                class:selected={theme === "system"}
                onclick={() => onChangeTheme("system")}
              >
                <div class="theme-preview system-preview">
                  <div class="preview-split-light"></div>
                  <div class="preview-split-dark"></div>
                  <Laptop size={22} class="preview-center-icon" />
                </div>
                <div class="theme-card-body">
                  <div class="theme-card-header">
                    <span class="theme-name">System Default</span>
                    {#if theme === "system"}
                      <Check size={16} class="check-icon" />
                    {/if}
                  </div>
                  <span class="theme-desc">Sync automatically with operating system appearance</span>
                </div>
              </button>

              <!-- Islands Light -->
              <button
                type="button"
                class="theme-card"
                class:selected={theme === "light"}
                onclick={() => onChangeTheme("light")}
              >
                <div class="theme-preview light-preview">
                  <div class="preview-wireframe light">
                    <div class="pw-sidebar"></div>
                    <div class="pw-main">
                      <div class="pw-bar"></div>
                      <div class="pw-lines"></div>
                    </div>
                  </div>
                  <Sun size={20} class="preview-center-icon light" />
                </div>
                <div class="theme-card-body">
                  <div class="theme-card-header">
                    <span class="theme-name">Islands Light</span>
                    {#if theme === "light"}
                      <Check size={16} class="check-icon" />
                    {/if}
                  </div>
                  <span class="theme-desc">Warm amber accents on crisp pure-white paper surfaces</span>
                </div>
              </button>

              <!-- Islands Dark -->
              <button
                type="button"
                class="theme-card"
                class:selected={theme === "dark"}
                onclick={() => onChangeTheme("dark")}
              >
                <div class="theme-preview dark-preview">
                  <div class="preview-wireframe dark">
                    <div class="pw-sidebar"></div>
                    <div class="pw-main">
                      <div class="pw-bar"></div>
                      <div class="pw-lines"></div>
                    </div>
                  </div>
                  <Moon size={20} class="preview-center-icon dark" />
                </div>
                <div class="theme-card-body">
                  <div class="theme-card-header">
                    <span class="theme-name">Islands Dark</span>
                    {#if theme === "dark"}
                      <Check size={16} class="check-icon" />
                    {/if}
                  </div>
                  <span class="theme-desc">True charcoal background with vibrant gold highlights</span>
                </div>
              </button>
            </div>
          </section>

          <!-- AI Agent & MCP Summary Card -->
          <section class="customize-section" aria-labelledby="ai-heading">
            <h3 id="ai-heading" class="customize-section-title">AI LaTeX Assistant & MCP</h3>
            <div class="summary-card">
              <div class="summary-card-icon ai">
                <Sparkles size={24} />
              </div>
              <div class="summary-card-content">
                <div class="summary-card-title-row">
                  <h4 class="summary-title">{getProviderLabel(aiSettings.provider)}</h4>
                  <span class="status-badge" class:ready={aiStatusText.includes("ready") || aiStatusText.includes("Configured")}>
                    {aiStatusText}
                  </span>
                </div>
                <p class="summary-detail">
                  Active Model: <strong>{aiSettings.model || "Default"}</strong> &bull;
                  Auto-compile after edits: {aiSettings.autoCompileAfterAiEdit ? "Enabled" : "Disabled"}
                </p>
                <p class="summary-subtext">
                  Bex connects directly to Gemini or local LLMs (Ollama, LM Studio) via native MCP tools for real-time document editing and compilation diagnostics.
                </p>
              </div>
              <div class="summary-card-action">
                <button
                  type="button"
                  class="action-btn secondary-cta"
                  onclick={() => onOpenSettings("ai")}
                >
                  <Settings size={15} />
                  <span>Configure AI & MCP...</span>
                </button>
              </div>
            </div>
          </section>

          <!-- LaTeX Compilers Summary Card -->
          <section class="customize-section" aria-labelledby="compilers-heading">
            <h3 id="compilers-heading" class="customize-section-title">LaTeX Compilers</h3>
            <div class="summary-card">
              <div class="summary-card-icon compiler">
                <Cpu size={24} />
              </div>
              <div class="summary-card-content">
                <div class="summary-card-title-row">
                  <h4 class="summary-title">Installed TeX Engines</h4>
                  <span class="status-badge ready">
                    {compilers.filter((c) => c.available).length} Available
                  </span>
                </div>
                <div class="compilers-list">
                  {#each compilers as comp (comp.id)}
                    <span class="compiler-item" class:available={comp.available}>
                      <span class="status-dot" class:available={comp.available}></span>
                      <span class="compiler-name">{comp.name}</span>
                      {#if comp.isRustNative}
                        <span class="native-tag">Built-in</span>
                      {/if}
                    </span>
                  {/each}
                </div>
                <p class="summary-subtext">
                  Embedded Tectonic provides zero-config compilation with automatic package downloads from CTAN. External engines (pdflatex, xelatex, lualatex) are auto-detected.
                </p>
              </div>
              <div class="summary-card-action">
                <button
                  type="button"
                  class="action-btn secondary-cta"
                  onclick={() => onOpenSettings("compilation")}
                >
                  <Settings size={15} />
                  <span>Compiler Settings...</span>
                </button>
              </div>
            </div>
          </section>
        </div>
      </div>
    {/if}
  </main>
</div>

<style>
  /* Base Home Container: Full dvb viewport with 2-column islands grid */
  .home-container {
    inline-size: 100%;
    block-size: 100dvb;
    background: var(--bg-app);
    padding: 12px;
    display: grid;
    grid-template-columns: 248px minmax(0, 1fr);
    gap: 10px;
    overflow: hidden;
    box-sizing: border-box;
  }

  /* Left Navigation Rail Island */
  .nav-island {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-island, 10px);
    box-shadow: var(--shadow-island, 0 2px 8px rgba(0, 0, 0, 0.04));
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 16px 12px;
    min-block-size: 0;
    overflow: hidden;
  }

  .nav-top {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .brand-block {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 6px;
  }

  .brand-logo {
    flex-shrink: 0;
    inline-size: 40px;
    block-size: 40px;
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.1));
  }

  .brand-info {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
  }

  .brand-title {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .brand-version {
    font-size: 11px;
    color: var(--text-muted);
    font-weight: 500;
  }

  .nav-items {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .nav-tab-btn {
    all: unset;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-radius: var(--radius-sm, 6px);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background-color 0.12s ease, color 0.12s ease;
  }

  .nav-tab-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-tab-btn.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  :global(.nav-tab-btn.active .tab-icon) {
    color: var(--accent);
  }

  .nav-bottom {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--border-subtle);
  }

  .update-section {
    display: flex;
    flex-direction: column;
  }

  .update-pill-btn.available {
    all: unset;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--success-soft, rgba(21, 128, 61, 0.12));
    color: var(--success, #15803d);
    border: 1px solid var(--success, #15803d);
    border-radius: var(--radius-sm, 6px);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.12s ease;
  }

  .update-pill-btn.available:hover {
    background: var(--success-soft, rgba(21, 128, 61, 0.22));
  }

  .pulse-dot {
    inline-size: 8px;
    block-size: 8px;
    border-radius: 50%;
    background: var(--success, #15803d);
    box-shadow: 0 0 0 0 rgba(21, 128, 61, 0.7);
    animation: pulse 1.8s infinite;
  }

  @keyframes pulse {
    0% {
      transform: scale(0.95);
      box-shadow: 0 0 0 0 rgba(21, 128, 61, 0.7);
    }
    70% {
      transform: scale(1);
      box-shadow: 0 0 0 6px rgba(21, 128, 61, 0);
    }
    100% {
      transform: scale(0.95);
      box-shadow: 0 0 0 0 rgba(21, 128, 61, 0);
    }
  }

  .update-status-pill.checking {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--bg-hover);
    color: var(--text-muted);
    border-radius: var(--radius-sm, 6px);
    font-size: 12px;
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .update-check-btn {
    all: unset;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    color: var(--text-muted);
    font-size: 12px;
    border-radius: var(--radius-sm, 6px);
    cursor: pointer;
    transition: background-color 0.12s ease, color 0.12s ease;
  }

  .update-check-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .settings-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border-radius: var(--radius-sm, 6px);
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    transition: background-color 0.12s ease, color 0.12s ease;
  }

  .settings-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .settings-btn-main {
    display: flex;
    align-items: center;
    gap: 8px;
    position: relative;
  }

  .settings-update-dot {
    inline-size: 6px;
    block-size: 6px;
    border-radius: 50%;
    background: var(--success, #15803d);
    position: absolute;
    top: -2px;
    right: -6px;
  }

  .shortcut-kbd {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    padding: 1px 5px;
    background: var(--bg-muted);
    border: 1px solid var(--border-subtle);
    border-radius: 3px;
    color: var(--text-muted);
  }

  /* Right Main Island */
  .main-island {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-island, 10px);
    box-shadow: var(--shadow-island, 0 2px 8px rgba(0, 0, 0, 0.04));
    display: flex;
    flex-direction: column;
    min-block-size: 0;
    overflow: hidden;
  }

  /* Projects Top Bar */
  .projects-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 14px 20px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    flex-shrink: 0;
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
    max-inline-size: 420px;
  }

  :global(.search-icon) {
    position: absolute;
    left: 10px;
    color: var(--text-muted);
    pointer-events: none;
  }

  .search-input {
    inline-size: 100%;
    block-size: 34px;
    padding: 0 32px 0 32px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm, 6px);
    font-size: 13px;
    color: var(--text-primary);
    transition: border-color 0.15s ease, background-color 0.15s ease;
  }

  .search-input:focus {
    background: var(--bg-surface);
    border-color: var(--accent);
    outline: none;
  }

  .search-clear-btn {
    all: unset;
    position: absolute;
    right: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    inline-size: 20px;
    block-size: 20px;
    border-radius: 50%;
    color: var(--text-muted);
    cursor: pointer;
  }

  .search-clear-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .topbar-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .action-btn {
    all: unset;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 14px;
    border-radius: var(--radius-sm, 6px);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.12s ease, opacity 0.12s ease, border-color 0.12s ease;
    white-space: nowrap;
  }

  .action-btn.primary-cta {
    background: var(--accent);
    color: var(--accent-fg, #ffffff);
    font-weight: 600;
  }

  .action-btn.primary-cta:hover {
    opacity: 0.92;
  }

  .action-btn.secondary-cta {
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    color: var(--text-primary);
  }

  .action-btn.secondary-cta:hover {
    background: var(--bg-hover);
    border-color: var(--border-strong);
  }

  /* Projects Content & Lists */
  .projects-content {
    flex: 1;
    min-block-size: 0;
    padding: 20px;
  }

  /* Empty Hero */
  .empty-hero {
    max-inline-size: 640px;
    margin: 40px auto 20px auto;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .hero-mascot-wrapper {
    margin-bottom: 16px;
  }

  .hero-mascot {
    filter: drop-shadow(0 4px 12px rgba(0, 0, 0, 0.15));
  }

  .hero-title {
    margin: 0 0 8px 0;
    font-size: 22px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .hero-subtitle {
    margin: 0 0 32px 0;
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .empty-cards-grid {
    inline-size: 100%;
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
  }

  .empty-card {
    all: unset;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 20px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-island, 10px);
    cursor: pointer;
    text-align: left;
    transition: transform 0.12s ease, border-color 0.12s ease, box-shadow 0.12s ease;
  }

  .empty-card:hover {
    transform: translateY(-1px);
    border-color: var(--accent);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.06);
  }

  .empty-card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    inline-size: 44px;
    block-size: 44px;
    border-radius: var(--radius-md, 8px);
    background: var(--bg-hover);
    color: var(--text-primary);
    flex-shrink: 0;
  }

  .empty-card-icon.primary {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .empty-card-text h3 {
    margin: 0 0 4px 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .empty-card-text p {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  /* Filterable lists */
  .lists-wrapper {
    display: flex;
    flex-direction: column;
    gap: 28px;
    max-inline-size: 960px;
    margin: 0 auto;
  }

  .list-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 4px;
  }

  .section-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .count-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 1px 6px;
    background: var(--bg-hover);
    border-radius: 999px;
    font-size: 11px;
    color: var(--text-secondary);
    font-weight: 500;
  }

  .no-results-msg {
    padding: 24px;
    text-align: center;
    color: var(--text-muted);
    font-style: italic;
    font-size: 13px;
  }

  .recent-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  /* Accessible Row Wrapper */
  .recent-row {
    position: relative;
    display: flex;
    align-items: center;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm, 6px);
    transition: background-color 0.1s ease, border-color 0.1s ease;
  }

  .recent-row:hover {
    background: var(--bg-hover);
    border-color: var(--border-strong);
  }

  .recent-main {
    all: unset;
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    min-inline-size: 0;
    padding: 10px 14px;
    cursor: pointer;
    text-align: left;
  }

  .project-avatar {
    inline-size: 36px;
    block-size: 36px;
    border-radius: var(--radius-sm, 6px);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.05em;
    flex-shrink: 0;
    user-select: none;
  }

  .file-icon-box {
    inline-size: 36px;
    block-size: 36px;
    border-radius: var(--radius-sm, 6px);
    background: var(--bg-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
    flex-shrink: 0;
  }

  .project-details {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
    min-inline-size: 0;
  }

  .project-row-top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-inline-size: 0;
  }

  .project-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .entry-file-pill,
  .workspace-pill {
    display: inline-block;
    padding: 1px 6px;
    border-radius: 4px;
    background: var(--bg-muted);
    border: 1px solid var(--border-subtle);
    font-size: 11px;
    color: var(--text-secondary);
    font-family: var(--font-mono, monospace);
    white-space: nowrap;
  }

  .project-row-bottom {
    display: flex;
    align-items: center;
    min-inline-size: 0;
  }

  .project-path {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .project-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 12px;
    flex-shrink: 0;
  }

  .relative-time {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .remove-btn {
    all: unset;
    display: flex;
    align-items: center;
    justify-content: center;
    inline-size: 28px;
    block-size: 28px;
    margin-right: 8px;
    border-radius: 4px;
    color: var(--text-muted);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.12s ease, background-color 0.12s ease, color 0.12s ease;
  }

  .recent-row:hover .remove-btn,
  .remove-btn:focus-visible {
    opacity: 1;
  }

  .remove-btn:hover {
    background: var(--bg-elevated);
    color: var(--danger, #c62828);
  }

  /* Customize Content */
  .customize-content {
    flex: 1;
    min-block-size: 0;
    padding: 28px;
  }

  .customize-inner {
    max-inline-size: 800px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 32px;
  }

  .customize-header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .customize-title {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .customize-subtitle {
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
  }

  .customize-section {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .customize-section-title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  /* Theme Cards Grid */
  .theme-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 14px;
  }

  .theme-card {
    all: unset;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border: 2px solid var(--border-subtle);
    border-radius: var(--radius-island, 10px);
    overflow: hidden;
    cursor: pointer;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .theme-card:hover {
    border-color: var(--border-strong);
  }

  .theme-card.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .theme-preview {
    block-size: 96px;
    inline-size: 100%;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border-subtle);
    overflow: hidden;
  }

  .system-preview {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }

  .preview-split-light {
    inline-size: 100%;
    block-size: 100%;
    background: #f2f2f2;
  }

  .preview-split-dark {
    inline-size: 100%;
    block-size: 100%;
    background: #0a0a0a;
  }

  :global(.preview-center-icon) {
    position: absolute;
    color: var(--text-primary);
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.2));
  }

  :global(.preview-center-icon.light) {
    color: #b45309;
  }

  :global(.preview-center-icon.dark) {
    color: #f59e0b;
  }

  .preview-wireframe {
    inline-size: 80%;
    block-size: 70%;
    border-radius: 4px;
    display: grid;
    grid-template-columns: 24px 1fr;
    gap: 4px;
    padding: 4px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.1);
  }

  .preview-wireframe.light {
    background: #ffffff;
    border: 1px solid #dfdfdf;
  }

  .preview-wireframe.light .pw-sidebar {
    background: #f2f2f2;
    border-radius: 2px;
  }

  .preview-wireframe.light .pw-bar {
    block-size: 6px;
    background: #b45309;
    border-radius: 2px;
    margin-bottom: 4px;
  }

  .preview-wireframe.light .pw-lines {
    block-size: 16px;
    background: #ececec;
    border-radius: 2px;
  }

  .preview-wireframe.dark {
    background: #0a0a0a;
    border: 1px solid #242424;
  }

  .preview-wireframe.dark .pw-sidebar {
    background: #1a1a1a;
    border-radius: 2px;
  }

  .preview-wireframe.dark .pw-bar {
    block-size: 6px;
    background: #f59e0b;
    border-radius: 2px;
    margin-bottom: 4px;
  }

  .preview-wireframe.dark .pw-lines {
    block-size: 16px;
    background: #262626;
    border-radius: 2px;
  }

  .theme-card-body {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .theme-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .theme-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  :global(.check-icon) {
    color: var(--accent);
  }

  .theme-desc {
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.35;
  }

  /* Summary Cards */
  .summary-card {
    display: flex;
    align-items: flex-start;
    gap: 16px;
    padding: 18px 20px;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-island, 10px);
  }

  .summary-card-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    inline-size: 44px;
    block-size: 44px;
    border-radius: var(--radius-md, 8px);
    background: var(--bg-elevated);
    flex-shrink: 0;
  }

  .summary-card-icon.ai {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .summary-card-icon.compiler {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .summary-card-content {
    flex: 1;
    min-inline-size: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .summary-card-title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .summary-title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .status-badge {
    display: inline-flex;
    align-items: center;
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 11px;
    font-weight: 600;
    background: var(--warning-soft, rgba(161, 92, 7, 0.12));
    color: var(--warning, #a15c07);
  }

  .status-badge.ready {
    background: var(--success-soft, rgba(21, 128, 61, 0.12));
    color: var(--success, #15803d);
  }

  .summary-detail {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .summary-subtext {
    margin: 0;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .summary-card-action {
    flex-shrink: 0;
  }

  .compilers-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 2px 0;
  }

  .compiler-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border-radius: 4px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    font-size: 12px;
    color: var(--text-secondary);
  }

  .compiler-item.available {
    color: var(--text-primary);
  }

  .status-dot {
    inline-size: 6px;
    block-size: 6px;
    border-radius: 50%;
    background: var(--text-muted);
  }

  .status-dot.available {
    background: var(--success, #15803d);
  }

  .native-tag {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 4px;
    border-radius: 3px;
    background: var(--accent-soft);
    color: var(--accent);
    text-transform: uppercase;
  }
</style>
