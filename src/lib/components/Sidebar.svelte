<script lang="ts">
  import {
    FolderOpen,
    Folder,
    FilePlus,
    FolderPlus,
    RefreshCw,
    ChevronRight,
    FileCode,
    BookOpen,
    FileBadge,
    FileType,
    FileImage,
    FileText,
    File,
    Star,
    Pencil,
    Trash,
  } from "@lucide/svelte";
  import type { FileNode, WorkspaceInfo } from "../types";

  interface Props {
    workspace: WorkspaceInfo | null;
    files: FileNode[];
    activeFilePath: string | null;
    entryFile: string | null;
    dirtyPaths: Set<string>;
    onSelectFile: (path: string) => void;
    onSetEntryFile: (path: string) => void;
    onOpenFolder: () => void;
    onCreateFile: (parentDir?: string) => void;
    onCreateFolder: (parentDir?: string) => void;
    onRenameNode: (node: FileNode) => void;
    onDeleteNode: (node: FileNode) => void;
    onRefresh: () => void;
  }

  let {
    workspace,
    files,
    activeFilePath,
    entryFile,
    dirtyPaths,
    onSelectFile,
    onSetEntryFile,
    onOpenFolder,
    onCreateFile,
    onCreateFolder,
    onRenameNode,
    onDeleteNode,
    onRefresh,
  }: Props = $props();

  let expandedDirs = $state<Record<string, boolean>>({});

  // Default-expand top-level folders whenever a workspace loads or new root dirs appear
  $effect(() => {
    const next = { ...expandedDirs };
    let changed = false;
    for (const node of files) {
      if (node.isDir && !(node.path in next)) {
        next[node.path] = true;
        changed = true;
      }
    }
    if (changed) {
      expandedDirs = next;
    }
  });

  function toggleDir(path: string) {
    expandedDirs = {
      ...expandedDirs,
      [path]: !expandedDirs[path],
    };
  }

  function getExt(name: string): string {
    const idx = name.lastIndexOf(".");
    return idx >= 0 ? name.slice(idx).toLowerCase() : "";
  }

  function isTexFile(name: string): boolean {
    const ext = getExt(name);
    return ext === ".tex" || ext === ".ltx";
  }

  function isImageExt(ext: string): boolean {
    return [
      ".png",
      ".jpg",
      ".jpeg",
      ".gif",
      ".svg",
      ".webp",
      ".bmp",
      ".eps",
    ].includes(ext);
  }

  function isStyleExt(ext: string): boolean {
    return [".cls", ".sty", ".dtx", ".bst", ".def"].includes(ext);
  }
</script>

