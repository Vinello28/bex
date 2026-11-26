<script lang="ts">
  import {
    Check,
    ChevronDown,
    ChevronRight,
    Copy,
    CornerDownLeft,
    FileCode,
    LoaderCircle,
    Paperclip,
    Plug,
    RotateCcw,
    Send,
    Settings,
    Sparkles,
    Trash2,
    TriangleAlert,
    X,
  } from "@lucide/svelte";
  import type {
    AiAttachment,
    AiChatMessage,
    AiEnvStatus,
    AiSettings,
    McpToolInfo,
  } from "../types";

  interface Props {
    messages: AiChatMessage[];
    isSending: boolean;
    aiSettings: AiSettings;
    aiEnvStatus: AiEnvStatus;
    mcpTools: McpToolInfo[];
    workspaceName: string | null;
    activeFilePath: string | null;
    attachments: AiAttachment[];
    onSendMessage: (prompt: string) => void;
    onClearChat: () => void;
    onPickAttachment: () => void;
    onAttachActiveFile: () => void;
    onRemoveAttachment: (path: string) => void;
    onInsertCodeIntoEditor: (code: string) => void;
    onOpenAiSettings: () => void;
    onClose: () => void;
  }

  let {
    messages,
    isSending,
    aiSettings,
    aiEnvStatus,
    mcpTools,
    workspaceName,
    activeFilePath,
    attachments,
    onSendMessage,
    onClearChat,
    onPickAttachment,
    onAttachActiveFile,
    onRemoveAttachment,
    onInsertCodeIntoEditor,
    onOpenAiSettings,
    onClose,
  }: Props = $props();

  let inputText = $state("");
  let showMcpTools = $state(false);
  let chatThreadEl: HTMLDivElement | undefined = $state();
  let textareaEl: HTMLTextAreaElement | undefined = $state();
  let copiedCode = $state<string | null>(null);
  let copyTimer: ReturnType<typeof setTimeout> | undefined;

  const starterPrompts = [
    "Summarize the attached file into a LaTeX section and write it to the active file",
    "Inspect and fix any LaTeX compilation errors",
    "Draft a concise abstract for this paper",
    "Add a clean LaTeX table summarizing our methodology",
  ];

  const providerLabels: Record<string, string> = {
    google: "Google AI Studio",
    ollama: "Ollama (Local)",
    lmstudio: "LM Studio (Local)",
    vllm: "vLLM",
  };

  const providerLabel = $derived(
    providerLabels[aiSettings.provider] ?? aiSettings.provider
  );

  const showKeyWarning = $derived(
    aiSettings.provider === "google" &&
      !aiSettings.apiKey.trim() &&
      !aiEnvStatus.hasGoogleEnvKey
  );

  const isActiveFileAttached = $derived(
    activeFilePath !== null &&
      attachments.some((att) => att.path === activeFilePath)
  );

  const canSend = $derived(
    !isSending && (inputText.trim().length > 0 || attachments.length > 0)
  );

  // Auto-scroll chat thread to bottom on message or sending change
  $effect(() => {
    const _len = messages.length;
    const _sending = isSending;
    if (chatThreadEl) {
      requestAnimationFrame(() => {
        if (chatThreadEl) {
          chatThreadEl.scrollTo({
            top: chatThreadEl.scrollHeight,
            behavior: "smooth",
          });
        }
      });
    }
  });

  function formatFileName(path: string | null): string {
    if (!path) return "";
    const parts = path.split(/[/\\]/);
    return parts[parts.length - 1] || path;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  function handleSend() {
    if (!canSend) return;
    const prompt = inputText.trim();
    onSendMessage(prompt);
    inputText = "";
  }

  function selectStarterPrompt(prompt: string) {
    inputText = prompt;
    textareaEl?.focus();
  }

  async function copyCode(code: string) {
    try {
      await navigator.clipboard.writeText(code);
      copiedCode = code;
      clearTimeout(copyTimer);
      copyTimer = setTimeout(() => {
        if (copiedCode === code) copiedCode = null;
      }, 2000);
    } catch (err) {
      console.error("Failed to copy code to clipboard", err);
    }
  }

  interface CodeBlock {
    type: "code";
    lang: string;
    code: string;
  }

  interface TextBlock {
    type: "text";
    paragraphs: string[];
  }

  type ParsedBlock = CodeBlock | TextBlock;

  function parseMessageContent(raw: string): ParsedBlock[] {
    const blocks: ParsedBlock[] = [];
    const fenceRegex = /```([a-zA-Z0-9_-]*)\r?\n([\s\S]*?)```/g;
    let lastIndex = 0;
    let match: RegExpExecArray | null;

    while ((match = fenceRegex.exec(raw)) !== null) {
      if (match.index > lastIndex) {
        const textChunk = raw.slice(lastIndex, match.index);
        const paras = textChunk
          .split(/\r?\n\r?\n/)
          .map((p) => p.trim())
          .filter(Boolean);
        if (paras.length > 0) {
          blocks.push({ type: "text", paragraphs: paras });
        }
      }
      blocks.push({
        type: "code",
        lang: match[1]?.trim() || "latex",
        code: match[2] ?? "",
      });
      lastIndex = match.index + match[0].length;
    }

    if (lastIndex < raw.length) {
      const textChunk = raw.slice(lastIndex);
      const paras = textChunk
        .split(/\r?\n\r?\n/)
        .map((p) => p.trim())
        .filter(Boolean);
      if (paras.length > 0) {
        blocks.push({ type: "text", paragraphs: paras });
      }
    }

    if (blocks.length === 0 && raw.trim().length > 0) {
      blocks.push({ type: "text", paragraphs: [raw.trim()] });
    }

    return blocks;
  }

  interface InlineSpan {
    isCode: boolean;
    text: string;
  }

  function parseInlineSpans(text: string): InlineSpan[] {
    const spans: InlineSpan[] = [];
    const inlineRegex = /`([^`]+)`/g;
    let last = 0;
    let m: RegExpExecArray | null;
    while ((m = inlineRegex.exec(text)) !== null) {
      if (m.index > last) {
        spans.push({ isCode: false, text: text.slice(last, m.index) });
      }
      spans.push({ isCode: true, text: m[1] });
      last = m.index + m[0].length;
    }
    if (last < text.length) {
      spans.push({ isCode: false, text: text.slice(last) });
    }
    return spans;
  }
</script>

<aside class="ai-sidebar">
  <!-- Top Island Header -->
  <header class="sidebar-header">
    <div class="header-brand">
      <Sparkles size={16} class="brand-sparkle-icon" aria-hidden="true" />
      <span class="brand-title">Bex AI</span>
    </div>

    <div class="header-actions">
      <!-- Clickable MCP status pill -->
      <button
        type="button"
        class="mcp-status-pill"
        class:mcp-active={showMcpTools}
        onclick={() => (showMcpTools = !showMcpTools)}
        title="Toggle MCP tools inspector"
        aria-expanded={showMcpTools}
      >
        <Plug size={12} class="mcp-plug-icon" aria-hidden="true" />
        <span class="mcp-pill-text">MCP · {mcpTools.length}</span>
      </button>

      <!-- Clear Chat -->
      <button
        type="button"
        class="icon-action-btn"
        onclick={onClearChat}
        title="Clear conversation"
        aria-label="Clear chat"
        disabled={messages.length === 0 && !isSending}
      >
        <Trash2 size={14} aria-hidden="true" />
      </button>

      <!-- AI Settings -->
      <button
        type="button"
        class="icon-action-btn"
        onclick={onOpenAiSettings}
        title="Open AI & MCP Settings"
        aria-label="AI Settings"
      >
        <Settings size={14} aria-hidden="true" />
      </button>

      <!-- Close Sidebar -->
      <button
        type="button"
        class="icon-action-btn close-action-btn"
        onclick={onClose}
        title="Close AI Sidebar"
        aria-label="Close AI Sidebar"
      >
        <X size={15} aria-hidden="true" />
      </button>
    </div>
  </header>

  <!-- Collapsible MCP Inspector Panel -->
  {#if showMcpTools}
    <div class="mcp-inspector-panel">
      <div class="mcp-inspector-top">
        <span class="mcp-inspector-title">
          <Plug size={13} aria-hidden="true" />
          Connected MCP Tools ({mcpTools.length})
        </span>
        <button
          type="button"
          class="icon-btn-compact"
          aria-label="Close tools panel"
          onclick={() => (showMcpTools = false)}
        >
          <X size={12} aria-hidden="true" />
        </button>
      </div>

      {#if mcpTools.length === 0}
        <p class="mcp-empty-hint">
          No tools active. Bex AI MCP server is ready for TeX edits and compilation.
        </p>
      {:else}
        <ul class="mcp-tools-list scrollable-pane">
          {#each mcpTools as tool (tool.name)}
            <li class="mcp-tool-item">
              <span class="mcp-tool-name">{tool.name}</span>
              <p class="mcp-tool-desc">{tool.description}</p>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  <!-- Active Provider Status Strip -->
  <div class="provider-status-strip">
    <div class="provider-model-info">
      <span class="provider-indicator-dot" aria-hidden="true"></span>
      <span class="provider-name">{providerLabel}</span>
      <span class="provider-model-sep" aria-hidden="true">/</span>
      <span class="provider-model-chip">{aiSettings.model || "default"}</span>
    </div>

    {#if showKeyWarning}
      <div class="provider-key-banner" role="alert">
        <TriangleAlert size={13} class="warning-alert-icon" aria-hidden="true" />
        <span class="warning-text">API key needed</span>
        <button
          type="button"
          class="key-config-btn"
          onclick={onOpenAiSettings}
        >
          Configure Key
        </button>
      </div>
    {/if}
  </div>

  <!-- Chat Thread -->
  <div
    bind:this={chatThreadEl}
    class="chat-thread scrollable-pane"
    role="log"
    aria-label="AI chat history"
  >
    {#if messages.length === 0}
      <!-- Empty State -->
      <div class="chat-empty-state">
        <div class="empty-sparkle-halo" aria-hidden="true">
          <Sparkles size={24} />
        </div>
        <h3 class="empty-title">Bex AI TeX Assistant</h3>
        <p class="empty-desc">
          Context-aware AI with embedded MCP tools. Read, edit, create, summarize
          files into LaTeX, and compile automatically.
        </p>

        <div class="starter-chips-container">
          <span class="starter-chips-label">Try asking:</span>
          <div class="starter-chips-list">
            {#each starterPrompts as prompt (prompt)}
              <button
                type="button"
                class="starter-chip"
                onclick={() => selectStarterPrompt(prompt)}
              >
                <span>{prompt}</span>
              </button>
            {/each}
          </div>
        </div>
      </div>
    {:else}
      <!-- Messages List -->
      <div class="messages-flow">
        {#each messages as msg (msg.id)}
          <div
            class="chat-message"
            class:is-user={msg.role === "user"}
            class:is-assistant={msg.role === "assistant"}
            class:has-error={msg.error}
          >
            <!-- User Attachments if present -->
            {#if msg.attachments && msg.attachments.length > 0}
              <div class="msg-attachments-preview">
                {#each msg.attachments as att (att.path)}
                  <span class="msg-attachment-pill" title={att.path}>
                    <FileCode size={11} aria-hidden="true" />
                    <span>{att.name}</span>
                  </span>
                {/each}
              </div>
            {/if}

            <!-- MCP Tool Traces for Assistant Messages -->
            {#if msg.toolTraces && msg.toolTraces.length > 0}
              <details class="mcp-traces-details">
                <summary class="mcp-traces-summary">
                  <Plug size={12} aria-hidden="true" />
                  <span>MCP · {msg.toolTraces.length} tool call{msg.toolTraces.length === 1 ? "" : "s"}</span>
                </summary>
                <div class="mcp-traces-content">
                  {#each msg.toolTraces as trace (trace.id)}
                    <div class="trace-item" class:trace-error={trace.isError}>
                      <div class="trace-header">
                        <span class="trace-tool-name">{trace.toolName}</span>
                        <span
                          class="trace-badge"
                          class:trace-ok={!trace.isError}
                          class:trace-fail={trace.isError}
                        >
                          {trace.isError ? "Error" : "OK"}
                        </span>
                      </div>
                      {#if trace.argumentsJson}
                        <pre class="trace-args"><code>{trace.argumentsJson}</code></pre>
                      {/if}
                      {#if trace.resultSummary}
                        <p class="trace-summary">{trace.resultSummary}</p>
                      {/if}
                    </div>
                  {/each}
                </div>
              </details>
            {/if}

            <!-- Message Body Rendered Safely Without {@html} -->
            <div class="msg-bubble">
              {#each parseMessageContent(msg.content) as block}
                {#if block.type === "code"}
                  <div class="fenced-code-card">
                    <div class="fenced-code-header">
                      <span class="fenced-code-lang">{block.lang}</span>
                      <div class="fenced-code-actions">
                        <button
                          type="button"
                          class="code-btn"
                          onclick={() => copyCode(block.code)}
                          title="Copy snippet"
                        >
                          {#if copiedCode === block.code}
                            <Check size={12} class="icon-copied" aria-hidden="true" />
                            <span>Copied</span>
                          {:else}
                            <Copy size={12} aria-hidden="true" />
                            <span>Copy</span>
                          {/if}
                        </button>
                        <button
                          type="button"
                          class="code-btn code-insert-btn"
                          onclick={() => onInsertCodeIntoEditor(block.code)}
                          title="Insert code into editor"
                        >
                          <span>Insert</span>
                        </button>
                      </div>
                    </div>
                    <pre class="fenced-code-body"><code>{block.code}</code></pre>
                  </div>
                {:else}
                  {#each block.paragraphs as para}
                    <p class="msg-para">
                      {#each parseInlineSpans(para) as span}
                        {#if span.isCode}
                          <code class="inline-code">{span.text}</code>
                        {:else}
                          {span.text}
                        {/if}
                      {/each}
                    </p>
                  {/each}
                {/if}
              {/each}
            </div>
          </div>
        {/each}

        <!-- Active Working Spinner -->
        {#if isSending}
          <div class="chat-message is-assistant">
            <div class="msg-bubble sending-bubble">
              <LoaderCircle size={15} class="spin-icon" aria-hidden="true" />
              <span>Bex AI is working via MCP...</span>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Composer Footer -->
  <footer class="composer-footer">
    <!-- Active File Context Strip & Attached Files -->
    <div class="composer-context-strip">
      {#if activeFilePath}
        <div class="active-file-chip" title={activeFilePath}>
          <FileCode size={12} class="file-chip-icon" aria-hidden="true" />
          <span class="file-chip-name">{formatFileName(activeFilePath)}</span>
          {#if !isActiveFileAttached}
            <button
              type="button"
              class="context-attach-btn"
              onclick={onAttachActiveFile}
              title="Attach current active file to AI context"
            >
              + Context
            </button>
          {:else}
            <span class="attached-badge">Attached</span>
          {/if}
        </div>
      {/if}

      {#if attachments.length > 0}
        <div class="attachments-list">
          {#each attachments as att (att.path)}
            <span class="attachment-pill" title={att.path}>
              <span class="attachment-name">{att.name}</span>
              <button
                type="button"
                class="attachment-remove-btn"
                onclick={() => onRemoveAttachment(att.path)}
                title="Remove attachment"
                aria-label="Remove {att.name}"
              >
                <X size={11} aria-hidden="true" />
              </button>
            </span>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Textarea Input -->
    <div class="composer-input-wrapper">
      <textarea
        bind:this={textareaEl}
        bind:value={inputText}
        class="composer-textarea scrollable-pane"
        placeholder="Ask Bex AI to edit, write, or fix LaTeX..."
        rows="2"
        disabled={isSending}
        onkeydown={handleKeyDown}
      ></textarea>
    </div>

    <!-- Footer Controls Bar -->
    <div class="composer-actions-bar">
      <div class="composer-left-actions">
        <button
          type="button"
          class="tool-btn"
          onclick={onPickAttachment}
          title="Attach file from disk"
          disabled={isSending}
        >
          <Paperclip size={13} aria-hidden="true" />
          <span>Attach File</span>
        </button>

        {#if activeFilePath && !isActiveFileAttached}
          <button
            type="button"
            class="tool-btn"
            onclick={onAttachActiveFile}
            title="Attach currently open editor file"
            disabled={isSending}
          >
            <FileCode size={13} aria-hidden="true" />
            <span>Attach Current</span>
          </button>
        {/if}
      </div>

      <button
        type="button"
        class="send-btn"
        disabled={!canSend}
        onclick={handleSend}
        title="Send prompt (Enter)"
        aria-label="Send message"
      >
        <span>Send</span>
        <Send size={13} aria-hidden="true" />
      </button>
    </div>
  </footer>
</aside>

<style>
  .ai-sidebar {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-island, 10px);
    box-shadow: var(--shadow-island, var(--shadow-dialog));
    display: flex;
    flex-direction: column;
    overflow: hidden;
    inline-size: 380px;
    min-inline-size: 320px;
    max-inline-size: 100%;
    block-size: 100%;
    min-block-size: 0;
  }

  .sidebar-header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }

  .header-brand {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }

  :global(.brand-sparkle-icon) {
    color: var(--accent);
  }

  .brand-title {
    font-size: 13.5px;
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--text-primary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .mcp-status-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-secondary);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 120ms ease, border-color 120ms ease, color 120ms ease;
  }

  .mcp-status-pill:hover,
  .mcp-status-pill.mcp-active {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--text-primary);
  }

  :global(.mcp-plug-icon) {
    color: var(--accent);
  }

  .icon-action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 26px;
    block-size: 26px;
    border-radius: var(--radius-xs);
    border: 1px solid transparent;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .icon-action-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .icon-action-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .close-action-btn:hover:not(:disabled) {
    color: var(--danger);
  }

  /* MCP inspector dropdown */
  .mcp-inspector-panel {
    flex-shrink: 0;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .mcp-inspector-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .mcp-inspector-title {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .icon-btn-compact {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 20px;
    block-size: 20px;
    border-radius: var(--radius-xs);
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .icon-btn-compact:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .mcp-empty-hint {
    margin: 0;
    font-size: 11.5px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .mcp-tools-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-block-size: 140px;
  }

  .mcp-tool-item {
    padding: 6px 8px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }

  .mcp-tool-name {
    display: block;
    font-family: var(--font-mono);
    font-size: 11.5px;
    font-weight: 600;
    color: var(--accent);
  }

  .mcp-tool-desc {
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.35;
  }

  /* Provider status strip */
  .provider-status-strip {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 7px 14px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-muted);
  }

  .provider-model-info {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .provider-indicator-dot {
    inline-size: 6px;
    block-size: 6px;
    border-radius: 50%;
    background: var(--success);
    flex-shrink: 0;
  }

  .provider-name {
    font-weight: 600;
  }

  .provider-model-sep {
    color: var(--text-muted);
  }

  .provider-model-chip {
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .provider-key-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 5px 8px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-subtle);
    background: var(--warning-soft);
  }

  :global(.warning-alert-icon) {
    color: var(--warning);
    flex-shrink: 0;
  }

  .warning-text {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-primary);
    flex: 1;
  }

  .key-config-btn {
    padding: 2px 7px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--accent);
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 10.5px;
    font-weight: 600;
    cursor: pointer;
    flex-shrink: 0;
  }

  /* Chat Thread */
  .chat-thread {
    flex: 1;
    min-block-size: 0;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .chat-empty-state {
    margin: auto 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 10px;
    padding: 16px 8px;
  }

  .empty-sparkle-halo {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 44px;
    block-size: 44px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
  }

  .empty-title {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .empty-desc {
    margin: 0;
    max-inline-size: 300px;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .starter-chips-container {
    inline-size: 100%;
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 6px;
  }

  .starter-chips-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted);
    text-align: left;
  }

  .starter-chips-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .starter-chip {
    padding: 7px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
    color: var(--text-secondary);
    font-size: 11.5px;
    text-align: left;
    cursor: pointer;
    transition: background-color 120ms ease, border-color 120ms ease, color 120ms ease;
    line-height: 1.35;
  }

  .starter-chip:hover {
    background: var(--bg-hover);
    border-color: var(--border-strong);
    color: var(--text-primary);
  }

  .messages-flow {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .chat-message {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-inline-size: 100%;
  }

  .chat-message.is-user {
    align-items: flex-end;
  }

  .chat-message.is-assistant {
    align-items: flex-start;
  }

  .msg-attachments-preview {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .msg-attachment-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: var(--radius-xs);
    background: var(--bg-muted);
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    font-size: 10.5px;
    font-family: var(--font-mono);
  }

  .msg-bubble {
    padding: 10px 12px;
    border-radius: var(--radius-md);
    font-size: 12.5px;
    line-height: 1.5;
    word-break: break-word;
    max-inline-size: 94%;
  }

  .chat-message.is-user .msg-bubble {
    background: var(--accent);
    color: var(--accent-fg);
    border-bottom-right-radius: var(--radius-xs);
  }

  .chat-message.is-assistant .msg-bubble {
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    color: var(--text-primary);
    border-bottom-left-radius: var(--radius-xs);
  }

  .chat-message.has-error .msg-bubble {
    border-color: var(--danger);
    background: var(--danger-soft);
    color: var(--danger);
  }

  .sending-bubble {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 12px;
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

  .msg-para {
    margin: 0 0 8px;
  }

  .msg-para:last-child {
    margin-bottom: 0;
  }

  .inline-code {
    font-family: var(--font-mono);
    font-size: 11.5px;
    padding: 1px 4px;
    border-radius: var(--radius-xs);
    background: var(--bg-muted);
    color: var(--text-primary);
    border: 1px solid var(--border-subtle);
  }

  .chat-message.is-user .inline-code {
    background: rgba(0, 0, 0, 0.15);
    border-color: rgba(255, 255, 255, 0.2);
    color: var(--accent-fg);
  }

  /* Fenced code block in assistant messages */
  .fenced-code-card {
    margin: 8px 0;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    overflow: hidden;
  }

  .fenced-code-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    background: var(--bg-muted);
    border-bottom: 1px solid var(--border-subtle);
  }

  .fenced-code-lang {
    font-family: var(--font-mono);
    font-size: 10.5px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
  }

  .fenced-code-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .code-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 10.5px;
    cursor: pointer;
    transition: background-color 100ms ease, color 100ms ease;
  }

  .code-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .code-insert-btn {
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 500;
  }

  .code-insert-btn:hover {
    background: var(--accent-soft);
  }

  :global(.icon-copied) {
    color: var(--success);
  }

  .fenced-code-body {
    margin: 0;
    padding: 8px 10px;
    font-family: var(--font-mono);
    font-size: 11.5px;
    line-height: 1.45;
    overflow-x: auto;
    color: var(--text-primary);
    background: var(--bg-surface);
  }

  /* MCP Traces collapsible */
  .mcp-traces-details {
    inline-size: 100%;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    font-size: 11.5px;
    overflow: hidden;
  }

  .mcp-traces-summary {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 9px;
    cursor: pointer;
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--bg-muted);
    user-select: none;
  }

  .mcp-traces-summary:hover {
    color: var(--text-primary);
  }

  .mcp-traces-content {
    padding: 6px 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .trace-item {
    padding: 6px 8px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
  }

  .trace-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .trace-tool-name {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
  }

  .trace-badge {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 999px;
    text-transform: uppercase;
  }

  .trace-badge.trace-ok {
    background: var(--success-soft);
    color: var(--success);
  }

  .trace-badge.trace-fail {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .trace-args {
    margin: 4px 0 0;
    padding: 4px 6px;
    border-radius: var(--radius-xs);
    background: var(--bg-muted);
    font-family: var(--font-mono);
    font-size: 10.5px;
    overflow-x: auto;
    color: var(--text-secondary);
  }

  .trace-summary {
    margin: 4px 0 0;
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.35;
  }

  /* Composer Footer */
  .composer-footer {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-surface);
  }

  .composer-context-strip {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }

  .active-file-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 7px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    font-size: 11px;
    color: var(--text-secondary);
  }

  :global(.file-chip-icon) {
    color: var(--accent);
  }

  .file-chip-name {
    font-family: var(--font-mono);
    max-inline-size: 130px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .context-attach-btn {
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--accent);
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 10px;
    font-weight: 600;
    cursor: pointer;
  }

  .context-attach-btn:hover {
    background: var(--accent);
    color: var(--accent-fg);
  }

  .attached-badge {
    font-size: 10px;
    color: var(--success);
    font-weight: 500;
  }

  .attachments-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .attachment-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-primary);
  }

  .attachment-name {
    max-inline-size: 110px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .attachment-remove-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
  }

  .attachment-remove-btn:hover {
    color: var(--danger);
  }

  .composer-input-wrapper {
    display: flex;
  }

  .composer-textarea {
    inline-size: 100%;
    min-block-size: 50px;
    max-block-size: 140px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-primary);
    font-size: 12.5px;
    line-height: 1.45;
    resize: none;
    outline: none;
    transition: border-color 120ms ease;
  }

  .composer-textarea:focus {
    border-color: var(--accent);
  }

  .composer-textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .composer-actions-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .composer-left-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .tool-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-subtle);
    background: var(--bg-muted);
    color: var(--text-secondary);
    font-size: 11.5px;
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .tool-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .tool-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .send-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 12px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    background: var(--accent);
    color: var(--accent-fg);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 120ms ease, opacity 120ms ease;
  }

  .send-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .send-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
