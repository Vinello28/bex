<script lang="ts">
  import { onMount } from "svelte";
  import {
    Annotation,
    Compartment,
    EditorState,
    Prec,
    StateEffect,
    StateField,
    type Extension,
  } from "@codemirror/state";
  import {
    Decoration,
    type DecorationSet,
    EditorView,
    crosshairCursor,
    drawSelection,
    dropCursor,
    highlightActiveLine,
    highlightActiveLineGutter,
    highlightSpecialChars,
    keymap,
    lineNumbers,
    rectangularSelection,
  } from "@codemirror/view";
  import {
    defaultKeymap,
    history,
    historyKeymap,
    indentWithTab,
  } from "@codemirror/commands";
  import {
    HighlightStyle,
    StreamLanguage,
    bracketMatching,
    foldGutter,
    foldKeymap,
    indentOnInput,
    syntaxHighlighting,
  } from "@codemirror/language";
  import { stex } from "@codemirror/legacy-modes/mode/stex";
  import {
    autocompletion,
    closeBrackets,
    closeBracketsKeymap,
    completionKeymap,
    snippetCompletion,
    type Completion,
    type CompletionContext,
    type CompletionResult,
  } from "@codemirror/autocomplete";
  import { lintGutter, lintKeymap, linter, setDiagnostics, type Diagnostic } from "@codemirror/lint";
  import {
    highlightSelectionMatches,
    openSearchPanel,
    search,
    searchKeymap,
  } from "@codemirror/search";
  import { tags } from "@lezer/highlight";
  import {
    Bold,
    Crosshair,
    Heading,
    Image,
    Italic,
    List,
    Percent,
    Save,
    Search,
    Sigma,
    SquareFunction,
  } from "@lucide/svelte";
  import { isCommentToggleShortcut, toggleLatexComment } from "../latexComment";
  import type { LatexDiagnostic } from "../types";

  interface Props {
    content: string;
    filePath: string | null;
    fileName: string | null;
    isDirty: boolean;
    diagnostics?: LatexDiagnostic[];
    jumpToLine?: { line: number; seq: number } | null;
    theme?: "light" | "dark" | "system";
    synctexAvailable?: boolean;
    onChange: (newContent: string) => void;
    onSaveAndCompile: () => void;
    onSyncToPdf?: (line: number, column: number) => void;
  }

  let {
    content,
    filePath,
    fileName,
    isDirty,
    diagnostics = [],
    jumpToLine = null,
    theme = "system",
    synctexAvailable = false,
    onChange,
    onSaveAndCompile,
    onSyncToPdf,
  }: Props = $props();

  let editorHost: HTMLDivElement | undefined = $state();
  let view: EditorView | null = null;
  let cursorLine = $state(1);
  let cursorCol = $state(1);
  let systemDark = $state(false);

  // Non-reactive trackers to prevent feedback loops between CodeMirror and Svelte props
  let lastSyncedContent = "";
  let lastFilePath: string | null = null;
  let lastJumpSeq = -1;
  let flashClearTimer: ReturnType<typeof setTimeout> | null = null;
  let flashParity = 0;

  const externalSyncAnnotation = Annotation.define<boolean>();
  const themeCompartment = new Compartment();
  const editableCompartment = new Compartment();

  const setFlashLineEffect = StateEffect.define<{ pos: number; parity: number } | null>();
  const flashLineDecoEven = Decoration.line({
    attributes: { class: "cm-synctex-flash-line cm-synctex-flash-a" },
  });
  const flashLineDecoOdd = Decoration.line({
    attributes: { class: "cm-synctex-flash-line cm-synctex-flash-b" },
  });

  const flashLineField = StateField.define<DecorationSet>({
    create() {
      return Decoration.none;
    },
    update(decorations, tr) {
      let next = decorations.map(tr.changes);
      for (const effect of tr.effects) {
        if (effect.is(setFlashLineEffect)) {
          if (effect.value === null) {
            next = Decoration.none;
          } else {
            const safePos = Math.max(0, Math.min(effect.value.pos, tr.state.doc.length));
            const line = tr.state.doc.lineAt(safePos);
            const deco = effect.value.parity % 2 === 0 ? flashLineDecoEven : flashLineDecoOdd;
            next = Decoration.set([deco.range(line.from)]);
          }
        }
      }
      return next;
    },
    provide: (f) => EditorView.decorations.from(f),
  });

  const isDark = $derived(theme === "dark" || (theme === "system" && systemDark));
  const isMac =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/i.test(navigator.platform || navigator.userAgent);
  const modKey = isMac ? "⌘" : "Ctrl+";

  const latexLanguage = StreamLanguage.define(stex);

  const LATEX_COMPLETIONS: Completion[] = [
    snippetCompletion("\\section{${1:Title}}\n${0}", {
      label: "\\section{}",
      detail: "Section heading",
      type: "keyword",
    }),
    snippetCompletion("\\subsection{${1:Title}}\n${0}", {
      label: "\\subsection{}",
      detail: "Subsection heading",
      type: "keyword",
    }),
    snippetCompletion("\\subsubsection{${1:Title}}\n${0}", {
      label: "\\subsubsection{}",
      detail: "Subsubsection heading",
      type: "keyword",
    }),
    snippetCompletion("\\begin{equation}\n\t${1}\n\\end{equation}${0}", {
      label: "\\begin{equation}",
      detail: "Numbered equation",
      type: "snippet",
    }),
    snippetCompletion("\\begin{align}\n\t${1}\n\\end{align}${0}", {
      label: "\\begin{align}",
      detail: "Aligned equations",
      type: "snippet",
    }),
    snippetCompletion(
      "\\begin{figure}[htbp]\n\t\\centering\n\t\\includegraphics[width=0.8\\linewidth]{${1:path}}\n\t\\caption{${2:Caption}}\n\t\\label{fig:${3:label}}\n\\end{figure}${0}",
      {
        label: "\\begin{figure}",
        detail: "Figure environment",
        type: "snippet",
      }
    ),
    snippetCompletion(
      "\\begin{table}[htbp]\n\t\\centering\n\t\\caption{${1:Caption}}\n\t\\label{tab:${2:label}}\n\t\\begin{tabular}{${3:lcc}}\n\t\t${4}\n\t\\end{tabular}\n\\end{table}${0}",
      {
        label: "\\begin{table}",
        detail: "Table environment",
        type: "snippet",
      }
    ),
    snippetCompletion("\\begin{itemize}\n\t\\item ${1}\n\\end{itemize}${0}", {
      label: "\\begin{itemize}",
      detail: "Bulleted list",
      type: "snippet",
    }),
    snippetCompletion("\\begin{enumerate}\n\t\\item ${1}\n\\end{enumerate}${0}", {
      label: "\\begin{enumerate}",
      detail: "Numbered list",
      type: "snippet",
    }),
    snippetCompletion("\\textbf{${1:text}}${0}", {
      label: "\\textbf{}",
      detail: "Bold text",
      type: "function",
    }),
    snippetCompletion("\\textit{${1:text}}${0}", {
      label: "\\textit{}",
      detail: "Italic text",
      type: "function",
    }),
    snippetCompletion("\\emph{${1:text}}${0}", {
      label: "\\emph{}",
      detail: "Emphasized text",
      type: "function",
    }),
    snippetCompletion("\\cite{${1:key}}${0}", {
      label: "\\cite{}",
      detail: "Citation",
      type: "function",
    }),
    snippetCompletion("\\ref{${1:label}}${0}", {
      label: "\\ref{}",
      detail: "Cross-reference",
      type: "function",
    }),
    snippetCompletion("\\label{${1:key}}${0}", {
      label: "\\label{}",
      detail: "Anchor label",
      type: "function",
    }),
    snippetCompletion("\\usepackage{${1:package}}${0}", {
      label: "\\usepackage{}",
      detail: "Load package",
      type: "keyword",
    }),
    snippetCompletion("\\frac{${1:num}}{${2:den}}${0}", {
      label: "\\frac{}{}",
      detail: "Fraction",
      type: "function",
    }),
    snippetCompletion("\\sqrt{${1:expr}}${0}", {
      label: "\\sqrt{}",
      detail: "Square root",
      type: "function",
    }),
    snippetCompletion("\\sum_{${1:i=1}}^{${2:n}} ${0}", {
      label: "\\sum",
      detail: "Summation",
      type: "variable",
    }),
    snippetCompletion("\\int_{${1:a}}^{${2:b}} ${3} \\, d${4:x}${0}", {
      label: "\\int",
      detail: "Definite integral",
      type: "variable",
    }),
    snippetCompletion("\\prod_{${1:i=1}}^{${2:n}} ${0}", {
      label: "\\prod",
      detail: "Product",
      type: "variable",
    }),
    snippetCompletion("\\lim_{${1:x \\to \\infty}} ${0}", {
      label: "\\lim",
      detail: "Limit",
      type: "variable",
    }),
    snippetCompletion("\\includegraphics[width=${1:0.8\\linewidth}]{${2:file}}${0}", {
      label: "\\includegraphics{}",
      detail: "Include image",
      type: "function",
    }),
    snippetCompletion("\\caption{${1:Caption}}${0}", {
      label: "\\caption{}",
      detail: "Float caption",
      type: "function",
    }),
    snippetCompletion("\\title{${1:Title}}${0}", {
      label: "\\title{}",
      detail: "Document title",
      type: "keyword",
    }),
    snippetCompletion("\\author{${1:Author}}${0}", {
      label: "\\author{}",
      detail: "Document author",
      type: "keyword",
    }),
    { label: "\\maketitle", detail: "Render title block", type: "keyword" },
    { label: "\\tableofcontents", detail: "Insert table of contents", type: "keyword" },
    snippetCompletion("\\mathbb{${1:R}}${0}", {
      label: "\\mathbb{}",
      detail: "Blackboard bold",
      type: "function",
    }),
    snippetCompletion("\\mathcal{${1:L}}${0}", {
      label: "\\mathcal{}",
      detail: "Calligraphic font",
      type: "function",
    }),
    snippetCompletion("\\text{${1:text}}${0}", {
      label: "\\text{}",
      detail: "Text inside math mode",
      type: "function",
    }),
    { label: "\\alpha", detail: "Greek alpha", type: "constant" },
    { label: "\\beta", detail: "Greek beta", type: "constant" },
    { label: "\\gamma", detail: "Greek gamma", type: "constant" },
    { label: "\\theta", detail: "Greek theta", type: "constant" },
    { label: "\\lambda", detail: "Greek lambda", type: "constant" },
    { label: "\\pi", detail: "Greek pi", type: "constant" },
    { label: "\\infty", detail: "Infinity symbol", type: "constant" },
  ];

  function latexCompletionSource(context: CompletionContext): CompletionResult | null {
    const match = context.matchBefore(/\\[a-zA-Z*]*/);
    if (!match) return null;
    if (match.from === match.to && !context.explicit) return null;

    return {
      from: match.from,
      options: LATEX_COMPLETIONS,
      validFor: /^\\[a-zA-Z*]*$/,
    };
  }

  const lightHighlightStyle = HighlightStyle.define([
    { tag: tags.comment, color: "#8c8c8c", fontStyle: "italic" },
    { tag: tags.keyword, color: "#0033b3", fontWeight: "600" },
    { tag: tags.tagName, color: "#0033b3", fontWeight: "600" },
    { tag: tags.atom, color: "#00737a", fontWeight: "500" },
    { tag: tags.number, color: "#1750eb" },
    { tag: tags.string, color: "#067d17" },
    { tag: tags.bracket, color: "#0033b3", fontWeight: "600" },
    { tag: tags.standard(tags.variableName), color: "#871094" },
    { tag: tags.invalid, color: "#c62828", textDecoration: "underline" },
  ]);

  const darkHighlightStyle = HighlightStyle.define([
    { tag: tags.comment, color: "#7a7e85", fontStyle: "italic" },
    { tag: tags.keyword, color: "#cf8e6d", fontWeight: "600" },
    { tag: tags.tagName, color: "#cf8e6d", fontWeight: "600" },
    { tag: tags.atom, color: "#2aacb8", fontWeight: "500" },
    { tag: tags.number, color: "#2aacb8" },
    { tag: tags.string, color: "#6aab73" },
    { tag: tags.bracket, color: "#bcbec4", fontWeight: "600" },
    { tag: tags.standard(tags.variableName), color: "#cf8e6d" },
    { tag: tags.invalid, color: "#ef5350", textDecoration: "underline" },
  ]);

  function buildThemeExtension(dark: boolean): Extension {
    const editorTheme = EditorView.theme(
      {
        "&": {
          height: "100%",
          backgroundColor: dark ? "var(--bg-surface, #1a1a1c)" : "var(--bg-surface, #ffffff)",
          color: dark ? "var(--text-primary, #ededee)" : "var(--text-primary, #141415)",
          fontSize: "13.5px",
          fontFamily:
            "'JetBrains Mono', 'SF Mono', ui-monospace, Menlo, Monaco, Consolas, monospace",
        },
        ".cm-scroller": {
          overflow: "auto",
          lineHeight: "1.6",
          fontFamily: "inherit",
          scrollbarGutter: "stable",
          overscrollBehavior: "contain",
        },
        ".cm-content": {
          paddingBlock: "12px",
          caretColor: dark ? "#f59e0b" : "#b45309",
        },
        ".cm-gutters": {
          backgroundColor: dark ? "var(--bg-elevated, #222224)" : "var(--bg-elevated, #f5f5f6)",
          color: dark ? "var(--text-muted, #808086)" : "var(--text-muted, #6e6e73)",
          borderRight: dark
            ? "1px solid var(--border-subtle, #2d2d30)"
            : "1px solid var(--border-subtle, #d6d6d8)",
        },
        ".cm-activeLineGutter": {
          backgroundColor: dark ? "rgba(245, 158, 11, 0.14)" : "rgba(180, 83, 9, 0.10)",
          color: dark ? "#ededee" : "#141415",
        },
        ".cm-activeLine": {
          backgroundColor: dark ? "rgba(255, 255, 255, 0.04)" : "rgba(0, 0, 0, 0.035)",
        },
        "&.cm-focused .cm-cursor": {
          borderLeftColor: dark ? "#f59e0b" : "#b45309",
          borderLeftWidth: "2px",
        },
        "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
          backgroundColor: dark ? "rgba(245, 158, 11, 0.25)" : "rgba(180, 83, 9, 0.18)",
        },
        ".cm-panels": {
          backgroundColor: dark ? "var(--bg-elevated, #222224)" : "var(--bg-elevated, #f5f5f6)",
          color: dark ? "var(--text-primary, #ededee)" : "var(--text-primary, #141415)",
          borderTop: dark
            ? "1px solid var(--border-subtle, #2d2d30)"
            : "1px solid var(--border-subtle, #d6d6d8)",
          borderBottom: dark
            ? "1px solid var(--border-subtle, #2d2d30)"
            : "1px solid var(--border-subtle, #d6d6d8)",
        },
        ".cm-tooltip": {
          backgroundColor: dark ? "var(--bg-elevated, #222224)" : "var(--bg-elevated, #f5f5f6)",
          color: dark ? "var(--text-primary, #ededee)" : "var(--text-primary, #141415)",
          border: dark
            ? "1px solid var(--border-subtle, #2d2d30)"
            : "1px solid var(--border-subtle, #d6d6d8)",
          borderRadius: "6px",
          boxShadow: dark
            ? "var(--shadow-popover, 0 12px 32px rgba(0, 0, 0, 0.65))"
            : "var(--shadow-popover, 0 10px 28px rgba(0, 0, 0, 0.14))",
        },
        ".cm-tooltip-autocomplete > ul > li[aria-selected]": {
          backgroundColor: dark ? "#f59e0b" : "#b45309",
          color: dark ? "#101011" : "#ffffff",
        },
      },
      { dark }
    );

    return [
      editorTheme,
      syntaxHighlighting(dark ? darkHighlightStyle : lightHighlightStyle),
    ];
  }

  function normalizePathForMatch(value: string): string {
    return value.replace(/\\/g, "/").replace(/^\.\//, "").toLowerCase();
  }

  function matchesCurrentFile(diagFile?: string | null): boolean {
    if (!diagFile) return true;
    if (!filePath && !fileName) return true;

    const normDiag = normalizePathForMatch(diagFile);
    if (filePath) {
      const normCurrent = normalizePathForMatch(filePath);
      if (
        normDiag === normCurrent ||
        normCurrent.endsWith(`/${normDiag}`) ||
        normDiag.endsWith(`/${normCurrent}`)
      ) {
        return true;
      }
    }
    if (fileName && !normDiag.includes("/") && normDiag === fileName.toLowerCase()) {
      return true;
    }
    return false;
  }

  function applyDiagnosticsToView(targetView: EditorView, items: LatexDiagnostic[]): void {
    const doc = targetView.state.doc;
    const cmDiagnostics: Diagnostic[] = [];

    for (const item of items) {
      if (!matchesCurrentFile(item.file)) continue;
      if (item.line == null || !Number.isInteger(item.line)) continue;
      if (item.line < 1 || item.line > doc.lines) continue;

      const lineObj = doc.line(item.line);
      const firstNonSpace = lineObj.text.search(/\S/);
      const from = firstNonSpace >= 0 ? lineObj.from + firstNonSpace : lineObj.from;
      const to = Math.max(from, lineObj.to);

      cmDiagnostics.push({
        from,
        to,
        severity: item.severity,
        message: item.message,
      });
    }

    targetView.dispatch(setDiagnostics(targetView.state, cmDiagnostics));
  }

  function createEditorState(initialDoc: string): EditorState {
    return EditorState.create({
      doc: initialDoc,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        foldGutter(),
        drawSelection(),
        dropCursor(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        bracketMatching(),
        closeBrackets(),
        autocompletion({
          override: [latexCompletionSource],
          activateOnTyping: true,
        }),
        rectangularSelection(),
        crosshairCursor(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        search({ top: true }),
        linter(null),
        lintGutter(),
        latexLanguage,
        latexLanguage.data.of({ commentTokens: { line: "%" } }),
        EditorView.lineWrapping,
        flashLineField,
        Prec.highest(
          EditorView.domEventHandlers({
            keydown(event, targetView) {
              if (isCommentToggleShortcut(event)) {
                if (filePath === null) return false;
                event.preventDefault();
                return toggleLatexComment(targetView);
              }
              return false;
            },
          })
        ),
        Prec.highest(
          keymap.of([
            {
              key: "Mod-s",
              preventDefault: true,
              run: () => {
                onSaveAndCompile();
                return true;
              },
            },
            {
              key: "Mod-Shift-j",
              preventDefault: true,
              run: () => {
                if (!filePath || !synctexAvailable) return false;
                triggerSyncToPdf();
                return true;
              },
            },
          ])
        ),
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          ...foldKeymap,
          ...completionKeymap,
          ...lintKeymap,
          indentWithTab,
        ]),
        themeCompartment.of(buildThemeExtension(isDark)),
        editableCompartment.of(EditorView.editable.of(filePath !== null)),
        EditorView.updateListener.of((update) => {
          if (update.docChanged || update.selectionSet) {
            const head = update.state.selection.main.head;
            const line = update.state.doc.lineAt(head);
            cursorLine = line.number;
            cursorCol = head - line.from + 1;
          }

          if (update.docChanged) {
            const isExternal = update.transactions.some((tr) =>
              tr.annotation(externalSyncAnnotation)
            );
            if (!isExternal) {
              const nextDoc = update.state.doc.toString();
              lastSyncedContent = nextDoc;
              onChange(nextDoc);
            }
          }
        }),
      ],
    });
  }

  function wrapOrInsert(prefix: string, suffix: string, placeholder = ""): void {
    if (!view || filePath === null) return;
    const { from, to } = view.state.selection.main;
    const selected = view.state.sliceDoc(from, to);
    const body = selected || placeholder;
    const insert = `${prefix}${body}${suffix}`;
    const cursorPos = selected ? from + insert.length : from + prefix.length;

    view.dispatch({
      changes: { from, to, insert },
      selection: selected
        ? { anchor: cursorPos }
        : { anchor: cursorPos, head: cursorPos + placeholder.length },
      scrollIntoView: true,
    });
    view.focus();
  }

  function insertEnvironment(env: string): void {
    if (!view || filePath === null) return;
    const { from, to } = view.state.selection.main;
    const selected = view.state.sliceDoc(from, to);

    let insert = "";
    let anchorOffset = 0;

    if (env === "itemize") {
      const body = selected || "";
      insert = `\\begin{itemize}\n  \\item ${body}\n\\end{itemize}`;
      anchorOffset = "\\begin{itemize}\n  \\item ".length + body.length;
    } else if (env === "figure") {
      insert =
        "\\begin{figure}[htbp]\n  \\centering\n  \\includegraphics[width=0.8\\linewidth]{}\n  \\caption{}\n  \\label{fig:}\n\\end{figure}";
      anchorOffset = insert.indexOf("{}") + 1;
    } else {
      const body = selected || "";
      insert = `\\begin{${env}}\n  ${body}\n\\end{${env}}`;
      anchorOffset = `\\begin{${env}}\n  `.length + body.length;
    }

    view.dispatch({
      changes: { from, to, insert },
      selection: { anchor: from + anchorOffset },
      scrollIntoView: true,
    });
    view.focus();
  }

  function triggerToggleComment(): void {
    if (!view || filePath === null) return;
    toggleLatexComment(view);
    view.focus();
  }

  function triggerSearch(): void {
    if (!view || filePath === null) return;
    openSearchPanel(view);
    view.focus();
  }

  function triggerSyncToPdf(): void {
    if (!filePath || !synctexAvailable) return;
    if (view) {
      const head = view.state.selection.main.head;
      const line = view.state.doc.lineAt(head);
      onSyncToPdf?.(line.number, head - line.from + 1);
    } else {
      onSyncToPdf?.(cursorLine, cursorCol);
    }
  }

  function handlePaneKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented) return;
    if ((event.metaKey || event.ctrlKey) && !event.altKey) {
      const key = event.key.toLowerCase();
      if (key === "s" && !event.shiftKey) {
        event.preventDefault();
        onSaveAndCompile();
      } else if (key === "j" && event.shiftKey) {
        if (!filePath || !synctexAvailable) return;
        event.preventDefault();
        triggerSyncToPdf();
      } else if (isCommentToggleShortcut(event)) {
        if (!filePath) return;
        event.preventDefault();
        triggerToggleComment();
      }
    }
  }

  onMount(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    systemDark = mq.matches;
    const handleMediaChange = (e: MediaQueryListEvent) => {
      systemDark = e.matches;
    };
    mq.addEventListener("change", handleMediaChange);

    lastFilePath = filePath;
    lastSyncedContent = content;

    if (editorHost) {
      view = new EditorView({
        state: createEditorState(content),
        parent: editorHost,
      });
      applyDiagnosticsToView(view, diagnostics);
    }

    return () => {
      mq.removeEventListener("change", handleMediaChange);
      if (flashClearTimer) {
        clearTimeout(flashClearTimer);
        flashClearTimer = null;
      }
      view?.destroy();
      view = null;
    };
  });

  // Sync external file switches and external content updates without feedback loops
  $effect(() => {
    const nextPath = filePath;
    const nextContent = content;
    if (!view) return;

    const fileChanged = nextPath !== lastFilePath;
    const currentDoc = view.state.doc.toString();

    if (fileChanged) {
      if (flashClearTimer) {
        clearTimeout(flashClearTimer);
        flashClearTimer = null;
      }
      lastFilePath = nextPath;
      lastSyncedContent = nextContent;
      cursorLine = 1;
      cursorCol = 1;
      view.setState(createEditorState(nextContent));
      applyDiagnosticsToView(view, diagnostics);
      return;
    }

    if (nextContent !== lastSyncedContent && nextContent !== currentDoc) {
      lastSyncedContent = nextContent;
      view.dispatch({
        changes: { from: 0, to: currentDoc.length, insert: nextContent },
        annotations: externalSyncAnnotation.of(true),
      });
    }
  });

  // Update theme dynamically via Compartment
  $effect(() => {
    const dark = isDark;
    if (!view) return;
    view.dispatch({
      effects: themeCompartment.reconfigure(buildThemeExtension(dark)),
    });
  });

  // Update editability when active file opens or closes
  $effect(() => {
    const canEdit = filePath !== null;
    if (!view) return;
    view.dispatch({
      effects: editableCompartment.reconfigure(EditorView.editable.of(canEdit)),
    });
  });

  // Apply compiler diagnostics whenever diagnostics or active file changes
  $effect(() => {
    const currentDiagnostics = diagnostics;
    void filePath;
    void fileName;
    if (!view) return;
    applyDiagnosticsToView(view, currentDiagnostics);
  });

  // Jump to line when requested from PDF diagnostics drawer or SyncTeX inverse search
  $effect(() => {
    const target = jumpToLine;
    if (!view || !target) return;
    if (target.seq === lastJumpSeq) return;
    lastJumpSeq = target.seq;

    if (flashClearTimer) {
      clearTimeout(flashClearTimer);
      flashClearTimer = null;
    }

    const doc = view.state.doc;
    const lineNum = Math.max(1, Math.min(target.line, doc.lines));
    const lineObj = doc.line(lineNum);
    const parity = ++flashParity;

    view.dispatch({
      selection: { anchor: lineObj.from },
      effects: [
        EditorView.scrollIntoView(lineObj.from, { y: "center" }),
        setFlashLineEffect.of({ pos: lineObj.from, parity }),
      ],
    });
    view.focus();

    const activeView = view;
    flashClearTimer = setTimeout(() => {
      flashClearTimer = null;
      if (view === activeView) {
        activeView.dispatch({
          effects: setFlashLineEffect.of(null),
        });
      }
    }, 1600);
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  class="editor-pane"
  style:color-scheme={theme === "system" ? "light dark" : theme}
  onkeydown={handlePaneKeydown}
  aria-label="LaTeX Source Editor"
>
  <header class="editor-toolbar">
    <div class="breadcrumb" title={filePath ?? "No file selected"}>
      {#if filePath}
        <span class="breadcrumb-path">{filePath}</span>
        {#if isDirty}
          <span class="dirty-dot" title="Unsaved changes" aria-label="Unsaved changes"></span>
        {/if}
      {:else}
        <span class="breadcrumb-empty">Select a file from the sidebar to start editing</span>
      {/if}
    </div>

    <div class="toolbar-actions" role="toolbar" aria-label="LaTeX formatting">
      <button
        type="button"
        class="tool-btn"
        title={"Bold (\\textbf{})"}
        aria-label="Insert bold text"
        disabled={!filePath}
        onclick={() => wrapOrInsert("\\textbf{", "}")}
      >
        <Bold size={15} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="tool-btn"
        title={"Italic (\\textit{})"}
        aria-label="Insert italic text"
        disabled={!filePath}
        onclick={() => wrapOrInsert("\\textit{", "}")}
      >
        <Italic size={15} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="tool-btn"
        title={"Section (\\section{})"}
        aria-label="Insert section"
        disabled={!filePath}
        onclick={() => wrapOrInsert("\\section{", "}")}
      >
        <Heading size={15} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="tool-btn"
        title={`Toggle Comment (${modKey}ù / ${modKey}/)`}
        aria-label="Toggle LaTeX comment"
        disabled={!filePath}
        onclick={triggerToggleComment}
      >
        <Percent size={15} aria-hidden="true" />
      </button>

      <span class="tool-sep" aria-hidden="true"></span>

      <button
        type="button"
        class="tool-btn"
        title="Inline Math ($...$)"
        aria-label="Insert inline math"
        disabled={!filePath}
        onclick={() => wrapOrInsert("$", "$")}
      >
        <Sigma size={15} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="tool-btn"
        title="Equation Environment"
        aria-label="Insert equation environment"
        disabled={!filePath}
        onclick={() => insertEnvironment("equation")}
      >
        <SquareFunction size={15} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="tool-btn"
        title="Bullet List (itemize)"
        aria-label="Insert bullet list"
        disabled={!filePath}
        onclick={() => insertEnvironment("itemize")}
      >
        <List size={15} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="tool-btn"
        title="Figure Environment"
        aria-label="Insert figure"
        disabled={!filePath}
        onclick={() => insertEnvironment("figure")}
      >
        <Image size={15} aria-hidden="true" />
      </button>

      <span class="tool-sep" aria-hidden="true"></span>

      <button
        type="button"
        class="tool-btn"
        title="Find and Replace (Cmd/Ctrl+F)"
        aria-label="Find and Replace"
        disabled={!filePath}
        onclick={triggerSearch}
      >
        <Search size={15} aria-hidden="true" />
      </button>
      <button
        type="button"
        class="tool-btn sync-btn"
        title="Jump to PDF (SyncTeX · Cmd/Ctrl+Shift+J)"
        aria-label="Sync to PDF"
        disabled={!filePath || !synctexAvailable}
        onclick={triggerSyncToPdf}
      >
        <Crosshair size={14} aria-hidden="true" />
        <span>Sync to PDF</span>
      </button>
      <button
        type="button"
        class="tool-btn save-btn"
        title="Save & Compile (Cmd/Ctrl+S)"
        aria-label="Save and Compile"
        disabled={!filePath}
        onclick={onSaveAndCompile}
      >
        <Save size={14} aria-hidden="true" />
        <span>Save</span>
      </button>
    </div>
  </header>

  <div class="editor-body">
    {#if !filePath}
      <div class="empty-overlay" aria-hidden="true">
        <p>Open a <code>.tex</code> or <code>.bib</code> file from the project tree.</p>
      </div>
    {/if}
    <div class="codemirror-host" bind:this={editorHost} class:dimmed={!filePath}></div>
  </div>

  <footer class="editor-status">
    <div class="status-left">
      <span class="status-item">Ln {cursorLine}, Col {cursorCol}</span>
      <span class="status-sep" aria-hidden="true">•</span>
      <span class="status-item">UTF-8</span>
      <span class="status-sep" aria-hidden="true">•</span>
      <span class="status-item">LaTeX</span>
    </div>
    <div class="status-right">
      <span class="status-hint">{modKey}ù Comment • {modKey}S Save &amp; Compile</span>
    </div>
  </footer>
</section>

<style>
  .editor-pane {
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

  .editor-toolbar {
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

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 6px;
    min-inline-size: 72px;
    flex: 1 1 auto;
    font-size: 12px;
    color: var(--text-secondary, #404040);
  }

  .breadcrumb-path {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    color: var(--text-primary, #111111);
    font-weight: 500;
  }

  .breadcrumb-empty {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-style: italic;
  }

  .dirty-dot {
    width: 8px;
    height: 8px;
    border-radius: 9999px;
    flex-shrink: 0;
    background: var(--warning, #a15c07);
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .tool-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    min-height: 26px;
    min-width: 28px;
    padding: 0 6px;
    border: 1px solid transparent;
    border-radius: 5px;
    background: transparent;
    color: var(--text-secondary, #404040);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
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

  .save-btn {
    color: var(--accent, #b45309);
    padding: 0 8px;
  }

  .save-btn:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent, #b45309) 12%, transparent);
    color: var(--accent, #b45309);
  }

  .sync-btn {
    padding: 0 7px;
    white-space: nowrap;
  }

  .sync-btn:hover:not(:disabled) {
    background: color-mix(in oklab, var(--accent, #b45309) 12%, transparent);
    color: var(--accent, #b45309);
  }

  .tool-sep {
    width: 1px;
    height: 16px;
    margin: 0 4px;
    background: var(--border-subtle, #dfdfdf);
  }

  .editor-body {
    position: relative;
    flex: 1 1 auto;
    min-height: 0;
    overflow: hidden;
  }

  .codemirror-host {
    height: 100%;
    width: 100%;
  }

  .codemirror-host.dimmed {
    opacity: 0.35;
    pointer-events: none;
  }

  .empty-overlay {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: grid;
    place-items: center;
    pointer-events: none;
    color: var(--text-muted, #6e6e6e);
    font-size: 13px;
  }

  .editor-status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    height: 26px;
    padding: 0 12px;
    background: var(--bg-elevated, #fafafa);
    border-top: 1px solid var(--border-subtle, #dfdfdf);
    color: var(--text-muted, #6e6e6e);
    font-size: 11.5px;
    flex-shrink: 0;
    user-select: none;
  }

  .status-left {
    display: flex;
    align-items: center;
    gap: 8px;
    font-variant-numeric: tabular-nums;
  }

  .status-sep {
    opacity: 0.5;
  }

  .status-hint {
    white-space: nowrap;
  }

  .codemirror-host :global(.cm-scroller) {
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb, #737373) var(--scrollbar-track, transparent);
  }

  .codemirror-host :global(.cm-line.cm-synctex-flash-line) {
    position: relative;
  }

  .codemirror-host :global(.cm-line.cm-synctex-flash-line)::after {
    content: "";
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: color-mix(in oklab, var(--accent, #b45309) 20%, transparent);
    border-left: 3px solid var(--accent, #b45309);
    border-radius: 2px;
    opacity: 1;
    will-change: opacity, transform;
  }

  @media (prefers-reduced-motion: no-preference) {
    .codemirror-host :global(.cm-line.cm-synctex-flash-a)::after {
      animation: cm-synctex-flash-a 1.6s cubic-bezier(0.22, 1, 0.36, 1) forwards;
    }

    .codemirror-host :global(.cm-line.cm-synctex-flash-b)::after {
      animation: cm-synctex-flash-b 1.6s cubic-bezier(0.22, 1, 0.36, 1) forwards;
    }
  }

  @keyframes cm-synctex-flash-a {
    0% {
      opacity: 0;
      transform: scaleX(0.985);
    }
    14% {
      opacity: 1;
      transform: scaleX(1);
    }
    72% {
      opacity: 0.85;
      transform: scaleX(1);
    }
    100% {
      opacity: 0;
      transform: scaleX(1);
    }
  }

  @keyframes cm-synctex-flash-b {
    0% {
      opacity: 0;
      transform: scaleX(0.985);
    }
    14% {
      opacity: 1;
      transform: scaleX(1);
    }
    72% {
      opacity: 0.85;
      transform: scaleX(1);
    }
    100% {
      opacity: 0;
      transform: scaleX(1);
    }
  }

  @supports not (scrollbar-color: auto) {
    .codemirror-host :global(.cm-scroller) {
      scrollbar-width: auto;
    }

    .codemirror-host :global(.cm-scroller::-webkit-scrollbar) {
      width: 10px;
      height: 10px;
    }

    .codemirror-host :global(.cm-scroller::-webkit-scrollbar-thumb) {
      background: var(--scrollbar-thumb, #737373);
      border-radius: 5px;
    }

    .codemirror-host :global(.cm-scroller::-webkit-scrollbar-track) {
      background: var(--scrollbar-track, transparent);
    }
  }
</style>
