import { create } from "zustand";
import type { FontPreset, ThemeKind } from "../lib/types";

interface ThemeState {
  theme: ThemeKind;
  fontPreset: FontPreset;
  lineHeight: number;
  pageWidth: number;
  setTheme: (theme: ThemeKind) => void;
  setFontPreset: (preset: FontPreset) => void;
  /** Persist current settings to ConfigService. */
  persist: () => Promise<void>;
}

/** Font presets resolved to CSS stacks (defined in theme/tokens.css). */
export const FONT_STACK: Record<FontPreset, string> = {
  serif: "var(--font-serif)",
  sans_serif: "var(--font-sans)",
};

export const useThemeStore = create<ThemeState>((set, get) => ({
  theme: "warm_paper",
  fontPreset: "serif",
  lineHeight: 1.8,
  pageWidth: 720,
  setTheme: (theme) => set({ theme }),
  setFontPreset: (fontPreset) => set({ fontPreset }),
  persist: async () => {
    const { theme, fontPreset, lineHeight, pageWidth } = get();
    const { configSave } = await import("../lib/ipc");
    await configSave({ theme, font_preset: fontPreset, line_height: lineHeight, page_width: pageWidth, show_file_tree: true });
  },
}));
