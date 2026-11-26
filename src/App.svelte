<script lang="ts">
  import { onMount } from "svelte";
  import TopBar, { type ThemeMode } from "./lib/components/TopBar.svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import EditorPane from "./lib/components/EditorPane.svelte";
  import PdfPreview from "./lib/components/PdfPreview.svelte";
  import ModalDialog, { type ModalMode } from "./lib/components/ModalDialog.svelte";
  import UpdateModal, {
    type UpdateModalStatus,
  } from "./lib/components/UpdateModal.svelte";
  import type {
    AppUpdateInfo,
    FileNode,
    WorkspaceInfo,
    CompilerInfo,
    LatexDiagnostic,
    UpdateProgress,
  } from "./lib/types";
  import {
    initDefaultWorkspace,
    openWorkspace,
    listWorkspaceFiles,
    readTextFile,
    saveTextFile,
    createFile,
    renamePath,
    deletePath,
    exportPdf,
    detectCompilers,
    compileLatex,
    pickWorkspaceFolder,
    pickPdfSavePath,
    getCurrentAppVersion,
    checkForAppUpdate,
    downloadAndInstallAppUpdate,
    relaunchApp,
  } from "./lib/api";

  const THEME_STORAGE_KEY = "bex-theme";
  const COMPILABLE_EXTENSIONS = new Set([
    ".tex",
    ".ltx",
    ".bib",
    ".sty",
    ".cls",
    ".dtx",
    ".bst",
    ".def",
    ".tikz",
  ]);
  const BINARY_EXTENSIONS = new Set([
    ".pdf",
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".webp",
    ".ico",
    ".bmp",
    ".eps",
    ".gz",
    ".zip",
  ]);

  // Workspace & files (relative paths matching FileNode.path & CompileRequest.entryFile)
  let workspace = $state<WorkspaceInfo | null>(null);
  let files = $state<FileNode[]>([]);
  let entryFile = $state<string | null>(null);
  let activeFilePath = $state<string | null>(null);

  // Multi-file in-memory buffers keyed by FileNode.path (relative path)
  let savedContentByPath = $state<Record<string, string>>({});
  let draftContentByPath = $state<Record<string, string>>({});

  const dirtyPaths = $derived.by(() => {
    const dirty = new Set<string>();
    for (const [path, draft] of Object.entries(draftContentByPath)) {
      if (savedContentByPath[path] !== draft) {
        dirty.add(path);
      }
    }
    return dirty;
  });

  const activeContent = $derived(
    activeFilePath
      ? (draftContentByPath[activeFilePath] ??
          savedContentByPath[activeFilePath] ??
          "")
      : ""
  );

  const activeFileName = $derived(
    activeFilePath ? basename(activeFilePath) : null
  );

  const isActiveDirty = $derived(
    activeFilePath ? dirtyPaths.has(activeFilePath) : false
  );

  const texFiles = $derived.by(() => collectTexFiles(files));

  // Compilers & compilation output
  let compilers = $state<CompilerInfo[]>([]);
  let selectedEngine = $state<string>("auto");
  let autoCompileOnSave = $state<boolean>(true);
  let isCompiling = $state<boolean>(false);
  let pendingCompile = false;
  let compileSuccess = $state<boolean | null>(null);
  let lastCompileMs = $state<number | null>(null);
  let pdfBase64 = $state<string | null>(null);
  let pdfPath = $state<string | null>(null);
  let diagnostics = $state<LatexDiagnostic[]>([]);
  let logOutput = $state<string>("");
  let showLogs = $state<boolean>(false);
  let statusNotice = $state<string | null>(null);

  // Jump-to-line signal for CodeMirror
  let jumpSeq = 0;
  let jumpToLine = $state<{ line: number; seq: number } | null>(null);

  // Layout & Resizing
  let sidebarOpen = $state<boolean>(true);
  let sidebarWidth = $state<number>(248);
  let splitRatio = $state<number>(0.5);
  let isResizing = $state<boolean>(false);
  let workspaceAreaEl: HTMLElement | undefined = $state();
  let splitAreaEl: HTMLElement | undefined = $state();

  // Theme
  function loadInitialTheme(): ThemeMode {
    try {
      const saved = localStorage.getItem(THEME_STORAGE_KEY);
      if (saved === "light" || saved === "dark" || saved === "system") {
        return saved;
      }
    } catch {
      // Ignore storage failures
    }
    return "system";
  }

  let theme = $state<ThemeMode>(loadInitialTheme());

  // Modal Dialog state
  let modalOpen = $state<boolean>(false);
  let modalMode = $state<ModalMode>(null);
  let modalTargetName = $state<string>("");
  let modalParentLabel = $state<string>("");
  let modalParentRelDir = $state<string>("");
  let modalTargetNode = $state<FileNode | null>(null);

  // App Auto-Updater state
  let appVersion = $state<string>("0.1.0");
  let availableUpdate = $state<AppUpdateInfo | null>(null);
  let updateModalOpen = $state<boolean>(false);
  let updateStatus = $state<UpdateModalStatus>("idle");
  let updateProgress = $state<UpdateProgress | null>(null);
  let updateError = $state<string | null>(null);
  let updateCheckPromise: Promise<void> | null = null;
  let updateCheckSilent = true;

  // Sync theme to documentElement.dataset.theme, meta[name="color-scheme"], and localStorage
  $effect(() => {
    const root = document.documentElement;
    const meta = document.querySelector('meta[name="color-scheme"]');

    if (theme === "light" || theme === "dark") {
      root.dataset.theme = theme;
      if (meta) meta.setAttribute("content", theme);
    } else {
      delete root.dataset.theme;
      if (meta) meta.setAttribute("content", "light dark");
    }

    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // Ignore storage failures
    }
  });

  // Listen to OS color-scheme changes while in "system" mode
  $effect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handleMediaChange = () => {
      if (theme === "system") {
        delete document.documentElement.dataset.theme;
      }
    };
    mq.addEventListener("change", handleMediaChange);
    return () => mq.removeEventListener("change", handleMediaChange);
  });

  // Global keyboard shortcuts (Cmd/Ctrl+S save, Cmd/Ctrl+B toggle sidebar)
  $effect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      const isMod = event.metaKey || event.ctrlKey;
      if (!isMod || event.altKey) return;

      const key = event.key.toLowerCase();
      if (key === "s") {
        event.preventDefault();
        void handleSaveCurrentFile();
      } else if (key === "b") {
        event.preventDefault();
        sidebarOpen = !sidebarOpen;
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  onMount(() => {
    void bootstrap();
  });

  async function bootstrap() {
    try {
      const [detected, ws, version] = await Promise.all([
        detectCompilers().catch(() => [] as CompilerInfo[]),
        initDefaultWorkspace(),
        getCurrentAppVersion(),
      ]);
      compilers = detected;
      appVersion = version;
      void runUpdateCheck({ silent: true });
      await applyWorkspace(ws);
    } catch (err) {
      reportNotice(`Failed to initialize workspace: ${formatError(err)}`);
    }
  }

  async function runUpdateCheck(options: { silent: boolean }): Promise<void> {
    if (!options.silent) {
      updateCheckSilent = false;
      updateStatus = "checking";
      updateError = null;
      updateProgress = null;
    }

    if (updateCheckPromise) {
      return updateCheckPromise;
    }

    updateCheckSilent = options.silent;
    updateCheckPromise = (async () => {
      try {
        const info = await checkForAppUpdate();
        if (info.currentVersion) {
          appVersion = info.currentVersion;
        }
        if (info.available && info.version) {
          availableUpdate = info;
          if (!updateCheckSilent || updateModalOpen) {
            updateStatus = "available";
          }
        } else {
          availableUpdate = null;
          if (!updateCheckSilent) {
            updateStatus = "up-to-date";
          }
        }
      } catch (err) {
        if (!updateCheckSilent) {
          updateError = formatError(err);
          updateStatus = "error";
        }
      } finally {
        updateCheckPromise = null;
      }
    })();

    return updateCheckPromise;
  }

  function handleOpenUpdateModal() {
    updateModalOpen = true;
    if (updateStatus === "downloading" || updateStatus === "ready-to-restart") {
      return;
    }
    if (availableUpdate) {
      updateError = null;
      updateProgress = null;
      updateStatus = "available";
      return;
    }
    void runUpdateCheck({ silent: false });
  }

  async function handleDownloadAndInstall() {
    if (updateStatus === "downloading") return;
    updateStatus = "downloading";
    updateError = null;
    updateProgress = { downloadedBytes: 0, contentLength: null, percent: null };

    try {
      await downloadAndInstallAppUpdate((progress) => {
        updateProgress = progress;
      });
      updateStatus = "ready-to-restart";
    } catch (err) {
      updateError = formatError(err);
      updateStatus = "error";
    }
  }

  async function handleRelaunch() {
    try {
      await relaunchApp();
    } catch (err) {
      updateError = formatError(err);
      updateStatus = "error";
    }
  }

  function handleCloseUpdateModal() {
    if (updateStatus === "downloading") return;
    updateModalOpen = false;
  }

  async function applyWorkspace(ws: WorkspaceInfo) {
    workspace = ws;
    files = ws.files;
    savedContentByPath = {};
    draftContentByPath = {};
    pdfBase64 = null;
    pdfPath = null;
    compileSuccess = null;
    lastCompileMs = null;
    diagnostics = [];
    logOutput = "";
    statusNotice = null;

    const availableTex = collectTexFiles(ws.files);
    const initialEntry =
      ws.defaultEntryFile && availableTex.includes(ws.defaultEntryFile)
        ? ws.defaultEntryFile
        : (availableTex[0] ?? null);

    entryFile = initialEntry;
    activeFilePath = null;

    if (initialEntry) {
      await openFileInEditor(initialEntry);
      await runCompile();
    }
  }

  function findNodeByPath(nodes: FileNode[], relPath: string): FileNode | null {
    for (const node of nodes) {
      if (node.path === relPath) return node;
      if (node.isDir && node.children) {
        const hit = findNodeByPath(node.children, relPath);
        if (hit) return hit;
      }
    }
    return null;
  }

  function resolveAbsPath(relPath: string): string {
    const node = findNodeByPath(files, relPath);
    if (node?.absPath) return node.absPath;
    if (!workspace) return relPath;
    return joinPath(workspace.rootPath, relPath);
  }

  async function openFileInEditor(relPath: string) {
    const ext = extensionOf(relPath);
    if (BINARY_EXTENSIONS.has(ext)) {
      reportNotice(`Binary file "${basename(relPath)}" cannot be opened in the text editor.`);
      return;
    }

    if (relPath in draftContentByPath || relPath in savedContentByPath) {
      activeFilePath = relPath;
      return;
    }

    try {
      const absPath = resolveAbsPath(relPath);
      const text = await readTextFile(absPath);
      savedContentByPath = { ...savedContentByPath, [relPath]: text };
      draftContentByPath = { ...draftContentByPath, [relPath]: text };
      activeFilePath = relPath;
      statusNotice = null;
    } catch (err) {
      reportNotice(`Unable to read "${basename(relPath)}": ${formatError(err)}`);
    }
  }

  function handleEditorChange(newContent: string) {
    if (!activeFilePath) return;
    draftContentByPath = {
      ...draftContentByPath,
      [activeFilePath]: newContent,
    };
  }

  async function flushDirtyBuffers(): Promise<string[]> {
    const flushed: string[] = [];
    const paths = Array.from(dirtyPaths);
    for (const relPath of paths) {
      const content = draftContentByPath[relPath];
      if (content === undefined) continue;
      await saveTextFile(resolveAbsPath(relPath), content);
      savedContentByPath = { ...savedContentByPath, [relPath]: content };
      flushed.push(relPath);
    }
    return flushed;
  }

  async function handleSaveCurrentFile() {
    if (!workspace) return;

    try {
      const targetPath = activeFilePath;
      if (targetPath && dirtyPaths.has(targetPath)) {
        const content = draftContentByPath[targetPath] ?? "";
        await saveTextFile(resolveAbsPath(targetPath), content);
        savedContentByPath = { ...savedContentByPath, [targetPath]: content };
      }

      const ext = targetPath ? extensionOf(targetPath) : "";
      const shouldCompile =
        autoCompileOnSave &&
        Boolean(entryFile) &&
        (!targetPath || COMPILABLE_EXTENSIONS.has(ext) || ext === "");

      if (shouldCompile) {
        await runCompile();
      }
    } catch (err) {
      reportNotice(`Save failed: ${formatError(err)}`);
    }
  }

  async function runCompile() {
    if (!workspace || !entryFile) return;

    if (isCompiling) {
      pendingCompile = true;
      return;
    }

    isCompiling = true;
    statusNotice = null;
    const startedAt = performance.now();

    try {
      // Flush all modified buffers so multi-file \input edits compile cleanly
      await flushDirtyBuffers();

      const res = await compileLatex({
        rootDir: workspace.rootPath,
        entryFile,
        engine: selectedEngine,
      });

      const elapsed =
        typeof res.durationMs === "number"
          ? Math.round(res.durationMs)
          : Math.round(performance.now() - startedAt);

      compileSuccess = res.success;
      lastCompileMs = elapsed;
      diagnostics = res.diagnostics ?? [];
      logOutput = res.logOutput ?? "";
      if (res.pdfPath) {
        pdfPath = res.pdfPath;
      }
      if (res.pdfBase64) {
        pdfBase64 = res.pdfBase64;
      }
    } catch (err) {
      compileSuccess = false;
      const msg = formatError(err);
      logOutput = msg;
      diagnostics = [
        {
          severity: "error",
          line: null,
          message: msg,
          file: entryFile,
        },
      ];
    } finally {
      isCompiling = false;
      if (pendingCompile) {
        pendingCompile = false;
        void runCompile();
      }
    }
  }

  async function handleOpenFolder() {
    try {
      const picked = await pickWorkspaceFolder();
      if (!picked) return;
      const ws = await openWorkspace(picked);
      await applyWorkspace(ws);
    } catch (err) {
      reportNotice(`Could not open folder: ${formatError(err)}`);
    }
  }

  async function refreshFileTree() {
    if (!workspace) return;
    try {
      const updated = await listWorkspaceFiles(workspace.rootPath);
      files = updated;
    } catch (err) {
      reportNotice(`Refresh failed: ${formatError(err)}`);
    }
  }

  async function handleExportPdf() {
    if (!pdfPath) return;
    try {
      const base = entryFile ? basename(entryFile).replace(/\.[^.]+$/, "") : "document";
      const dest = await pickPdfSavePath(`${base}.pdf`);
      if (!dest) return;
      await exportPdf(pdfPath, dest);
      reportNotice(`Exported PDF to ${basename(dest)}`);
    } catch (err) {
      reportNotice(`PDF export failed: ${formatError(err)}`);
    }
  }

  async function handleJumpToLine(line: number, file?: string | null) {
    if (file) {
      const resolved = resolveWorkspaceFilePath(file);
      if (resolved && resolved !== activeFilePath) {
        await openFileInEditor(resolved);
      }
    }
    jumpToLine = { line, seq: ++jumpSeq };
  }

  function resolveWorkspaceFilePath(fileHint: string): string | null {
    const normalizedHint = fileHint.replace(/\\/g, "/").replace(/^\.\//, "");
    const allNodes = collectAllFileNodes(files);
    const exactRel = allNodes.find((n) => n.path === normalizedHint);
    if (exactRel) return exactRel.path;

    const exactAbs = allNodes.find((n) => n.absPath === fileHint);
    if (exactAbs) return exactAbs.path;

    const suffixOrName = allNodes.find(
      (n) =>
        n.path.endsWith("/" + normalizedHint) ||
        n.name === basename(normalizedHint)
    );
    return suffixOrName?.path ?? null;
  }

  function handleCycleTheme() {
    if (theme === "system") theme = "light";
    else if (theme === "light") theme = "dark";
    else theme = "system";
  }

  // Modal Triggers
  function openCreateModal(
    kind: "create-file" | "create-folder",
    parentRelDir = ""
  ) {
    if (!workspace) return;
    const cleanParent = parentRelDir.replace(/^\/+|\/+$/g, "");
    modalMode = kind;
    modalParentRelDir = cleanParent;
    modalParentLabel = cleanParent
      ? `${workspace.name}/${cleanParent}`
      : workspace.name;
    modalTargetName = "";
    modalTargetNode = null;
    modalOpen = true;
  }

  function openRenameModal(node: FileNode) {
    if (!workspace) return;
    modalMode = "rename";
    modalTargetNode = node;
    modalTargetName = node.name;
    modalParentRelDir = dirname(node.path);
    modalParentLabel = `${workspace.name}/${node.path}`;
    modalOpen = true;
  }

  function openDeleteModal(node: FileNode) {
    if (!workspace) return;
    modalMode = "delete";
    modalTargetNode = node;
    modalTargetName = node.name;
    modalParentRelDir = dirname(node.path);
    modalParentLabel = `${workspace.name}/${node.path}`;
    modalOpen = true;
  }

  function closeModal() {
    modalOpen = false;
    modalMode = null;
    modalTargetNode = null;
  }

  async function handleModalConfirm(value?: string) {
    const currentMode = modalMode;
    const targetNode = modalTargetNode;
    const parentRel = modalParentRelDir;
    closeModal();

    if (!workspace || !currentMode) return;

    try {
      if (currentMode === "create-file" && value) {
        const cleanRel = value.replace(/^\/+/, "");
        const newRelPath = parentRel ? `${parentRel}/${cleanRel}` : cleanRel;
        const absPath = joinPath(workspace.rootPath, newRelPath);
        await createFile(absPath, false);
        await refreshFileTree();
        if (!entryFile && extensionOf(newRelPath) === ".tex") {
          entryFile = newRelPath;
        }
        await openFileInEditor(newRelPath);
      } else if (currentMode === "create-folder" && value) {
        const cleanRel = value.replace(/^\/+/, "");
        const newRelPath = parentRel ? `${parentRel}/${cleanRel}` : cleanRel;
        const absPath = joinPath(workspace.rootPath, newRelPath);
        await createFile(absPath, true);
        await refreshFileTree();
      } else if (currentMode === "rename" && targetNode && value) {
        const oldRelPath = targetNode.path;
        const parentOfOld = dirname(oldRelPath);
        const newRelPath = parentOfOld ? `${parentOfOld}/${value}` : value;
        if (oldRelPath === newRelPath) return;

        const oldAbs = targetNode.absPath || resolveAbsPath(oldRelPath);
        const newAbs = joinPath(workspace.rootPath, newRelPath);
        await renamePath(oldAbs, newAbs);
        migrateRenamedState(oldRelPath, newRelPath, targetNode.isDir);
        await refreshFileTree();
      } else if (currentMode === "delete" && targetNode) {
        const targetRelPath = targetNode.path;
        const targetAbs = targetNode.absPath || resolveAbsPath(targetRelPath);
        await deletePath(targetAbs);
        pruneDeletedState(targetRelPath, targetNode.isDir);
        await refreshFileTree();
        await ensureValidEntryAndActiveFile();
      }
    } catch (err) {
      reportNotice(`Operation failed: ${formatError(err)}`);
    }
  }

  function migrateRenamedState(oldPath: string, newPath: string, isDir: boolean) {
    const remapPath = (p: string): string => {
      if (p === oldPath) return newPath;
      if (isDir && p.startsWith(`${oldPath}/`)) {
        return `${newPath}${p.slice(oldPath.length)}`;
      }
      return p;
    };

    const nextSaved: Record<string, string> = {};
    for (const [k, v] of Object.entries(savedContentByPath)) {
      nextSaved[remapPath(k)] = v;
    }
    savedContentByPath = nextSaved;

    const nextDraft: Record<string, string> = {};
    for (const [k, v] of Object.entries(draftContentByPath)) {
      nextDraft[remapPath(k)] = v;
    }
    draftContentByPath = nextDraft;

    if (activeFilePath) activeFilePath = remapPath(activeFilePath);
    if (entryFile) entryFile = remapPath(entryFile);
  }

  function pruneDeletedState(targetPath: string, isDir: boolean) {
    const matchesTarget = (p: string): boolean =>
      p === targetPath || (isDir && p.startsWith(`${targetPath}/`));

    const nextSaved: Record<string, string> = {};
    for (const [k, v] of Object.entries(savedContentByPath)) {
      if (!matchesTarget(k)) nextSaved[k] = v;
    }
    savedContentByPath = nextSaved;

    const nextDraft: Record<string, string> = {};
    for (const [k, v] of Object.entries(draftContentByPath)) {
      if (!matchesTarget(k)) nextDraft[k] = v;
    }
    draftContentByPath = nextDraft;

    if (activeFilePath && matchesTarget(activeFilePath)) {
      activeFilePath = null;
    }
    if (entryFile && matchesTarget(entryFile)) {
      entryFile = null;
    }
  }

  async function ensureValidEntryAndActiveFile() {
    const remainingTex = collectTexFiles(files);
    if (!entryFile && remainingTex.length > 0) {
      entryFile = remainingTex[0];
    }
    if (!activeFilePath) {
      const nextOpen =
        entryFile ?? remainingTex[0] ?? collectAllFileNodes(files)[0]?.path;
      if (nextOpen) {
        await openFileInEditor(nextOpen);
      }
    }
  }

  // Pointer Resize Handlers
  function startSidebarResize(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);
    isResizing = true;

    const onMove = (e: PointerEvent) => {
      if (!workspaceAreaEl) return;
      const rect = workspaceAreaEl.getBoundingClientRect();
      const nextWidth = Math.min(420, Math.max(180, Math.round(e.clientX - rect.left)));
      sidebarWidth = nextWidth;
    };

    const onUp = (e: PointerEvent) => {
      isResizing = false;
      try {
        handle.releasePointerCapture(e.pointerId);
      } catch {
        // Ignore if already released
      }
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
      handle.removeEventListener("pointercancel", onUp);
    };

    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
    handle.addEventListener("pointercancel", onUp);
  }

  function startSplitResize(event: PointerEvent) {
    const handle = event.currentTarget as HTMLElement;
    handle.setPointerCapture(event.pointerId);
    isResizing = true;

    const onMove = (e: PointerEvent) => {
      if (!splitAreaEl) return;
      const rect = splitAreaEl.getBoundingClientRect();
      if (rect.width <= 0) return;
      const ratio = (e.clientX - rect.left) / rect.width;
      splitRatio = Math.min(0.75, Math.max(0.25, ratio));
    };

    const onUp = (e: PointerEvent) => {
      isResizing = false;
      try {
        handle.releasePointerCapture(e.pointerId);
      } catch {
        // Ignore if already released
      }
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
      handle.removeEventListener("pointercancel", onUp);
    };

    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
    handle.addEventListener("pointercancel", onUp);
  }

  function handleSidebarResizerKey(e: KeyboardEvent) {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      sidebarWidth = Math.max(180, sidebarWidth - 16);
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      sidebarWidth = Math.min(420, sidebarWidth + 16);
    }
  }

  function handleSplitResizerKey(e: KeyboardEvent) {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      splitRatio = Math.max(0.25, splitRatio - 0.03);
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      splitRatio = Math.min(0.75, splitRatio + 0.03);
    }
  }

  // Helpers
  function collectTexFiles(nodes: FileNode[]): string[] {
    const out: string[] = [];
    for (const node of nodes) {
      if (node.isDir) {
        if (node.children) out.push(...collectTexFiles(node.children));
      } else {
        const ext = extensionOf(node.name);
        if (ext === ".tex" || ext === ".ltx") {
          out.push(node.path);
        }
      }
    }
    return out;
  }

  function collectAllFileNodes(nodes: FileNode[]): FileNode[] {
    const out: FileNode[] = [];
    for (const node of nodes) {
      if (node.isDir) {
        if (node.children) out.push(...collectAllFileNodes(node.children));
      } else {
        out.push(node);
      }
    }
    return out;
  }

  function basename(p: string): string {
    const norm = p.replace(/\\/g, "/").replace(/\/+$/, "");
    const idx = norm.lastIndexOf("/");
    return idx >= 0 ? norm.slice(idx + 1) : norm;
  }

  function dirname(p: string): string {
    const norm = p.replace(/\\/g, "/").replace(/\/+$/, "");
    const idx = norm.lastIndexOf("/");
    return idx > 0 ? norm.slice(0, idx) : "";
  }

  function joinPath(base: string, child: string): string {
    const cleanBase = base.replace(/[\\/]+$/, "");
    const cleanChild = child.replace(/^[\\/]+/, "");
    return `${cleanBase}/${cleanChild}`;
  }

  function extensionOf(p: string): string {
    const name = basename(p);
    const idx = name.lastIndexOf(".");
    return idx >= 0 ? name.slice(idx).toLowerCase() : "";
  }

  function formatError(err: unknown): string {
    if (err instanceof Error) return err.message;
    if (typeof err === "string") return err;
    try {
      return JSON.stringify(err);
    } catch {
      return String(err);
    }
  }

  function reportNotice(message: string) {
    statusNotice = message;
  }
