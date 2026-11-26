<script lang="ts">
  import { onMount } from "svelte";
  import * as pdfjsLib from "pdfjs-dist";
  import pdfWorkerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import type { PDFDocumentProxy, RenderTask } from "pdfjs-dist";
  import {
    ChevronLeft,
    ChevronRight,
    CircleAlert,
    CircleCheck,
    Download,
    FileText,
    Info,
    LoaderCircle,
    MoveHorizontal,
    PanelBottom,
    PanelBottomClose,
    RefreshCw,
    Terminal,
    TriangleAlert,
    ZoomIn,
    ZoomOut,
  } from "@lucide/svelte";
  import type { LatexDiagnostic } from "../types";

  pdfjsLib.GlobalWorkerOptions.workerSrc = pdfWorkerUrl;

  interface Props {
    pdfBase64: string | null;
    pdfPath: string | null;
    isCompiling: boolean;
    compileSuccess: boolean | null;
    diagnostics: LatexDiagnostic[];
    logOutput: string;
    showLogs: boolean;
    onToggleLogs: () => void;
    onJumpToLine?: (line: number, file?: string | null) => void;
    onExportPdf: () => void;
    onRecompile: () => void;
  }

  let {
    pdfBase64,
    pdfPath,
    isCompiling,
    compileSuccess,
    diagnostics,
    logOutput,
    showLogs,
    onToggleLogs,
    onJumpToLine,
    onExportPdf,
    onRecompile,
  }: Props = $props();

  const CSS_UNITS = 96 / 72;
  const MIN_ZOOM = 0.4;
  const MAX_ZOOM = 3.0;
  const ZOOM_STEP = 0.15;

  let viewportEl: HTMLDivElement | undefined = $state();
  let pagesEl: HTMLDivElement | undefined = $state();

  let totalPages = $state(0);
  let currentPage = $state(1);
  let fitWidth = $state(true);
  let customZoom = $state(1.0);
  let effectiveZoom = $state(1.0);
  let renderError = $state<string | null>(null);
  let activeDrawerTab = $state<"issues" | "log">("issues");
  let issuesTabEl: HTMLButtonElement | undefined = $state();
  let logTabEl: HTMLButtonElement | undefined = $state();

  let currentDoc: PDFDocumentProxy | null = null;
  let lastLoadedBase64: string | null = null;
  let activeRenderTasks: RenderTask[] = [];
  let renderGeneration = 0;
  let resizeTimer: ReturnType<typeof setTimeout> | null = null;

  const errorCount = $derived(diagnostics.filter((d) => d.severity === "error").length);
  const warningCount = $derived(diagnostics.filter((d) => d.severity === "warning").length);
  const zoomPercent = $derived(Math.round(effectiveZoom * 100));

  function base64ToUint8Array(base64: string): Uint8Array {
    const clean = base64.replace(/^data:application\/pdf;base64,/, "");
    const binary = atob(clean);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes;
  }

  function cancelActiveRenders(): void {
    for (const task of activeRenderTasks) {
      try {
        task.cancel();
      } catch {
        // Ignore already-settled render tasks
      }
    }
    activeRenderTasks = [];
  }

  async function renderDocumentPages(doc: PDFDocumentProxy, generation: number): Promise<void> {
    if (!pagesEl || !viewportEl) return;

    cancelActiveRenders();

    const firstPage = await doc.getPage(1);
    if (generation !== renderGeneration) return;

    const baseViewport = firstPage.getViewport({ scale: 1 });
    let targetZoom = customZoom;
    if (fitWidth) {
      const availWidth = Math.max(220, viewportEl.clientWidth - 48);
      targetZoom = Math.min(
        MAX_ZOOM,
        Math.max(MIN_ZOOM, availWidth / (baseViewport.width * CSS_UNITS))
      );
    }

    effectiveZoom = targetZoom;
    const renderScale = targetZoom * CSS_UNITS;
    const dpr = Math.min(Math.max(window.devicePixelRatio || 1, 1), 3);
    const transform = dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : undefined;

    const cards: HTMLDivElement[] = [];

    for (let pageNum = 1; pageNum <= doc.numPages; pageNum++) {
      if (generation !== renderGeneration) return;

      const page = pageNum === 1 ? firstPage : await doc.getPage(pageNum);
      if (generation !== renderGeneration) return;

      const cssViewport = page.getViewport({ scale: renderScale });
      const cssWidth = Math.floor(cssViewport.width);
      const cssHeight = Math.floor(cssViewport.height);

      const card = document.createElement("div");
      card.className = "pdf-page-card";
      card.dataset.pageNumber = String(pageNum);
      card.style.width = `${cssWidth}px`;
      card.style.height = `${cssHeight}px`;

      const canvas = document.createElement("canvas");
      canvas.width = Math.floor(cssViewport.width * dpr);
      canvas.height = Math.floor(cssViewport.height * dpr);
      canvas.style.width = `${cssWidth}px`;
      canvas.style.height = `${cssHeight}px`;
      canvas.setAttribute("aria-label", `PDF Page ${pageNum} of ${doc.numPages}`);

      card.appendChild(canvas);
      cards.push(card);

      const task = page.render({
        canvas,
        viewport: cssViewport,
        transform,
      });
      activeRenderTasks.push(task);

      try {
        await task.promise;
      } catch (err) {
        if (err instanceof Error && err.name === "RenderingCancelledException") {
          return;
        }
        throw err;
      }
    }

    if (generation !== renderGeneration || !pagesEl || !viewportEl) return;

    // Save scroll position right before swapping rendered canvases so live edits stay in place
    const savedScrollTop = viewportEl.scrollTop;
    const savedScrollLeft = viewportEl.scrollLeft;

    pagesEl.replaceChildren(...cards);

    const maxTop = Math.max(0, viewportEl.scrollHeight - viewportEl.clientHeight);
    const maxLeft = Math.max(0, viewportEl.scrollWidth - viewportEl.clientWidth);
    viewportEl.scrollTop = Math.min(savedScrollTop, maxTop);
    viewportEl.scrollLeft = Math.min(savedScrollLeft, maxLeft);

    requestAnimationFrame(() => {
      if (!viewportEl) return;
      const clampTop = Math.max(0, viewportEl.scrollHeight - viewportEl.clientHeight);
      const clampLeft = Math.max(0, viewportEl.scrollWidth - viewportEl.clientWidth);
      viewportEl.scrollTop = Math.min(savedScrollTop, clampTop);
      viewportEl.scrollLeft = Math.min(savedScrollLeft, clampLeft);
      updateCurrentPageFromScroll();
    });
  }

  async function loadAndRenderPdf(base64: string): Promise<void> {
    const generation = ++renderGeneration;
    renderError = null;
    cancelActiveRenders();

    const loadingTask = pdfjsLib.getDocument({
      data: base64ToUint8Array(base64),
    });

    try {
      const nextDoc = await loadingTask.promise;
      if (generation !== renderGeneration) {
        await nextDoc.destroy();
        return;
      }

      const prevDoc = currentDoc;
      currentDoc = nextDoc;
      totalPages = nextDoc.numPages;
      currentPage = Math.min(Math.max(1, currentPage), nextDoc.numPages);
      if (prevDoc) {
        await prevDoc.destroy();
      }

      await renderDocumentPages(nextDoc, generation);
    } catch (err) {
      if (generation !== renderGeneration) return;
      renderError = err instanceof Error ? err.message : "Failed to decode PDF preview.";
    }
  }

  function updateCurrentPageFromScroll(): void {
    if (!viewportEl || !pagesEl || totalPages === 0) return;
    const cards = pagesEl.querySelectorAll<HTMLDivElement>(".pdf-page-card");
    if (cards.length === 0) return;

    const targetY = viewportEl.scrollTop + viewportEl.clientHeight * 0.35;
    let detected = 1;

    for (const card of cards) {
      if (card.offsetTop <= targetY) {
        detected = Number(card.dataset.pageNumber) || detected;
      } else {
        break;
      }
    }
    currentPage = detected;
  }

  function scrollToPage(pageNum: number): void {
    if (!viewportEl || !pagesEl || totalPages === 0) return;
    const clamped = Math.max(1, Math.min(pageNum, totalPages));
    const card = pagesEl.querySelector<HTMLDivElement>(
      `.pdf-page-card[data-page-number="${clamped}"]`
    );
    if (card) {
      card.scrollIntoView({ behavior: "smooth", block: "start" });
      currentPage = clamped;
    }
  }

  function zoomIn(): void {
    fitWidth = false;
    customZoom = Math.min(MAX_ZOOM, Math.round((effectiveZoom + ZOOM_STEP) * 100) / 100);
    if (currentDoc) {
      const generation = ++renderGeneration;
      void renderDocumentPages(currentDoc, generation);
    }
  }

  function zoomOut(): void {
    fitWidth = false;
    customZoom = Math.max(MIN_ZOOM, Math.round((effectiveZoom - ZOOM_STEP) * 100) / 100);
    if (currentDoc) {
      const generation = ++renderGeneration;
      void renderDocumentPages(currentDoc, generation);
    }
  }

  function toggleFitWidth(): void {
    fitWidth = !fitWidth;
    if (!fitWidth) {
      customZoom = effectiveZoom;
    }
    if (currentDoc) {
      const generation = ++renderGeneration;
      void renderDocumentPages(currentDoc, generation);
    }
  }

  function handleTabKeydown(event: KeyboardEvent): void {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight" && event.key !== "Home" && event.key !== "End") {
      return;
    }
    event.preventDefault();
    if (event.key === "Home") {
      activeDrawerTab = "issues";
      issuesTabEl?.focus();
    } else if (event.key === "End") {
      activeDrawerTab = "log";
      logTabEl?.focus();
    } else {
      activeDrawerTab = activeDrawerTab === "issues" ? "log" : "issues";
      if (activeDrawerTab === "issues") {
        issuesTabEl?.focus();
      } else {
        logTabEl?.focus();
      }
    }
  }

  function handleDiagnosticClick(diag: LatexDiagnostic): void {
    if (diag.line != null && diag.line >= 1) {
      onJumpToLine?.(diag.line, diag.file);
    }
  }

  onMount(() => {
    const observer = new ResizeObserver(() => {
      if (!fitWidth || !currentDoc) return;
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        if (currentDoc && fitWidth) {
          const generation = ++renderGeneration;
          void renderDocumentPages(currentDoc, generation);
        }
      }, 100);
    });

    if (viewportEl) {
      observer.observe(viewportEl);
    }

    return () => {
      observer.disconnect();
      if (resizeTimer) clearTimeout(resizeTimer);
      cancelActiveRenders();
      void currentDoc?.destroy();
      currentDoc = null;
    };
  });

  $effect(() => {
    const nextPdf = pdfBase64;
    if (nextPdf === lastLoadedBase64) return;
    lastLoadedBase64 = nextPdf;

    if (!nextPdf) {
      cancelActiveRenders();
      void currentDoc?.destroy();
      currentDoc = null;
      totalPages = 0;
      currentPage = 1;
      pagesEl?.replaceChildren();
      return;
    }

    void loadAndRenderPdf(nextPdf);
  });
