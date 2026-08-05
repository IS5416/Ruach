import { create } from "zustand";
import type { LayoutMode } from "../lib/types";

interface UiState {
  layoutMode: LayoutMode;
  treeVisible: boolean;
  commandPaletteOpen: boolean;
  /** Transient status message (exports, etc.), auto-hides in App. */
  status: string | null;
  setLayoutMode: (mode: LayoutMode) => void;
  toggleTree: () => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setStatus: (status: string | null) => void;
}

/**
 * Four layout states ("breathing rhythms"): edit / preview / split / immersion.
 * Immersion hides chrome and centers a narrow column.
 */
export const useUiStore = create<UiState>((set) => ({
  layoutMode: "edit",
  treeVisible: true,
  commandPaletteOpen: false,
  status: null,
  setLayoutMode: (layoutMode) => set({ layoutMode }),
  toggleTree: () => set((s) => ({ treeVisible: !s.treeVisible })),
  setCommandPaletteOpen: (commandPaletteOpen) => set({ commandPaletteOpen }),
  setStatus: (status) => set({ status }),
}));