</script>

<div class="app-shell" class:resizing={isResizing}>
  <TopBar
    projectName={workspace?.name ?? "No Project"}
    {sidebarOpen}
    onToggleSidebar={() => (sidebarOpen = !sidebarOpen)}
    {texFiles}
    workspaceRoot={workspace?.rootPath ?? null}
    {entryFile}
    onChangeEntryFile={(path) => {
      entryFile = path;
      void runCompile();
    }}
    {compilers}
    {selectedEngine}
    onChangeEngine={(engine) => {
      selectedEngine = engine;
    }}
    {autoCompileOnSave}
    onToggleAutoCompile={(enabled) => {
      autoCompileOnSave = enabled;
    }}
    {isCompiling}
    {compileSuccess}
    {lastCompileMs}
    {diagnostics}
    isDirty={isActiveDirty}
    hasAnyDirty={dirtyPaths.size > 0}
    hasPdf={Boolean(pdfPath)}
    {theme}
    {appVersion}
    availableUpdateVersion={availableUpdate?.version ?? null}
    onCycleTheme={handleCycleTheme}
    onOpenFolder={handleOpenFolder}
    onNewFile={() => openCreateModal("create-file", "")}
    onNewFolder={() => openCreateModal("create-folder", "")}
    onSave={handleSaveCurrentFile}
    onRecompile={runCompile}
    onExportPdf={handleExportPdf}
    onToggleLogs={() => (showLogs = !showLogs)}
    onCheckUpdates={handleOpenUpdateModal}
  />

  {#if statusNotice}
    <div class="notice-banner" role="status">
      <span>{statusNotice}</span>
      <button
        type="button"
        class="notice-dismiss"
        aria-label="Dismiss message"
        onclick={() => (statusNotice = null)}
      >
        Dismiss
      </button>
    </div>
  {/if}

  <main class="workspace-panes" bind:this={workspaceAreaEl}>
    {#if sidebarOpen}
      <div class="pane-sidebar" style={`inline-size: ${sidebarWidth}px`}>
        <Sidebar
          {workspace}
          {files}
          {activeFilePath}
          {entryFile}
          {dirtyPaths}
          onSelectFile={openFileInEditor}
          onSetEntryFile={(path) => {
            entryFile = path;
            void runCompile();
          }}
          onOpenFolder={handleOpenFolder}
          onCreateFile={(dir) => openCreateModal("create-file", dir ?? "")}
          onCreateFolder={(dir) => openCreateModal("create-folder", dir ?? "")}
          onRenameNode={openRenameModal}
          onDeleteNode={openDeleteModal}
          onRefresh={refreshFileTree}
        />
      </div>

      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="resizer"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize project sidebar"
        aria-valuemin={180}
        aria-valuemax={420}
        aria-valuenow={sidebarWidth}
        tabindex="0"
        onpointerdown={startSidebarResize}
        onkeydown={handleSidebarResizerKey}
      ></div>
    {/if}

    <div class="split-area" bind:this={splitAreaEl}>
      <section
        class="pane-editor"
        aria-label="LaTeX source editor"
        style={`flex: 0 0 calc(${(splitRatio * 100).toFixed(3)}% - 3px)`}
      >
        <EditorPane
          content={activeContent}
          filePath={activeFilePath}
          fileName={activeFileName}
          isDirty={isActiveDirty}
          {diagnostics}
          {jumpToLine}
          {theme}
          onChange={handleEditorChange}
          onSaveAndCompile={handleSaveCurrentFile}
        />
      </section>

      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="resizer"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize editor and PDF preview"
        aria-valuemin={25}
        aria-valuemax={75}
        aria-valuenow={Math.round(splitRatio * 100)}
        tabindex="0"
        onpointerdown={startSplitResize}
        onkeydown={handleSplitResizerKey}
      ></div>

      <section class="pane-preview" aria-label="PDF preview">
        <PdfPreview
          {pdfBase64}
          {pdfPath}
          {isCompiling}
          {compileSuccess}
          {diagnostics}
          {logOutput}
          {showLogs}
          onToggleLogs={() => (showLogs = !showLogs)}
          onJumpToLine={handleJumpToLine}
          onExportPdf={handleExportPdf}
          onRecompile={runCompile}
        />
      </section>
    </div>
  </main>

  <ModalDialog
    open={modalOpen}
    mode={modalMode}
    targetName={modalTargetName}
    parentLabel={modalParentLabel}
    onConfirm={handleModalConfirm}
    onCancel={closeModal}
  />

  <UpdateModal
    open={updateModalOpen}
    status={updateStatus}
    currentVersion={appVersion}
    updateInfo={availableUpdate}
    progress={updateProgress}
    errorMessage={updateError}
    onCheckAgain={() => void runUpdateCheck({ silent: false })}
    onDownloadAndInstall={() => void handleDownloadAndInstall()}
    onRelaunch={() => void handleRelaunch()}
    onClose={handleCloseUpdateModal}
  />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    inline-size: 100%;
    block-size: 100dvb;
    overflow: hidden;
    background: var(--bg-app);
  }

  .app-shell.resizing {
    cursor: col-resize;
    user-select: none;
  }

  .notice-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 6px 14px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-primary);
    font-size: 12px;
    flex-shrink: 0;
  }

  .notice-dismiss {
    padding: 2px 8px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-secondary);
    font-size: 11px;
    cursor: pointer;
  }

  .workspace-panes {
    flex: 1;
    min-block-size: 0;
    display: flex;
    overflow: hidden;
  }

  .pane-sidebar {
    flex-shrink: 0;
    block-size: 100%;
    min-block-size: 0;
    overflow: hidden;
  }

  .split-area {
    flex: 1;
    min-inline-size: 0;
    min-block-size: 0;
    display: flex;
    overflow: hidden;
  }

  .pane-editor {
    min-inline-size: 0;
    min-block-size: 0;
    block-size: 100%;
    overflow: hidden;
  }

  .pane-preview {
    flex: 1 1 0;
    min-inline-size: 0;
    min-block-size: 0;
    block-size: 100%;
    overflow: hidden;
  }

  .resizer {
    inline-size: 6px;
    flex-shrink: 0;
    background: var(--bg-app);
    border-inline: 1px solid var(--border-subtle);
    cursor: col-resize;
    position: relative;
    transition: background-color 120ms ease;
    touch-action: none;
  }

  @media (hover: hover) {
    .resizer:hover {
      background: var(--accent-soft);
      border-color: var(--accent);
    }
  }

  .resizer:focus-visible {
    outline: none;
    background: var(--accent-soft);
    border-color: var(--accent);
  }
</style>