<aside class="sidebar" aria-label="Project file explorer">
  <header class="sidebar-header">
    <h2 class="sidebar-title" title={workspace?.rootPath ?? "No project open"}>
      {workspace?.name ?? "Explorer"}
    </h2>
    <div class="header-actions">
      <button
        type="button"
        class="header-btn"
        title="Open Project Folder"
        aria-label="Open Project Folder"
        onclick={onOpenFolder}
      >
        <FolderOpen size={14} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="header-btn"
        title="New File"
        aria-label="New File"
        disabled={!workspace}
        onclick={() => onCreateFile("")}
      >
        <FilePlus size={14} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="header-btn"
        title="New Folder"
        aria-label="New Folder"
        disabled={!workspace}
        onclick={() => onCreateFolder("")}
      >
        <FolderPlus size={14} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="header-btn"
        title="Refresh Files"
        aria-label="Refresh Files"
        disabled={!workspace}
        onclick={onRefresh}
      >
        <RefreshCw size={14} aria-hidden="true" />
      </button>
    </div>
  </header>

  <div class="tree-scroll scrollable-pane">
    {#if !workspace}
      <div class="empty-state">
        <p class="empty-text">No project folder loaded.</p>
        <button type="button" class="open-btn" onclick={onOpenFolder}>
          <FolderOpen size={15} aria-hidden="true" />
          <span>Open Project Folder</span>
        </button>
      </div>
    {:else if files.length === 0}
      <div class="empty-state">
        <p class="empty-text">This folder is empty.</p>
        <button
          type="button"
          class="open-btn"
          onclick={() => onCreateFile("")}
        >
          <FilePlus size={15} aria-hidden="true" />
          <span>Create main.tex</span>
        </button>
      </div>
    {:else}
      <ul class="tree-list" role="tree" aria-label="Workspace files">
        {@render renderNodes(files, 0)}
      </ul>
    {/if}
  </div>
</aside>

{#snippet renderNodes(nodes: FileNode[], depth: number)}
  {#each nodes as node (node.path)}
    {@const isExpanded = Boolean(expandedDirs[node.path])}
    {@const isActive = !node.isDir && node.path === activeFilePath}
    {@const isMain = !node.isDir && node.path === entryFile}
    {@const isDirty = !node.isDir && dirtyPaths.has(node.path)}
    {@const ext = getExt(node.name)}

    <li
      class="tree-node"
      role="treeitem"
      aria-expanded={node.isDir ? isExpanded : undefined}
      aria-selected={isActive}
    >
      <div
        class="tree-row"
        class:active={isActive}
        style={`padding-inline-start: ${8 + depth * 14}px`}
      >
        <button
          type="button"
          class="row-main"
          title={node.path}
          onclick={() => (node.isDir ? toggleDir(node.path) : onSelectFile(node.path))}
        >
          {#if node.isDir}
            <span class="chevron" class:expanded={isExpanded} aria-hidden="true">
              <ChevronRight size={13} />
            </span>
            <span class="node-icon folder-icon" aria-hidden="true">
              {#if isExpanded}
                <FolderOpen size={15} />
              {:else}
                <Folder size={15} />
              {/if}
            </span>
          {:else}
            <span class="chevron-spacer" aria-hidden="true"></span>
            <span class="node-icon" aria-hidden="true">
              {#if isTexFile(node.name)}
                <FileCode size={15} class="icon-tex" />
              {:else if ext === ".bib"}
                <BookOpen size={15} class="icon-bib" />
              {:else if isStyleExt(ext)}
                <FileBadge size={15} class="icon-style" />
              {:else if ext === ".pdf"}
                <FileType size={15} class="icon-pdf" />
              {:else if isImageExt(ext)}
                <FileImage size={15} class="icon-img" />
              {:else if ext === ".md" || ext === ".txt" || ext === ".log"}
                <FileText size={15} />
              {:else}
                <File size={15} />
              {/if}
            </span>
          {/if}

          <span class="node-name">{node.name}</span>

          {#if isDirty}
            <span
              class="dirty-dot"
              title="Unsaved changes"
              aria-label="Unsaved changes"
            ></span>
          {/if}

          {#if isMain}
            <span class="main-badge" title="Main entry file">MAIN</span>
          {/if}
        </button>

        <div class="row-actions">
          {#if node.isDir}
            <button
              type="button"
              class="row-action-btn"
              title={`New file in ${node.name}`}
              aria-label={`New file in ${node.name}`}
              onclick={(e) => {
                e.stopPropagation();
                expandedDirs = { ...expandedDirs, [node.path]: true };
                onCreateFile(node.path);
              }}
            >
              <FilePlus size={12} aria-hidden="true" />
            </button>
            <button
              type="button"
              class="row-action-btn"
              title={`New folder in ${node.name}`}
              aria-label={`New folder in ${node.name}`}
              onclick={(e) => {
                e.stopPropagation();
                expandedDirs = { ...expandedDirs, [node.path]: true };
                onCreateFolder(node.path);
              }}
            >
              <FolderPlus size={12} aria-hidden="true" />
            </button>
          {:else if isTexFile(node.name) && !isMain}
            <button
              type="button"
              class="row-action-btn"
              title="Set as Main .tex"
              aria-label={`Set ${node.name} as main TeX file`}
              onclick={(e) => {
                e.stopPropagation();
                onSetEntryFile(node.path);
              }}
            >
              <Star size={12} aria-hidden="true" />
            </button>
          {/if}

          <button
            type="button"
            class="row-action-btn"
            title={`Rename ${node.name}`}
            aria-label={`Rename ${node.name}`}
            onclick={(e) => {
              e.stopPropagation();
              onRenameNode(node);
            }}
          >
            <Pencil size={12} aria-hidden="true" />
          </button>

          <button
            type="button"
            class="row-action-btn danger"
            title={`Delete ${node.name}`}
            aria-label={`Delete ${node.name}`}
            onclick={(e) => {
              e.stopPropagation();
              onDeleteNode(node);
            }}
          >
            <Trash size={12} aria-hidden="true" />
          </button>
        </div>
      </div>

      {#if node.isDir && isExpanded && node.children && node.children.length > 0}
        <ul class="tree-list" role="group">
          {@render renderNodes(node.children, depth + 1)}
        </ul>
      {/if}
    </li>
  {/each}
{/snippet}

<style>
  .sidebar {
    inline-size: 100%;
    block-size: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border-inline-end: 1px solid var(--border-subtle);
    min-inline-size: 0;
    user-select: none;
  }

  .sidebar-header {
    block-size: 38px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    padding: 0 8px 0 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .sidebar-title {
    margin: 0;
    font-size: 11.5px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .header-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 24px;
    block-size: 24px;
    padding: 0;
    border: none;
    border-radius: var(--radius-xs);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .header-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  @media (hover: hover) {
    .header-btn:not(:disabled):hover {
      background: var(--bg-hover);
      color: var(--text-primary);
    }
  }

  .tree-scroll {
    flex: 1;
    min-block-size: 0;
    padding: 6px 4px;
  }

  .tree-list {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .tree-row {
    display: flex;
    align-items: center;
    gap: 2px;
    min-block-size: 28px;
    padding-inline-end: 6px;
    border-radius: var(--radius-xs);
    position: relative;
  }

  @media (hover: hover) {
    .tree-row:hover {
      background: var(--bg-hover);
    }
  }

  .tree-row.active {
    background: var(--bg-active);
  }

  .tree-row.active .node-name {
    font-weight: 600;
    color: var(--text-primary);
  }

  .row-main {
    flex: 1;
    min-inline-size: 0;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 2px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    text-align: start;
    cursor: pointer;
  }

  .chevron,
  .chevron-spacer {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 14px;
    block-size: 14px;
    flex-shrink: 0;
    color: var(--text-muted);
    transition: transform 140ms ease;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .node-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: var(--text-muted);
  }

  .folder-icon {
    color: var(--accent);
  }

  :global(.icon-tex) {
    color: var(--accent);
  }

  :global(.icon-bib) {
    color: var(--warning);
  }

  :global(.icon-style) {
    color: #9333ea;
  }

  :global(.icon-pdf) {
    color: var(--danger);
  }

  :global(.icon-img) {
    color: #0d9488;
  }

  @supports (color: light-dark(white, black)) {
    :global(.icon-style) {
      color: light-dark(#a21caf, #e879f9);
    }

    :global(.icon-img) {
      color: light-dark(#0f766e, #2dd4bf);
    }
  }

  .node-name {
    flex: 1;
    min-inline-size: 0;
    font-size: 12.5px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dirty-dot {
    inline-size: 7px;
    block-size: 7px;
    border-radius: 50%;
    background: var(--warning);
    flex-shrink: 0;
  }

  .main-badge {
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.04em;
    flex-shrink: 0;
  }

  .row-actions {
    display: flex;
    align-items: center;
    gap: 1px;
    opacity: 0;
    pointer-events: none;
    flex-shrink: 0;
    transition: opacity 100ms ease;
  }

  .tree-row:hover .row-actions,
  .tree-row:focus-within .row-actions {
    opacity: 1;
    pointer-events: auto;
  }

  .row-action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 20px;
    block-size: 20px;
    padding: 0;
    border: none;
    border-radius: var(--radius-xs);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  @media (hover: hover) {
    .row-action-btn:hover {
      background: var(--bg-surface);
      color: var(--text-primary);
    }

    .row-action-btn.danger:hover {
      background: var(--danger-soft);
      color: var(--danger);
    }
  }

  .empty-state {
    padding: 24px 14px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 12px;
  }

  .empty-text {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-muted);
  }

  .open-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 13px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
  }

  @media (hover: hover) {
    .open-btn:hover {
      background: var(--accent-hover);
    }
  }
</style>
