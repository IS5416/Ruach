import { create } from "zustand";
import type { AppSettings, FontPreset, ThemeKind } from "../lib/types";

interface ThemeState {
  theme: ThemeKind;
  fontPreset: FontPreset;
  lineHeight: number;
  pageWidth: number;
  /** True once the user changed anything — hydrate must not clobber it. */
  touched: boolean;
  setTheme: (theme: ThemeKind) => void;
  setFontPreset: (preset: FontPreset) => void;
  setLineHeight: (lineHeight: number) => void;
  setPageWidth: (pageWidth: number) => void;
  /** Apply persisted settings (app boot); skips fields the user already changed. */
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
  touched: false,
  setTheme: (theme) => set({ theme, touched: true }),
  setFontPreset: (fontPreset) => set({ fontPreset, touched: true }),
  setLineHeight: (lineHeight) => set({ lineHeight, touched: true }),
  setPageWidth: (pageWidth) => set({ pageWidth, touched: true }),
  hydrate: (s) =>
    set((cur) => ({
      // The async configLoad may resolve after the user already changed a
      // setting — never overwrite what they touched.
      theme: cur.touched ? cur.theme : s.theme,
      fontPreset: cur.touched ? cur.fontPreset : s.font_preset,
      lineHeight: cur.touched ? cur.lineHeight : s.line_height,
      pageWidth: cur.touched ? cur.pageWidth : s.page_width,
    })),
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
