import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import type {
  AiAgentRequest,
  AiAgentResponse,
  AiEnvStatus,
  AppUpdateInfo,
  CacheBucketInfo,
  CompileCacheInfo,
  CompileRequest,
  CompileResponse,
  CompilerInfo,
  FileNode,
  LatexDiagnostic,
  McpToolInfo,
  SaveResult,
  SyncTexForwardRequest,
  SyncTexInverseRequest,
  SyncTexLocation,
  SyncTexRect,
  UpdateProgress,
  WorkspaceInfo,
} from "./types";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

// --- Browser preview virtual workspace (used only when !isTauriRuntime()) ---

const MOCK_ROOT = "/mock-workspace";
let mockProjectCacheBytes = 245_760;
let mockProjectCacheFiles = 6;
let mockTectonicCacheBytes = 48_234_496;
let mockTectonicCacheFiles = 142;

function buildMockCompileCacheInfo(rootDir: string | null): CompileCacheInfo {
  const normRoot = rootDir ? normalizeMockPath(rootDir) : null;
  const projectCache: CacheBucketInfo = normRoot
    ? {
        path: `${normRoot}/.bex-build`,
        bytes: mockProjectCacheBytes,
        fileCount: mockProjectCacheFiles,
        exists: mockProjectCacheBytes > 0,
      }
    : {
        path: null,
        bytes: 0,
        fileCount: 0,
        exists: false,
      };

  const tectonicCache: CacheBucketInfo = {
    path: "/mock-cache/Tectonic",
    bytes: mockTectonicCacheBytes,
    fileCount: mockTectonicCacheFiles,
    exists: mockTectonicCacheBytes > 0,
  };

  return {
    projectCache,
    tectonicCache,
    totalBytes: projectCache.bytes + tectonicCache.bytes,
  };
}
const mockDirs = new Set<string>([`${MOCK_ROOT}/sections`]);
const mockFiles = new Map<string, string>([
  [
    `${MOCK_ROOT}/main.tex`,
    `\\documentclass[11pt,a4paper]{article}
\\usepackage[utf8]{inputenc}
\\usepackage[T1]{fontenc}
\\usepackage{amsmath,amssymb,graphicx}
\\usepackage{hyperref}

\\title{Bex LaTeX Studio — Sample Paper}
\\author{Research Team}
\\date{\\today}

\\begin{document}

\\maketitle

\\begin{abstract}
This sample project demonstrates live editing, LaTeX syntax highlighting,
inline diagnostics, and real-time PDF rendering inside Bex LaTeX Studio.
\\end{abstract}

\\section{Introduction}
\\label{sec:intro}
Press \\textbf{Cmd/Ctrl + S} to save and trigger compilation.
You can edit sections modularly in \\texttt{sections/introduction.tex}.

\\input{sections/introduction}

\\section{Mathematical Formulation}
Consider the continuous optimization objective:
\\begin{equation}
  \\mathcal{L}(\\theta) = \\frac{1}{N} \\sum_{i=1}^{N} \\left( y_i - f_{\\theta}(x_i) \\right)^2 + \\lambda \\|\\theta\\|_2^2
\\end{equation}

\\begin{itemize}
  \\item Fast Rust-native Tectonic compilation
  \\item Multi-page HiDPI PDF preview
  \\item Clickable error diagnostics
\\end{itemize}

\\end{document}
`,
  ],
  [
    `${MOCK_ROOT}/sections/introduction.tex`,
    `\\subsection{Background}
Modern scientific writing benefits from instant feedback loops between source
editing and typeset PDF output~\\cite{lamport1994latex}.

\\begin{equation}
  \\int_{-\\infty}^{\\infty} e^{-x^2}\\,dx = \\sqrt{\\pi}
\\end{equation}
`,
  ],
  [
    `${MOCK_ROOT}/references.bib`,
    `@book{lamport1994latex,
  title     = {LaTeX: A Document Preparation System},
  author    = {Lamport, Leslie},
  year      = {1994},
  publisher = {Addison-Wesley}
}
`,
  ],
]);

function normalizeMockPath(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "");
}

