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
  openDoc: (relPath: string) => Promise<void>;
  /** New untitled draft — lives in the recovery buffer, not on disk. */
  newDraft: () => void;
  /** Restore a recovered session into the editor. */
  restoreSession: (docKey: string, content: string) => void;
  setContent: (content: string) => void;
  /**
   * Save: file docs go to disk (mtime-conflict checked on the Rust side),
   * untitled drafts flush to the recovery buffer. Caller debounces.
   */
  save: () => Promise<void>;
}

export const useDocStore = create<DocState>((set, get) => ({
  relPath: null,
  meta: null,
  content: "",
  draftKey: null,
  dirty: false,
  error: null,
  openDoc: async (relPath) => {
    set({ error: null });
    try {
      const { docOpen } = await import("../lib/ipc");
      const res = await docOpen(relPath);
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
  restoreSession: (docKey, content) =>
    set({
      relPath: docKey.startsWith(UNTITLED_PREFIX) ? null : docKey,
      meta: null,
      content,
      draftKey: docKey,
      dirty: false,
      error: null,
    }),
  setContent: (content) => set({ content, dirty: true }),
  save: async () => {
    const { relPath, draftKey, content, dirty, meta } = get();
    if (!dirty) return;
    try {
      const { docSave, sessionFlush } = await import("../lib/ipc");
      if (relPath) {
        // Baseline = mtime we last saw; Rust rejects if the disk moved on.
        const mtime = await docSave(relPath, content, meta?.mtime);
        set((s) => ({ dirty: false, meta: s.meta ? { ...s.meta, mtime } : null }));
      } else if (draftKey) {
        await sessionFlush(draftKey, content);
        set({ dirty: false });
      }
    } catch (e) {
      set({ error: RuachError.fromUnknown(e) });
    }
  },
}));
