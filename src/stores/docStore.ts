import { create } from "zustand";
import type { DocumentMeta } from "../lib/types";
import { RuachError } from "../lib/error";

export const UNTITLED_PREFIX = ":untitled:";

interface DocState {
  relPath: string | null;
  meta: DocumentMeta | null;
  content: string;
  /** Session key for untitled drafts (`:untitled:<ts>`); null for file docs. */
  draftKey: string | null;
  /** True while an autosave is in flight or pending. */
  dirty: boolean;
  error: RuachError | null;
  openDoc: (relPath: string, opts?: { discardChanges?: boolean }) => Promise<void>;
  /** New untitled draft — lives in the recovery buffer, not on disk. */
  newDraft: () => void;
  /** Restore a recovered session into the editor. */
  restoreSession: (docKey: string, content: string) => Promise<void>;
  setContent: (content: string) => void;
  /**
   * Save: file docs go to disk (mtime-conflict checked on the Rust side),
   * untitled drafts flush to the recovery buffer. Caller debounces.
   * `force` skips the mtime check (used by the conflict banner's
   * "overwrite" action). Resolves true on success — openDoc aborts its
   * switch if the flush of pending edits fails, so the user's content is
   * never dropped.
   */
  save: (force?: boolean) => Promise<boolean>;
}

export const useDocStore = create<DocState>((set, get) => ({
  relPath: null,
  meta: null,
  content: "",
  draftKey: null,
  dirty: false,
  error: null,
  openDoc: async (relPath, opts) => {
    // Flush pending edits before switching — the debounced autosave may be
    // in flight, and a failed save (e.g. FileChanged) keeps the current
    // document open with its content intact. discardChanges skips the flush
    // for the conflict banner's "reload" action.
    if (!opts?.discardChanges && get().dirty && !(await get().save())) return;
    set({ error: null });
    try {
      const { docOpen } = await import("../lib/ipc");
      const res = await docOpen(relPath);
      // User typed while the open flew — keep their content instead of
      // clobbering it with the freshly-read disk state.
      if (get().dirty) return;
      set({
        relPath,
        meta: res.meta,
        content: res.content,
        draftKey: null,
        dirty: false,
      });
    } catch (e) {
      set({ error: RuachError.fromUnknown(e) });
    }
  },
  newDraft: () =>
    set({
      relPath: null,
      meta: null,
      content: "",
      draftKey: `${UNTITLED_PREFIX}${Date.now()}`,
      dirty: false,
      error: null,
    }),
  restoreSession: async (docKey, content) => {
    let meta: DocumentMeta | null = null;
    // File docs need the disk mtime as the conflict-detection baseline —
    // otherwise the restored content could blind-write over a newer file.
    if (!docKey.startsWith(UNTITLED_PREFIX)) {
      try {
        const { docOpen } = await import("../lib/ipc");
        const res = await docOpen(docKey);
        meta = res.meta;
      } catch {
        // Doc vanished from disk; keep meta null (first save creates it).
      }
    }
    set({
      relPath: docKey.startsWith(UNTITLED_PREFIX) ? null : docKey,
      meta,
      content,
      draftKey: docKey,
      // Dirty on purpose: the recovered content must land on disk (or the
      // recovery buffer) once more instead of lingering in memory.
      dirty: true,
      error: null,
    });
  },
  setContent: (content) => {
    // No-op when unchanged: external swaps (openDoc, doc:changed reload)
    // must not re-mark the doc dirty — that would trigger spurious saves
    // and a save→reload→save ping-pong between windows.
    if (content === get().content) return;
    set({ content, dirty: true });
  },
  save: async (force) => {
    const { relPath, draftKey, content, dirty, meta } = get();
    if (!dirty) return true;
    try {
      const { docSave, sessionFlush } = await import("../lib/ipc");
      if (relPath) {
        // Baseline = mtime we last saw; Rust rejects if the disk moved on.
        // force (conflict banner) passes null so the check is skipped.
        const mtime = await docSave(relPath, content, force ? null : meta?.mtime);
        set((s) => ({
          // Only clear dirty if nothing was typed while the save flew —
          // otherwise the newer content is left dirty and the debounce
          // re-fires instead of silently losing it.
          dirty: s.content !== content,
          meta: s.meta ? { ...s.meta, mtime } : null,
        }));
      } else if (draftKey) {
        await sessionFlush(draftKey, content);
        set((s) => ({ dirty: s.content !== content }));
      }
      return true;
    } catch (e) {
      set({ error: RuachError.fromUnknown(e) });
      return false;
    }
  },
}));
