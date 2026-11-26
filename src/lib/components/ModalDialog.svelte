<script lang="ts">
  import { untrack } from "svelte";
  import {
    FilePlus,
    FolderPlus,
    Pencil,
    Trash,
    CircleExclamationPoint,
  } from "@lucide/svelte";

  export type ModalMode =
    | "create-file"
    | "create-folder"
    | "rename"
    | "delete"
    | null;

  interface Props {
    open: boolean;
    mode: ModalMode;
    targetName?: string;
    parentLabel?: string;
    onConfirm: (value?: string) => void;
    onCancel: () => void;
  }

  let {
    open,
    mode,
    targetName = "",
    parentLabel = "",
    onConfirm,
    onCancel,
  }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  let inputEl: HTMLInputElement | undefined = $state();
  let inputValue = $state("");
  let validationError = $state("");

  const titleText = $derived.by(() => {
    switch (mode) {
      case "create-file":
        return "Create New File";
      case "create-folder":
        return "Create New Folder";
      case "rename":
        return "Rename Item";
      case "delete":
        return "Delete Item";
      default:
        return "Dialog";
    }
  });

  const inputLabel = $derived.by(() => {
    switch (mode) {
      case "create-file":
        return "File name (e.g. chapter1.tex or refs.bib)";
      case "create-folder":
        return "Folder name (e.g. appendices)";
      case "rename":
        return "New name";
      default:
        return "Name";
    }
  });

  const confirmLabel = $derived.by(() => {
    switch (mode) {
      case "create-file":
        return "Create File";
      case "create-folder":
        return "Create Folder";
      case "rename":
        return "Rename";
      case "delete":
        return "Delete";
      default:
        return "Confirm";
    }
  });

  // Sync native showModal() / close() with `open` prop
  $effect(() => {
    if (!dialogEl) return;

    if (open && mode) {
      untrack(() => {
        validationError = "";
        inputValue = mode === "rename" ? targetName : "";
      });

      if (!dialogEl.open) {
        dialogEl.showModal();
      }

      // Highlight filename stem on rename for ergonomic editing
      if (mode === "rename") {
        queueMicrotask(() => {
          if (!inputEl) return;
          const dotIdx = inputEl.value.lastIndexOf(".");
          if (dotIdx > 0) {
            inputEl.setSelectionRange(0, dotIdx);
          } else {
            inputEl.select();
          }
        });
      }
    } else if (!open && dialogEl.open) {
      dialogEl.close();
    }
  });

  // Fallback light-dismiss only for browsers lacking native <dialog closedby>
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
      pressedOutside = isOutside(e);
    };
    const onPointerUp = (e: PointerEvent) => {
      if (pressedOutside && isOutside(e)) {
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

  function validateName(raw: string): string | null {
    const trimmed = raw.trim();
    if (!trimmed) {
      return "Please enter a name.";
    }
    if (trimmed === "." || trimmed === "..") {
      return "Name cannot be '.' or '..'.";
    }
    if (trimmed.includes("\\") || trimmed.split("/").some((part) => part === "..")) {
      return "Invalid path segments are not allowed.";
    }
    if (mode === "rename" && trimmed.includes("/")) {
      return "Rename expects a single file or folder name without slashes.";
    }
    return null;
  }

  function handleFormSubmit(event: SubmitEvent) {
    if (mode === "delete") return;
    const err = validateName(inputValue);
    if (err) {
      event.preventDefault();
      validationError = err;
      inputEl?.focus();
    }
  }

  function handleCancelClick() {
    dialogEl?.close("cancel");
  }

  function handleDialogClose() {
    if (!dialogEl) return;
    const result = dialogEl.returnValue;
    dialogEl.returnValue = "";

    if (result === "confirm") {
      if (mode === "delete") {
        onConfirm();
      } else {
        onConfirm(inputValue.trim());
      }
    } else {
      onCancel();
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  class="bex-modal"
  closedby="any"
  aria-labelledby="modal-dialog-title"
  onclose={handleDialogClose}
>
  <div class="modal-surface">
    <header class="modal-header">
      <span
        class="modal-icon"
        class:danger={mode === "delete"}
        aria-hidden="true"
      >
        {#if mode === "create-file"}
          <FilePlus size={18} />
        {:else if mode === "create-folder"}
          <FolderPlus size={18} />
        {:else if mode === "rename"}
          <Pencil size={18} />
        {:else if mode === "delete"}
          <Trash size={18} />
        {/if}
      </span>
      <div class="modal-titles">
        <h1 id="modal-dialog-title" class="modal-title">{titleText}</h1>
        {#if parentLabel && (mode === "create-file" || mode === "create-folder")}
          <p class="modal-subtitle">Inside <code>{parentLabel}</code></p>
        {/if}
      </div>
    </header>

    <form method="dialog" class="modal-form" onsubmit={handleFormSubmit}>
      {#if mode === "delete"}
        <p class="delete-copy">
          Are you sure you want to permanently delete
          <strong>{targetName}</strong>?
          {#if parentLabel}
            <span class="delete-path">({parentLabel})</span>
          {/if}
        </p>
        <p class="delete-warning">This action cannot be undone.</p>
      {:else}
        <label class="field-label" for="modal-name-input">{inputLabel}</label>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          id="modal-name-input"
          bind:this={inputEl}
          bind:value={inputValue}
          type="text"
          class="field-input"
          class:has-error={Boolean(validationError)}
          placeholder={mode === "create-folder" ? "sections" : "document.tex"}
          autocomplete="off"
          spellcheck="false"
          autofocus
          oninput={() => {
            if (validationError) validationError = "";
          }}
        />
        {#if validationError}
          <p class="field-error" role="alert">
            <CircleExclamationPoint size={14} aria-hidden="true" />
            <span>{validationError}</span>
          </p>
        {/if}
      {/if}

      <footer class="modal-actions">
        {#if mode === "delete"}
          <!-- svelte-ignore a11y_autofocus -->
          <button
            type="button"
            class="btn btn-secondary"
            autofocus
            onclick={handleCancelClick}
          >
            Cancel
          </button>
          <button type="submit" value="confirm" class="btn btn-danger">
            {confirmLabel}
          </button>
        {:else}
          <button
            type="button"
            class="btn btn-secondary"
            onclick={handleCancelClick}
          >
            Cancel
          </button>
          <button type="submit" value="confirm" class="btn btn-primary">
            {confirmLabel}
          </button>
        {/if}
      </footer>
    </form>
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
    inline-size: min(440px, calc(100% - 32px));
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

  .modal-icon.danger {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .modal-titles {
    min-inline-size: 0;
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

  .modal-subtitle code {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-secondary);
  }

  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .field-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .field-input {
    inline-size: 100%;
    padding: 8px 11px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 13px;
    transition: border-color 120ms ease;
  }

  .field-input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .field-input.has-error {
    border-color: var(--danger);
  }

  .field-error {
    margin: 0;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--danger);
  }

  .delete-copy {
    margin: 0;
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.5;
    word-break: break-word;
  }

  .delete-path {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: 11.5px;
    margin-inline-start: 4px;
  }

  .delete-warning {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .modal-actions {
    margin-top: 6px;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn {
    min-block-size: 32px;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-size: 12.5px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid transparent;
    transition: background-color 120ms ease, border-color 120ms ease;
  }

  .btn-secondary {
    background: var(--bg-muted);
    border-color: var(--border-subtle);
    color: var(--text-primary);
  }

  @media (hover: hover) {
    .btn-secondary:hover {
      background: var(--bg-hover);
      border-color: var(--border-strong);
    }
  }

  .btn-primary {
    background: var(--accent);
    color: var(--accent-fg);
  }

  @media (hover: hover) {
    .btn-primary:hover {
      background: var(--accent-hover);
    }
  }

  .btn-danger {
    background: var(--danger);
    color: #ffffff;
  }

  @supports (color: light-dark(white, black)) {
    .btn-danger {
      background: light-dark(var(--danger-light), #d32f2f);
    }
  }

  @media (hover: hover) {
    .btn-danger:hover {
      background: var(--danger-hover);
    }

    @supports (color: light-dark(white, black)) {
      .btn-danger:hover {
        background: light-dark(var(--danger-hover-light), #b71c1c);
      }
    }
  }
</style>
