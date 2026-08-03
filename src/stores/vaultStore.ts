import { create } from "zustand";
import type { TreeNode } from "../lib/types";
import { RuachError } from "../lib/error";

interface VaultState {
  vaultPath: string | null;
  tree: TreeNode[];
  loading: boolean;
  error: RuachError | null;
  openVault: (path: string) => Promise<void>;
  refresh: () => Promise<void>;
}

export const useVaultStore = create<VaultState>((set, get) => ({
  vaultPath: null,
  tree: [],
  loading: false,
  error: null,
  openVault: async (path) => {
    set({ loading: true, error: null });
    try {
      const { vaultOpen } = await import("../lib/ipc");
      await vaultOpen(path);
      set({ vaultPath: path });
      await get().refresh();
    } catch (e) {
      set({ error: RuachError.fromUnknown(e) });
    } finally {
      set({ loading: false });
    }
  },
  refresh: async () => {
    try {
      const { vaultScan } = await import("../lib/ipc");
      const tree = await vaultScan();
      set({ tree, error: null });
    } catch (e) {
      set({ error: RuachError.fromUnknown(e) });
    }
  },
}));
