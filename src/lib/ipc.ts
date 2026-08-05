import { invoke } from "@tauri-apps/api/core";
import { RuachError } from "./error";
import type {
  AppSettings,
  AttachmentData,
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
  // Plain-browser visit (vite dev without the Tauri shell): no IPC runtime.
  if (!("__TAURI_INTERNALS__" in window)) {
    throw new RuachError(
      "no_tauri",
      "此页面在浏览器中打开，无法调用桌面功能；请用 npm run tauri dev 运行",
    );
  }
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
/**
 * Returns the new file mtime (baseline for the next autosave).
 * `expectedMtime: null` skips the conflict check (forced overwrite).
 */
export const docSave = (relPath: string, content: string, expectedMtime?: number | null) =>
  ipc<number>("doc_save", { relPath, content, expectedMtime });

export const sessionFlush = (docKey: string, content: string, cursor?: number | null) =>
  ipc<void>("session_flush", { docKey, content, cursor });
export const sessionList = () => ipc<SessionInfo[]>("session_list");
export const sessionRestore = (docKey: string) => ipc<SessionDraft>("session_restore", { docKey });
export const sessionDiscard = (docKey: string) => ipc<void>("session_discard", { docKey });

export const indexFile = (relPath: string) => ipc<void>("index_file", { relPath });
export const indexReindex = () => ipc<number>("index_reindex");

export const searchQuery = (q: string) => ipc<SearchHit[]>("search_query", { q });

export const attachPaste = (dataUrl: string, origName?: string) =>
  ipc<{ rel_path: string }>("attach_paste", { dataUrl, origName });
export const attachRead = (relPath: string) => ipc<AttachmentData>("attach_read", { relPath });

export const renderMarkdown = (content: string) => ipc<string>("render_markdown", { content });

export const exportDocument = (relPath: string, format: ExportFormat, destDir?: string) =>
  ipc<string>("export_document", { relPath, format, destDir });

export const windowCreate = (relPath?: string, vaultPath?: string) =>
  ipc<void>("window_create", { relPath, vaultPath });

export const snapshotCreate = (relPath: string) => ipc<number>("snapshot_create", { relPath });
export const snapshotRestore = (relPath: string, snapshotAt: number) =>
  ipc<string>("snapshot_restore", { relPath, snapshotAt });
export const snapshotList = (relPath: string) => ipc<number[]>("snapshot_list", { relPath });

export const configLoad = () => ipc<AppSettings>("config_load");
export const configSave = (settings: AppSettings) => ipc<void>("config_save", { settings });