function resolveMockPath(path: string): string {
  const norm = normalizeMockPath(path).replace(/^\.\//, "");
  if (!norm || norm === MOCK_ROOT) return MOCK_ROOT;
  if (norm.startsWith(`${MOCK_ROOT}/`)) return norm;
  const rel = norm.replace(/^\/+/, "");
  return `${MOCK_ROOT}/${rel}`;
}

function getMockRelPath(absPath: string, rootPath: string): string {
  const normAbs = normalizeMockPath(absPath);
  const normRoot = normalizeMockPath(rootPath);
  if (normAbs.startsWith(`${normRoot}/`)) {
    return normAbs.slice(normRoot.length + 1);
  }
  return normAbs;
}

function buildMockTree(rootPath: string): FileNode[] {
  const normRoot = normalizeMockPath(rootPath);
  const rootNodes: FileNode[] = [];
  const dirMap = new Map<string, FileNode>();

  const ensureDirNode = (relDir: string): FileNode[] => {
    if (!relDir) return rootNodes;
    const existing = dirMap.get(relDir);
    if (existing) return existing.children ?? (existing.children = []);

    const parts = relDir.split("/");
    const name = parts[parts.length - 1];
    const parentRel = parts.slice(0, -1).join("/");
    const parentList = ensureDirNode(parentRel);

    const node: FileNode = {
      name,
      path: relDir,
      absPath: `${normRoot}/${relDir}`,
      isDir: true,
      children: [],
      extension: null,
    };
    parentList.push(node);
    dirMap.set(relDir, node);
    return node.children!;
  };

  for (const dirAbs of mockDirs) {
    const rel = getMockRelPath(dirAbs, normRoot);
    if (rel && rel !== dirAbs) {
      ensureDirNode(rel);
    }
  }

  for (const fileAbs of mockFiles.keys()) {
    const rel = getMockRelPath(fileAbs, normRoot);
    const parts = rel.split("/");
    const name = parts[parts.length - 1];
    const parentRel = parts.slice(0, -1).join("/");
    const parentList = ensureDirNode(parentRel);
    const dotIdx = name.lastIndexOf(".");
    const extension = dotIdx > 0 ? name.slice(dotIdx + 1) : null;

    parentList.push({
      name,
      path: rel,
      absPath: fileAbs,
      isDir: false,
      children: null,
      extension,
    });
  }

  const sortNodes = (nodes: FileNode[]) => {
    nodes.sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    for (const node of nodes) {
      if (node.children) sortNodes(node.children);
    }
  };

  sortNodes(rootNodes);
  return rootNodes;
}

function escapePdfText(text: string): string {
  return text
    .replace(/[^\x20-\x7e]/g, " ")
    .replace(/\\/g, "\\\\")
    .replace(/\(/g, "\\(")
    .replace(/\)/g, "\\)");
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

/** Build a valid single-page PDF document for browser preview mode. */
function buildPreviewPdfBase64(entryFile: string, source: string): string {
  const titleMatch = source.match(/\\title\{([^}]*)\}/);
  const title = titleMatch?.[1]?.trim() || entryFile;
  const rawLines = source
    .replace(/\\(begin|end)\{[^}]*\}/g, "")
    .replace(/\\[a-zA-Z]+\*?(?:\[[^\]]*\])?/g, " ")
    .replace(/[{}]/g, "")
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith("%"))
    .slice(0, 26);

  const ops: string[] = [
    "BT",
    "/F1 18 Tf",
    "56 760 Td",
    `(${escapePdfText(title)}) Tj`,
    "/F1 10 Tf",
    "0 -22 Td",
    `(${escapePdfText(`Browser Preview Mode • ${entryFile}`)}) Tj`,
    "/F1 11 Tf",
    "0 -26 Td",
  ];

  for (const line of rawLines) {
    const clipped = line.length > 82 ? `${line.slice(0, 79)}...` : line;
    ops.push(`(${escapePdfText(clipped)}) Tj`);
    ops.push("0 -16 Td");
  }
  ops.push("ET");

  const stream = ops.join("\n");
  const objects = [
    "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n",
    "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
    "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n",
    `4 0 obj\n<< /Length ${stream.length} >>\nstream\n${stream}\nendstream\nendobj\n`,
    "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n",
  ];

  let pdf = "%PDF-1.4\n";
  const offsets: number[] = [0];
  for (const obj of objects) {
    offsets.push(pdf.length);
    pdf += obj;
  }

  const xrefStart = pdf.length;
  pdf += `xref\n0 ${objects.length + 1}\n`;
  pdf += "0000000000 65535 f \n";
  for (let i = 1; i < offsets.length; i++) {
    pdf += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
  }
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefStart}\n%%EOF\n`;

  const encoded = new TextEncoder().encode(pdf);
  return bytesToBase64(encoded);
}

function analyzeMockLatex(entryFile: string, source: string): LatexDiagnostic[] {
  const diagnostics: LatexDiagnostic[] = [];
  const lines = source.split(/\r?\n/);
  let braceBalance = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNo = i + 1;
    const stripped = line.replace(/(^|[^\\])%.*$/, "$1");

    for (let j = 0; j < stripped.length; j++) {
      if (stripped[j] === "{" && stripped[j - 1] !== "\\") braceBalance++;
      if (stripped[j] === "}" && stripped[j - 1] !== "\\") braceBalance--;
    }

    if (stripped.includes("\\undefined")) {
      diagnostics.push({
        severity: "error",
        file: entryFile,
        line: lineNo,
        message: "Undefined control sequence \\undefined",
      });
    }
    if (stripped.includes("TODO")) {
      diagnostics.push({
        severity: "warning",
        file: entryFile,
        line: lineNo,
        message: "Unresolved TODO comment in manuscript",
      });
    }
  }

  if (braceBalance !== 0) {
    diagnostics.push({
      severity: "error",
      file: entryFile,
      line: Math.max(1, lines.length),
      message: `Unbalanced curly braces detected (balance: ${braceBalance > 0 ? `+${braceBalance}` : braceBalance})`,
    });
  }

  return diagnostics;
}

// --- Public Tauri IPC API ---

export async function initDefaultWorkspace(): Promise<WorkspaceInfo> {
  if (!isTauriRuntime()) {
    const files = buildMockTree(MOCK_ROOT);
    return {
      rootPath: MOCK_ROOT,
      name: "sample_project",
      files,
      defaultEntryFile: "main.tex",
    };
  }
  return invoke<WorkspaceInfo>("init_default_workspace");
}

export async function openWorkspace(rootPath: string): Promise<WorkspaceInfo> {
  if (!isTauriRuntime()) {
    const norm = normalizeMockPath(rootPath) || MOCK_ROOT;
    const parts = norm.split("/");
    return {
      rootPath: norm,
      name: parts[parts.length - 1] || "workspace",
      files: buildMockTree(norm),
      defaultEntryFile: "main.tex",
    };
  }
  return invoke<WorkspaceInfo>("open_workspace", { rootPath });
}

export async function createNewProject(
  rootPath: string,
  seedTemplate = true
): Promise<WorkspaceInfo> {
  if (!isTauriRuntime()) {
    const norm = normalizeMockPath(rootPath) || MOCK_ROOT;
    const parts = norm.split("/");
    if (!mockFiles.has(`${norm}/main.tex`)) {
      const sample = mockFiles.get(`${MOCK_ROOT}/main.tex`) ?? "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n";
      mockFiles.set(`${norm}/main.tex`, seedTemplate ? sample : "\\documentclass{article}\n\\begin{document}\n\n\\end{document}\n");
    }
    return {
      rootPath: norm,
      name: parts[parts.length - 1] || "new_project",
      files: buildMockTree(norm),
      defaultEntryFile: "main.tex",
    };
  }
  return invoke<WorkspaceInfo>("create_new_project", { rootPath, seedTemplate });
}

export async function listWorkspaceFiles(rootPath: string): Promise<FileNode[]> {
  if (!isTauriRuntime()) {
    return buildMockTree(rootPath);
  }
  return invoke<FileNode[]>("list_workspace_files", { rootPath });
}

export async function readTextFile(path: string): Promise<string> {
  if (!isTauriRuntime()) {
    const norm = resolveMockPath(path);
    const content = mockFiles.get(norm);
    if (content === undefined) {
      throw new Error(`Mock file not found: ${path}`);
    }
    return content;
  }
  return invoke<string>("read_text_file", { path });
}

export async function saveTextFile(path: string, content: string): Promise<SaveResult> {
  if (!isTauriRuntime()) {
    const norm = resolveMockPath(path);
    mockFiles.set(norm, content);
    return {
      path: norm,
      bytesWritten: new TextEncoder().encode(content).length,
    };
  }
  return invoke<SaveResult>("save_text_file", { path, content });
}

export async function createFile(path: string, isDir: boolean): Promise<void> {
  if (!isTauriRuntime()) {
    const norm = resolveMockPath(path);
    if (isDir) {
      mockDirs.add(norm);
    } else {
      const parent = norm.slice(0, norm.lastIndexOf("/"));
      if (parent && parent !== MOCK_ROOT) mockDirs.add(parent);
      mockFiles.set(norm, "");
    }
    return;
  }
  return invoke<void>("create_file", { path, isDir });
}

export async function renamePath(oldPath: string, newPath: string): Promise<void> {
  if (!isTauriRuntime()) {
    const normOld = resolveMockPath(oldPath);
    const normNew = resolveMockPath(newPath);

    if (mockFiles.has(normOld)) {
      const val = mockFiles.get(normOld) ?? "";
      mockFiles.delete(normOld);
      mockFiles.set(normNew, val);
      return;
    }

    if (mockDirs.has(normOld)) {
      mockDirs.delete(normOld);
      mockDirs.add(normNew);
      for (const dir of [...mockDirs]) {
        if (dir.startsWith(`${normOld}/`)) {
          mockDirs.delete(dir);
          mockDirs.add(`${normNew}${dir.slice(normOld.length)}`);
        }
      }
      for (const [key, val] of [...mockFiles.entries()]) {
        if (key.startsWith(`${normOld}/`)) {
          mockFiles.delete(key);
          mockFiles.set(`${normNew}${key.slice(normOld.length)}`, val);
        }
      }
    }
    return;
  }
  return invoke<void>("rename_path", { oldPath, newPath });
}

export async function deletePath(path: string): Promise<void> {
  if (!isTauriRuntime()) {
    const norm = resolveMockPath(path);
    mockFiles.delete(norm);
    mockDirs.delete(norm);
    for (const dir of [...mockDirs]) {
      if (dir.startsWith(`${norm}/`)) mockDirs.delete(dir);
    }
    for (const key of [...mockFiles.keys()]) {
      if (key.startsWith(`${norm}/`)) mockFiles.delete(key);
    }
    return;
  }
  return invoke<void>("delete_path", { path });
}

export async function exportPdf(sourcePdf: string, destPath: string): Promise<void> {
  if (!isTauriRuntime()) {
    return;
  }
  return invoke<void>("export_pdf", { sourcePdf, destPath });
}

export async function detectCompilers(): Promise<CompilerInfo[]> {
  if (!isTauriRuntime()) {
    return [
      {
        id: "tectonic",
        name: "Tectonic (Browser Preview Mock)",
        available: true,
        version: "0.15.0 (mock)",
        executablePath: null,
        isRustNative: true,
      },
      {
        id: "pdflatex",
        name: "pdfLaTeX",
        available: false,
        version: null,
        executablePath: null,
        isRustNative: false,
      },
    ];
  }
  return invoke<CompilerInfo[]>("detect_compilers");
}

export async function compileLatex(request: CompileRequest): Promise<CompileResponse> {
  if (!isTauriRuntime()) {
    const start = performance.now();
    const normRoot = normalizeMockPath(request.rootDir);
    const fullEntry = request.entryFile.startsWith("/")
      ? normalizeMockPath(request.entryFile)
      : `${normRoot}/${request.entryFile.replace(/^\/+/, "")}`;
    const source = mockFiles.get(fullEntry) ?? "";
    const diagnostics = analyzeMockLatex(request.entryFile, source);
    const hasError = diagnostics.some((d) => d.severity === "error");
    const pdfBase64 = hasError ? null : buildPreviewPdfBase64(request.entryFile, source);
    const durationMs = Math.max(12, Math.round(performance.now() - start));

    const logLines = [
      `This is ${request.engine} (Bex Browser Preview Mock)`,
      `Compiling entry file: ${request.entryFile}`,
      ...diagnostics.map(
        (d) => `${d.severity.toUpperCase()} ${d.file ?? ""}:${d.line ?? 1}: ${d.message}`
      ),
      hasError
        ? "! Emergency stop: compilation failed."
        : `Output written on ${request.entryFile.replace(/\.tex$/i, ".pdf")} (1 page).`,
    ];

    if (!hasError) {
      mockProjectCacheBytes = Math.max(mockProjectCacheBytes, 245_760);
      mockProjectCacheFiles = Math.max(mockProjectCacheFiles, 6);
      mockTectonicCacheBytes = Math.max(mockTectonicCacheBytes, 48_234_496);
      mockTectonicCacheFiles = Math.max(mockTectonicCacheFiles, 142);
    }

    return {
      success: !hasError,
      pdfBase64,
      pdfPath: hasError ? null : `${normRoot}/${request.entryFile.replace(/\.tex$/i, ".pdf")}`,
      logOutput: logLines.join("\n"),
      diagnostics,
      durationMs,
      engineUsed: request.engine,
      synctexAvailable: !hasError,
    };
  }
  return invoke<CompileResponse>("compile_latex", { request });
}

export async function synctexForward(
  request: SyncTexForwardRequest
): Promise<SyncTexRect[]> {
  if (!isTauriRuntime()) {
    return [
      {
        page: 1,
        x: 72,
        y: 120 + Math.min(600, request.line * 14),
        width: 420,
        height: 16,
      },
    ];
  }
  return invoke<SyncTexRect[]>("synctex_forward", { request });
}

export async function synctexInverse(
  request: SyncTexInverseRequest
): Promise<SyncTexLocation | null> {
  if (!isTauriRuntime()) {
    return {
      file: request.entryFile,
      line: Math.max(1, Math.round((request.y - 100) / 14)),
      column: 1,
    };
  }
  return invoke<SyncTexLocation | null>("synctex_inverse", { request });
}

export async function getCompileCacheInfo(
  rootDir: string | null
): Promise<CompileCacheInfo> {
  if (!isTauriRuntime()) {
    return buildMockCompileCacheInfo(rootDir);
  }
  return invoke<CompileCacheInfo>("get_compile_cache_info", { rootDir });
}

export async function clearCompileCache(
  rootDir: string | null,
  scope: "project" | "tectonic" | "all"
): Promise<CompileCacheInfo> {
  if (!isTauriRuntime()) {
    if (scope === "project" || scope === "all") {
      mockProjectCacheBytes = 0;
      mockProjectCacheFiles = 0;
    }
    if (scope === "tectonic" || scope === "all") {
      mockTectonicCacheBytes = 0;
      mockTectonicCacheFiles = 0;
    }
    return buildMockCompileCacheInfo(rootDir);
  }
  return invoke<CompileCacheInfo>("clear_compile_cache", { rootDir, scope });
}

export async function pickWorkspaceFolder(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Open LaTeX Project Folder",
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickPdfSavePath(defaultName: string): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  return save({
    defaultPath: defaultName,
    filters: [{ name: "PDF Document", extensions: ["pdf"] }],
  });
}

export async function pickAttachmentFile(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }
  const selected = await open({
    directory: false,
    multiple: false,
    title: "Attach File for Bex AI",
    filters: [
      {
        name: "Text & LaTeX Documents",
        extensions: ["tex", "bib", "md", "txt", "csv", "json", "sty", "cls", "log"],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

// --- MCP & AI Agent IPC API ---

const MOCK_MCP_TOOLS: McpToolInfo[] = [
  {
    name: "get_editor_context",
    description: "Inspect the current workspace root, active file, entry .tex file, compiler engine, and diagnostics.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "list_project_files",
    description: "List all files and directories in the current LaTeX workspace.",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "read_file",
    description: "Read the UTF-8 contents of a file inside the workspace.",
    inputSchema: {
      type: "object",
      properties: { path: { type: "string", description: "Workspace-relative file path" } },
      required: ["path"],
    },
  },
  {
    name: "write_file",
    description: "Overwrite or write a file inside the workspace.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string", description: "Workspace-relative file path" },
        content: { type: "string", description: "Full file content" },
      },
      required: ["path", "content"],
    },
  },
  {
    name: "insert_text",
    description: "Insert LaTeX or text into an existing file at a chosen anchor.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string" },
        text: { type: "string" },
        position: { type: "string", enum: ["before_end_document", "end", "start", "line"] },
        line_number: { type: "integer" },
      },
      required: ["text"],
    },
  },
  {
    name: "replace_in_file",
    description: "Replace an exact string inside a workspace file.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string" },
        search: { type: "string" },
        replacement: { type: "string" },
        replace_all: { type: "boolean" },
      },
      required: ["search", "replacement"],
    },
  },
  {
    name: "create_file",
    description: "Create a new file or directory in the workspace.",
    inputSchema: {
      type: "object",
      properties: {
        path: { type: "string" },
        content: { type: "string" },
        is_dir: { type: "boolean" },
      },
      required: ["path"],
    },
  },
  {
    name: "compile_project",
    description: "Compile the current LaTeX project to PDF and return diagnostics.",
    inputSchema: {
      type: "object",
      properties: { entry_file: { type: "string" } },
    },
  },
];

export async function mcpListTools(): Promise<McpToolInfo[]> {
  if (!isTauriRuntime()) {
    return MOCK_MCP_TOOLS;
  }
  return invoke<McpToolInfo[]>("mcp_list_tools");
}

export async function mcpJsonRpc(
  request: Record<string, unknown>,
  workspaceRoot: string,
  activeFile?: string | null,
  entryFile?: string | null,
  selectedEngine?: string
): Promise<unknown> {
  if (!isTauriRuntime()) {
    return {
      jsonrpc: "2.0",
      id: request.id ?? null,
      result: { tools: MOCK_MCP_TOOLS },
    };
  }
  return invoke<unknown>("mcp_jsonrpc", {
    request,
    workspaceRoot,
    activeFile: activeFile ?? null,
    entryFile: entryFile ?? null,
    selectedEngine: selectedEngine ?? "auto",
  });
}

export async function aiGetEnvStatus(): Promise<AiEnvStatus> {
  if (!isTauriRuntime()) {
    return {
      hasGoogleEnvKey: false,
      defaultGoogleModel: "gemini-2.5-flash",
    };
  }
  return invoke<AiEnvStatus>("ai_get_env_status");
}

export async function aiListModels(
  provider: string,
  baseUrl: string,
  apiKey: string
): Promise<string[]> {
  if (!isTauriRuntime()) {
    if (provider === "google") return ["gemini-2.5-flash", "gemini-2.5-pro"];
    if (provider === "ollama") return ["llama3.2", "qwen2.5-coder"];
    return [baseUrl ? "local-model" : "default"];
  }
  return invoke<string[]>("ai_list_models", { provider, baseUrl, apiKey });
}

export async function aiSendMessage(request: AiAgentRequest): Promise<AiAgentResponse> {
  if (!isTauriRuntime()) {
    return {
      assistantMessage:
        "Browser preview mode is active. Run the desktop Tauri app (`npm run tauri dev`) to connect Bex AI to Google AI Studio, Ollama, LM Studio, or vLLM with live MCP workspace tools.",
      toolTraces: [],
      mutations: [],
      compileResult: null,
    };
  }
  return invoke<AiAgentResponse>("ai_send_message", { request });
}

// --- App Auto-Updater API ---

let pendingUpdate: Update | null = null;

async function releasePendingUpdate(): Promise<void> {
  if (!pendingUpdate) return;
  const prev = pendingUpdate;
  pendingUpdate = null;
  await prev.close().catch(() => {});
}

export const FALLBACK_APP_VERSION = __APP_VERSION__;

export async function getCurrentAppVersion(): Promise<string> {
  if (!isTauriRuntime()) {
    return FALLBACK_APP_VERSION;
  }
  try {
    return await getVersion();
  } catch {
    return FALLBACK_APP_VERSION;
  }
}

export async function checkForAppUpdate(): Promise<AppUpdateInfo> {
  if (!isTauriRuntime()) {
    return {
      available: false,
      currentVersion: FALLBACK_APP_VERSION,
      version: null,
      date: null,
      body: null,
    };
  }

  await releasePendingUpdate();
  const currentVersion = await getCurrentAppVersion();
  const update = await check();

  if (update) {
    await releasePendingUpdate();
    pendingUpdate = update;
    return {
      available: true,
      currentVersion: update.currentVersion || currentVersion,
      version: update.version,
      date: update.date ?? null,
      body: update.body ?? null,
    };
  }

  return {
    available: false,
    currentVersion,
    version: null,
    date: null,
    body: null,
  };
}

export async function downloadAndInstallAppUpdate(
  onProgress?: (progress: UpdateProgress) => void
): Promise<void> {
  if (!isTauriRuntime()) {
    throw new Error("In-app updates are only available in the desktop app.");
  }

  if (!pendingUpdate) {
    await checkForAppUpdate();
  }

  const update = pendingUpdate;
  if (!update) {
    throw new Error("No update package is currently available.");
  }

  let downloadedBytes = 0;
  let contentLength: number | null = null;

  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      contentLength = event.data.contentLength ?? null;
      onProgress?.({
        downloadedBytes: 0,
        contentLength,
        percent: contentLength ? 0 : null,
      });
    } else if (event.event === "Progress") {
      downloadedBytes += event.data.chunkLength;
      const percent =
        contentLength && contentLength > 0
          ? Math.min(100, Math.round((downloadedBytes / contentLength) * 100))
          : null;
      onProgress?.({
        downloadedBytes,
        contentLength,
        percent,
      });
    } else if (event.event === "Finished") {
      onProgress?.({
        downloadedBytes,
        contentLength,
        percent: 100,
      });
    }
  });

  if (pendingUpdate === update) {
    pendingUpdate = null;
    await update.close().catch(() => {});
  }
}

export async function relaunchApp(): Promise<void> {
  if (!isTauriRuntime()) return;
  await relaunch();
}
