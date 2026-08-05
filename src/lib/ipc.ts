import { invoke } from "@tauri-apps/api/core";
import { RuachError } from "./error";
import type {
  AppSettings,
  DocOpenResult,
  ExportFormat,
  SearchHit,
  SessionDraft,
  SessionInfo,
  TreeNode,
} from "./types";

/**
 * Typed IPC bridge. Every call goes through here so the Rust AppError
 * envelope `{ code, message }` becomes a RuachError on the JS side.
 */
export async function ipc<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw RuachError.fromUnknown(e);
  }
}

/* ---- Command wrappers (mirror commands.rs) ---- */

export const vaultOpen = (path: string) => ipc<void>("vault_open", { path });
export const vaultScan = () => ipc<TreeNode[]>("vault_scan");

export const docOpen = (relPath: string) => ipc<DocOpenResult>("doc_open", { relPath });
/** Returns the new file mtime (baseline for the next autosave). */
export const docSave = (relPath: string, content: string, expectedMtime?: number) =>
  ipc<number>("doc_save", { relPath, content, expectedMtime });

export const sessionFlush = (docKey: string, content: string, cursor?: number | null) =>
  ipc<void>("session_flush", { docKey, content, cursor });
export const sessionList = () => ipc<SessionInfo[]>("session_list");
export const sessionRestore = (docKey: string) => ipc<SessionDraft>("session_restore", { docKey });
export const sessionDiscard = (docKey: string) => ipc<void>("session_discard", { docKey });

export const indexFile = (relPath: string) => ipc<void>("index_file", { relPath });
export const indexReindex = () => ipc<number>("index_reindex");

export const searchQuery = (q: string) => ipc<SearchHit[]>("search_query", { q });

export const attachPaste = (dataUrl: string) => ipc<{ rel_path: string }>("attach_paste", { dataUrl });

export const renderMarkdown = (content: string) => ipc<string>("render_markdown", { content });

export const exportDocument = (relPath: string, format: ExportFormat, destDir?: string) =>
  ipc<string>("export_document", { relPath, format, destDir });

export const windowCreate = (relPath?: string) => ipc<void>("window_create", { relPath });

export const configLoad = () => ipc<AppSettings>("config_load");
export const configSave = (settings: AppSettings) => ipc<void>("config_save", { settings });
