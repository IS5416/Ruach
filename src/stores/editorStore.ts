import { create } from "zustand";

interface EditorState {
  /** CodeMirror cursor position, synced for session recovery (P1). */
  cursor: number;
  setCursor: (cursor: number) => void;
}

export const useEditorStore = create<EditorState>((set) => ({
  cursor: 0,
  setCursor: (cursor) => set({ cursor }),
}));