</script>

<section class="pdf-preview" aria-label="PDF Live Preview">
  <header class="pdf-toolbar">
    <div class="toolbar-group">
      <button
        type="button"
        class="tool-btn"
        title="Zoom Out"
        aria-label="Zoom out"
        disabled={!pdfBase64 || effectiveZoom <= MIN_ZOOM}
        onclick={zoomOut}
      >
        <ZoomOut size={15} aria-hidden="true" />
      </button>

      <span class="zoom-readout" aria-label="Current zoom level">{zoomPercent}%</span>

      <button
        type="button"
        class="tool-btn"
        title="Zoom In"
        aria-label="Zoom in"
        disabled={!pdfBase64 || effectiveZoom >= MAX_ZOOM}
        onclick={zoomIn}
      >
        <ZoomIn size={15} aria-hidden="true" />
      </button>

      <button
        type="button"
        class="tool-btn fit-btn"
        class:active={fitWidth}
        title="Fit to Width"
        aria-label="Fit PDF page to width"
        aria-pressed={fitWidth}
        disabled={!pdfBase64}
        onclick={toggleFitWidth}
      >
        <MoveHorizontal size={14} aria-hidden="true" />
        <span>Fit Width</span>
      </button>
    </div>

    <div class="toolbar-group page-nav">
      <button
        type="button"
        class="tool-btn"
        title="Previous Page"
        aria-label="Previous page"
        disabled={!pdfBase64 || currentPage <= 1}
        onclick={() => scrollToPage(currentPage - 1)}
      >
        <ChevronLeft size={15} aria-hidden="true" />
      </button>
      <span class="page-readout">
        {#if totalPages > 0}
          Page {currentPage} / {totalPages}
        {:else}
          Page 0 / 0
        {/if}
      </span>
      <button
        type="button"
        class="tool-btn"
        title="Next Page"
        aria-label="Next page"
        disabled={!pdfBase64 || currentPage >= totalPages}
        onclick={() => scrollToPage(currentPage + 1)}
      >
        <ChevronRight size={15} aria-hidden="true" />
      </button>
    </div>

    <div class="toolbar-group">
      <button
        type="button"
        class="tool-btn"
        title="Recompile Project"
        aria-label="Recompile project"
        disabled={isCompiling}
        onclick={onRecompile}
      >
        <RefreshCw size={14} class={isCompiling ? "spin" : ""} aria-hidden="true" />
      </button>

      <button
        type="button"
        class="tool-btn"
        title="Export / Download PDF"
        aria-label="Download PDF"
        disabled={!pdfBase64 && !pdfPath}
        onclick={onExportPdf}
      >
        <Download size={15} aria-hidden="true" />
      </button>

      <button
        type="button"
        class="tool-btn logs-toggle"
        class:active={showLogs}
        title="Toggle Build Logs & Diagnostics"
        aria-label="Toggle build logs and diagnostics"
        aria-expanded={showLogs}
        onclick={onToggleLogs}
      >
        {#if showLogs}
          <PanelBottomClose size={15} aria-hidden="true" />
        {:else}
          <PanelBottom size={15} aria-hidden="true" />
        {/if}
        <span>Logs</span>
        {#if errorCount > 0}
          <span class="badge badge-error" title="{errorCount} errors">{errorCount}</span>
        {/if}
        {#if warningCount > 0}
          <span class="badge badge-warning" title="{warningCount} warnings">{warningCount}</span>
        {/if}
      </button>
    </div>
  </header>

  <div class="preview-stage">
    {#if isCompiling}
      <div class="compile-progress" role="status" aria-live="polite">
        <div class="progress-bar"></div>
        <div class="progress-pill">
          <LoaderCircle size={13} class="spin" aria-hidden="true" />
          <span>Compiling LaTeX…</span>
        </div>
      </div>
    {/if}

    <div
      class="pdf-viewport"
      bind:this={viewportEl}
      onscroll={updateCurrentPageFromScroll}
    >
      {#if renderError}
        <div class="state-card error-card" role="alert">
          <CircleAlert size={28} aria-hidden="true" />
          <h3>Unable to render PDF</h3>
          <p>{renderError}</p>
          <button type="button" class="action-btn" onclick={onRecompile}>
            Recompile
          </button>
        </div>
      {:else if !pdfBase64}
        {#if compileSuccess === false}
          <div class="state-card error-card" role="alert">
            <CircleAlert size={30} aria-hidden="true" />
            <h3>Compilation Failed</h3>
            <p>
              {errorCount > 0
                ? `${errorCount} error${errorCount === 1 ? "" : "s"} prevented PDF output.`
                : "No PDF output was produced by the compiler."}
            </p>
            <div class="state-actions">
              <button type="button" class="action-btn secondary" onclick={onToggleLogs}>
                <Terminal size={14} aria-hidden="true" />
                <span>{showLogs ? "Hide Logs" : "View Logs"}</span>
              </button>
              <button type="button" class="action-btn" onclick={onRecompile}>
                <RefreshCw size={14} aria-hidden="true" />
                <span>Retry Compile</span>
              </button>
            </div>
          </div>
        {:else}
          <div class="state-card">
            <FileText size={32} aria-hidden="true" />
            <h3>No PDF Preview Yet</h3>
            <p>Press <strong>Cmd/Ctrl + S</strong> in the editor or compile to preview your document.</p>
            <button
              type="button"
              class="action-btn"
              disabled={isCompiling}
              onclick={onRecompile}
            >
              <RefreshCw size={14} aria-hidden="true" />
              <span>Compile Now</span>
            </button>
          </div>
        {/if}
      {/if}

      <div class="pdf-pages" bind:this={pagesEl} class:hidden-pages={!pdfBase64 || !!renderError}></div>
    </div>

    {#if showLogs}
      <aside class="logs-drawer" aria-label="Build Diagnostics and Compiler Log">
        <div class="drawer-header">
          <div
            class="drawer-tabs"
            role="tablist"
            aria-label="Compiler output views"
          >
            <button
              bind:this={issuesTabEl}
              type="button"
              id="pdf-issues-tab"
              role="tab"
              aria-selected={activeDrawerTab === "issues"}
              tabindex={activeDrawerTab === "issues" ? 0 : -1}
              onclick={() => (activeDrawerTab = "issues")}
              onkeydown={handleTabKeydown}
            >
              <span>Issues</span>
              <span class="tab-count">{diagnostics.length}</span>
            </button>

            <button
              bind:this={logTabEl}
              type="button"
              id="pdf-rawlog-tab"
              role="tab"
              aria-selected={activeDrawerTab === "log"}
              tabindex={activeDrawerTab === "log" ? 0 : -1}
              onclick={() => (activeDrawerTab = "log")}
              onkeydown={handleTabKeydown}
            >
              <Terminal size={13} aria-hidden="true" />
              <span>Raw TeX Log</span>
            </button>
          </div>

          <button
            type="button"
            class="tool-btn"
            title="Close Drawer"
            aria-label="Close diagnostics drawer"
            onclick={onToggleLogs}
          >
            <PanelBottomClose size={15} aria-hidden="true" />
          </button>
        </div>

        <div
          role="tabpanel"
          aria-labelledby="pdf-issues-tab"
          hidden={activeDrawerTab !== "issues"}
          class="drawer-tabpanel"
        >
          <div class="drawer-panel-inner">
            {#if diagnostics.length === 0}
              <div class="drawer-empty">
                {#if compileSuccess === true}
                  <CircleCheck size={16} class="icon-success" aria-hidden="true" />
                  <span>No errors or warnings reported.</span>
                {:else}
                  <Info size={16} aria-hidden="true" />
                  <span>No parsed issues yet.</span>
                {/if}
              </div>
            {:else}
              <ul class="issues-list">
                {#each diagnostics as diag, index (index)}
                  <li>
                    {#if diag.line != null && diag.line >= 1}
                      <button
                        type="button"
                        class="issue-card clickable severity-{diag.severity}"
                        onclick={() => handleDiagnosticClick(diag)}
                      >
                        <span class="issue-icon">
                          {#if diag.severity === "error"}
                            <CircleAlert size={15} aria-hidden="true" />
                          {:else if diag.severity === "warning"}
                            <TriangleAlert size={15} aria-hidden="true" />
                          {:else}
                            <Info size={15} aria-hidden="true" />
                          {/if}
                        </span>
                        <span class="issue-content">
                          <span class="issue-location">
                            {diag.file ? `${diag.file}:${diag.line}` : `Line ${diag.line}`}
                          </span>
                          <span class="issue-message">{diag.message}</span>
                        </span>
                      </button>
                    {:else}
                      <div class="issue-card severity-{diag.severity}">
                        <span class="issue-icon">
                          {#if diag.severity === "error"}
                            <CircleAlert size={15} aria-hidden="true" />
                          {:else if diag.severity === "warning"}
                            <TriangleAlert size={15} aria-hidden="true" />
                          {:else}
                            <Info size={15} aria-hidden="true" />
                          {/if}
                        </span>
                        <span class="issue-content">
                          <span class="issue-location">{diag.file ?? "General"}</span>
                          <span class="issue-message">{diag.message}</span>
                        </span>
                      </div>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>

        <div
          role="tabpanel"
          aria-labelledby="pdf-rawlog-tab"
          hidden={activeDrawerTab !== "log"}
          class="drawer-tabpanel"
        >
          <div class="drawer-panel-inner">
            <pre class="raw-log">{logOutput || "No compiler log output yet."}</pre>
          </div>
        </div>
      </aside>
    {/if}
  </div>
</section>

<style>
  .pdf-preview {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-island);
    box-shadow: var(--shadow-island);
    overflow: hidden;
    color: var(--text-primary);
  }

  .pdf-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 38px;
    padding: 4px 10px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
    overflow-x: auto;
  }

  .toolbar-group {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .tool-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    height: 26px;
    min-width: 26px;
    padding: 0 6px;
    border: 1px solid transparent;
    border-radius: 5px;
    background: transparent;
    color: var(--text-secondary, #404040);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    transition: background-color 0.12s ease, color 0.12s ease, border-color 0.12s ease;
  }

  .tool-btn:hover:not(:disabled) {
    background: var(--bg-hover, #e2e2e2);
    color: var(--text-primary, #111111);
  }

  .tool-btn:focus-visible {
    outline: 2px solid var(--accent, #b45309);
    outline-offset: -2px;
  }

  .tool-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .tool-btn.active {
    background: color-mix(in oklab, var(--accent, #b45309) 14%, transparent);
    color: var(--accent, #b45309);
    border-color: color-mix(in oklab, var(--accent, #b45309) 30%, transparent);
  }

  .zoom-readout,
  .page-readout {
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary, #404040);
    padding: 0 4px;
    user-select: none;
    white-space: nowrap;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9999px;
    font-size: 11px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .badge-error {
    background: color-mix(in oklab, var(--danger, #c62828) 12%, transparent);
    color: color-mix(in oklab, var(--danger, #c62828) 86%, var(--text-primary, #111111));
  }

  .badge-warning {
    background: color-mix(in oklab, var(--warning, #a15c07) 13%, transparent);
    color: color-mix(in oklab, var(--warning, #a15c07) 84%, var(--text-primary, #111111));
  }

  .preview-stage {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1 1 auto;
    min-height: 0;
  }

  .compile-progress {
    position: absolute;
    inset: 0 0 auto 0;
    z-index: 10;
    pointer-events: none;
  }

  .progress-bar {
    height: 3px;
    width: 100%;
    background: linear-gradient(
      90deg,
      transparent 0%,
      var(--accent, #b45309) 50%,
      transparent 100%
    );
    background-size: 200% 100%;
    animation: progress-slide 1.2s ease-in-out infinite;
  }

  .progress-pill {
    position: absolute;
    top: 10px;
    right: 16px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 9999px;
    background: var(--bg-elevated, #fafafa);
    color: var(--text-primary, #111111);
    border: 1px solid var(--border-subtle, #dfdfdf);
    font-size: 12px;
    font-weight: 500;
    box-shadow: var(--shadow-popover, 0 4px 12px rgba(0, 0, 0, 0.14));
  }

  .pdf-viewport {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    padding: 20px 24px;
    background: var(--bg-elevated);
    scrollbar-gutter: stable;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track);
  }

  .pdf-pages {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    min-width: max-content;
  }

  .pdf-pages.hidden-pages {
    display: none;
  }

  .pdf-pages :global(.pdf-page-card) {
    background: #ffffff;
    border-radius: 4px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.18);
    overflow: hidden;
    flex-shrink: 0;
  }

  .pdf-pages :global(.pdf-page-card canvas) {
    display: block;
  }

  .state-card {
    max-width: 360px;
    margin: 48px auto;
    padding: 24px;
    border-radius: 10px;
    background: var(--bg-surface, #ffffff);
    border: 1px solid var(--border-subtle, #dfdfdf);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    color: var(--text-secondary, #404040);
  }

  .state-card h3 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary, #111111);
  }

  .state-card p {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }

  .error-card {
    border-color: color-mix(in oklab, var(--danger, #c62828) 35%, transparent);
    color: var(--danger, #c62828);
  }

  .error-card p {
    color: var(--text-secondary, #404040);
  }

  .state-actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: var(--accent, #b45309);
    color: var(--accent-fg, #ffffff);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
  }

  .action-btn.secondary {
    background: var(--bg-muted, #ececec);
    color: var(--text-primary, #111111);
  }

  .action-btn:focus-visible {
    outline: 2px solid var(--accent, #b45309);
    outline-offset: 2px;
  }

  .logs-drawer {
    display: flex;
    flex-direction: column;
    height: 240px;
    min-height: 140px;
    flex-shrink: 0;
    background: var(--bg-surface, #ffffff);
    border-top: 1px solid var(--border-subtle, #dfdfdf);
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 10px;
    height: 36px;
    background: var(--bg-elevated, #fafafa);
    border-bottom: 1px solid var(--border-subtle, #dfdfdf);
    flex-shrink: 0;
  }

  .drawer-tabs {
    display: flex;
    align-items: stretch;
    gap: 4px;
    height: 100%;
    overflow-x: auto;
  }

  [role="tab"] {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    border: none;
    background: transparent;
    color: var(--text-secondary, #404040);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    flex: none;
    white-space: nowrap;
  }

  [role="tab"]:hover {
    color: var(--text-primary, #111111);
  }

  [role="tab"]:focus-visible {
    outline: 2px solid var(--accent, #b45309);
    outline-offset: -2px;
  }

  [role="tab"][aria-selected="true"] {
    color: var(--accent, #b45309);
  }

  [role="tab"][aria-selected="true"]::after {
    content: "";
    position: absolute;
    inset: auto 0 0;
    border-block-end: 2px solid var(--accent, #b45309);
  }

  .tab-count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9999px;
    font-size: 11px;
    background: var(--bg-muted, #ececec);
    color: var(--text-primary, #111111);
    font-variant-numeric: tabular-nums;
  }

  .drawer-tabpanel {
    flex: 1 1 auto;
    min-height: 0;
  }

  .drawer-tabpanel[hidden] {
    display: none;
  }

  .drawer-panel-inner {
    height: 100%;
    overflow: auto;
    padding: 10px 12px;
    box-sizing: border-box;
    scrollbar-gutter: stable;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb, #737373) var(--scrollbar-track, transparent);
  }

  .drawer-empty {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    color: var(--text-muted, #6e6e6e);
    font-size: 13px;
  }

  .drawer-empty :global(.icon-success) {
    color: var(--success, #15803d);
  }

  .issues-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .issue-card {
    width: 100%;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 6px;
    border: 1px solid var(--border-subtle, #dfdfdf);
    background: var(--bg-elevated, #fafafa);
    color: var(--text-primary, #111111);
    text-align: left;
    font: inherit;
    font-size: 12.5px;
  }

  .issue-card.clickable {
    cursor: pointer;
    transition: border-color 0.12s ease, background-color 0.12s ease;
  }

  .issue-card.clickable:hover {
    border-color: var(--accent, #b45309);
    background: color-mix(in oklab, var(--accent, #b45309) 6%, var(--bg-elevated, #fafafa));
  }

  .issue-card.clickable:focus-visible {
    outline: 2px solid var(--accent, #b45309);
    outline-offset: -2px;
  }

  .issue-icon {
    flex-shrink: 0;
    margin-top: 1px;
  }

  .severity-error .issue-icon {
    color: var(--danger, #c62828);
  }

  .severity-warning .issue-icon {
    color: var(--warning, #a15c07);
  }

  .severity-info .issue-icon {
    color: var(--accent, #b45309);
  }

  .issue-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .issue-location {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-secondary, #404040);
  }

  .issue-message {
    line-height: 1.4;
    word-break: break-word;
  }

  .raw-log {
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text-primary, #111111);
  }

  :global(.spin) {
    animation: spin 1s linear infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .progress-bar,
    :global(.spin) {
      animation: none;
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes progress-slide {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  @supports not (scrollbar-color: auto) {
    .pdf-viewport,
    .drawer-panel-inner {
      scrollbar-width: auto;
    }

    .pdf-viewport::-webkit-scrollbar,
    .drawer-panel-inner::-webkit-scrollbar {
      width: 10px;
      height: 10px;
    }

    .pdf-viewport::-webkit-scrollbar-thumb,
    .drawer-panel-inner::-webkit-scrollbar-thumb {
      background: var(--scrollbar-thumb, #737373);
      border-radius: 5px;
    }

    .pdf-viewport::-webkit-scrollbar-track,
    .drawer-panel-inner::-webkit-scrollbar-track {
      background: var(--scrollbar-track, transparent);
    }
  }
</style>
