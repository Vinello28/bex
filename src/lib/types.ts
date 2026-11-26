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
