export type ThemeMode = "system" | "light" | "dark";

export type AppView = "home" | "editor";

export type SettingsSectionId =
  | "compilation"
  | "appearance"
  | "ai"
  | "updates"
  | "about";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "up-to-date"
  | "downloading"
  | "ready-to-restart"
  | "error";

export interface RecentProject {
  path: string;
  name: string;
  lastOpened: number;
  entryFile?: string | null;
  lastActiveFile?: string | null;
}

export interface RecentFile {
  workspacePath: string;
  workspaceName: string;
  relPath: string;
  fileName: string;
  lastOpened: number;
}

export type AiProviderId = "google" | "ollama" | "lmstudio" | "vllm";

export interface AiSettings {
  provider: AiProviderId;
  baseUrl: string;
  apiKey: string;
  model: string;
  autoCompileAfterAiEdit: boolean;
}

export interface AiEnvStatus {
  hasGoogleEnvKey: boolean;
  defaultGoogleModel: string;
}

export interface AiAttachment {
  name: string;
  path: string;
  content: string;
}

export interface McpToolInfo {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
}

export interface EditorMutation {
  kind: "write" | "create" | "insert" | "replace";
  relPath: string;
  absPath: string;
  content?: string | null;
}

export interface McpToolTrace {
  id: string;
  toolName: string;
  argumentsJson: string;
  resultSummary: string;
  isError: boolean;
  mutation?: EditorMutation | null;
}

export interface AiChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  timestamp: number;
  attachments?: { name: string; path: string }[];
  toolTraces?: McpToolTrace[];
  error?: boolean;
}

export interface AiAgentRequest {
  settings: AiSettings;
  messages: { role: "user" | "assistant"; content: string }[];
  attachments: AiAttachment[];
  workspaceRoot: string;
  activeFilePath: string | null;
  activeFileContent: string | null;
  entryFile: string | null;
  selectedEngine: string;
  diagnostics: LatexDiagnostic[];
}

export interface AiAgentResponse {
  assistantMessage: string;
  toolTraces: McpToolTrace[];
  mutations: EditorMutation[];
  compileResult?: CompileResponse | null;
}

export interface FileNode {
  name: string;
  path: string;
  absPath: string;
  isDir: boolean;
  children?: FileNode[] | null;
  extension?: string | null;
}

export interface WorkspaceInfo {
  rootPath: string;
  name: string;
  files: FileNode[];
  defaultEntryFile?: string | null;
}

export interface SaveResult {
  path: string;
  bytesWritten: number;
}

export interface CompilerInfo {
  id: string;
  name: string;
  available: boolean;
  version?: string | null;
  executablePath?: string | null;
  isRustNative: boolean;
}

export interface CompileRequest {
  rootDir: string;
  entryFile: string;
  engine: string;
}

export interface LatexDiagnostic {
  severity: "error" | "warning" | "info";
  file?: string | null;
  line?: number | null;
  message: string;
}

export interface CompileResponse {
  success: boolean;
  pdfBase64?: string | null;
  pdfPath?: string | null;
  logOutput: string;
  diagnostics: LatexDiagnostic[];
  durationMs: number;
  engineUsed: string;
}

export interface AppUpdateInfo {
  available: boolean;
  currentVersion: string;
  version: string | null;
  date: string | null;
  body: string | null;
}

export interface UpdateProgress {
  downloadedBytes: number;
  contentLength: number | null;
  percent: number | null;
}
