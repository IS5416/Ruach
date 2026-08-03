import { create } from "zustand";
import type { DocumentMeta } from "../lib/types";
import { RuachError } from "../lib/error";

interface DocState {
  relPath: string | null;
  meta: DocumentMeta | null;
  content: string;
  /** True while an autosave is in flight or pending. */
  dirty: boolean;
  error: RuachError | null;
  openDoc: (relPath: string) => Promise<void>;
  setContent: (content: string) => void;
  /** Debounced autosave (1.5s). Implemented in P1; stores state shape now. */
  save: () => Promise<void>;
}

export const useDocStore = create<DocState>((set, get) => ({
  relPath: null,
  meta: null,
  content: "",
  dirty: false,
  error: null,
  openDoc: async (relPath) => {
    set({ error: null });
    try {
      const { docOpen } = await import("../lib/ipc");
      const res = await docOpen(relPath);
      set({ relPath, meta: res.meta, content: res.content, dirty: false });
    } catch (e) {
      set({ error: RuachError.fromUnknown(e) });
    }
  },
  setContent: (content) => set({ content, dirty: true }),
  save: async () => {
    const { relPath, content, meta, dirty } = get();
    if (!relPath || !dirty) return;
    try {
      const { docSave } = await import("../lib/ipc");
      await docSave(relPath, content, meta?.mtime);
      set({ dirty: false });
    } catch (e) {
      set({ error: RuachError.fromUnknown(e) });
    }
  },
}));
