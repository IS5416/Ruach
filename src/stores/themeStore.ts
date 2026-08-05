import { create } from "zustand";
import type { AppSettings, FontPreset, ThemeKind } from "../lib/types";

interface ThemeState {
  theme: ThemeKind;
  fontPreset: FontPreset;
  lineHeight: number;
  pageWidth: number;
  setTheme: (theme: ThemeKind) => void;
  setFontPreset: (preset: FontPreset) => void;
  setLineHeight: (lineHeight: number) => void;
  setPageWidth: (pageWidth: number) => void;
  /** Overwrite all fields from persisted settings (app boot). */
  hydrate: (settings: AppSettings) => void;
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
  setLineHeight: (lineHeight) => set({ lineHeight }),
  setPageWidth: (pageWidth) => set({ pageWidth }),
  hydrate: (s) =>
    set({
      theme: s.theme,
      fontPreset: s.font_preset,
      lineHeight: s.line_height,
      pageWidth: s.page_width,
    }),
  persist: async () => {
    const { theme, fontPreset, lineHeight, pageWidth } = get();
    const { configSave } = await import("../lib/ipc");
    const { useUiStore } = await import("./uiStore");
    await configSave({
      theme,
      font_preset: fontPreset,
      line_height: lineHeight,
      page_width: pageWidth,
      show_file_tree: useUiStore.getState().treeVisible,
    });
  },
}));
