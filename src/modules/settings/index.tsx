import { useThemeStore } from "../../stores/themeStore";
import { useUiStore } from "../../stores/uiStore";
import type { FontPreset, ThemeKind } from "../../lib/types";

const THEMES: { value: ThemeKind; label: string }[] = [
  { value: "warm_paper", label: "暖纸" },
  { value: "cold_stone", label: "冷石" },
  { value: "night_ink", label: "墨夜" },
];

const FONTS: { value: FontPreset; label: string }[] = [
  { value: "serif", label: "衬线（宋体/Georgia）" },
  { value: "sans_serif", label: "无衬线（黑体/Inter）" },
];

const LINE_HEIGHTS = [1.6, 1.8, 2.0, 2.2];
const PAGE_WIDTHS = [640, 720, 860];

/** Settings panel: theme, typography presets, tree visibility — all
 *  persisted through ConfigService on change. */
export function SettingsPanel() {
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);
  const fontPreset = useThemeStore((s) => s.fontPreset);
  const setFontPreset = useThemeStore((s) => s.setFontPreset);
  const lineHeight = useThemeStore((s) => s.lineHeight);
  const setLineHeight = useThemeStore((s) => s.setLineHeight);
  const pageWidth = useThemeStore((s) => s.pageWidth);
  const setPageWidth = useThemeStore((s) => s.setPageWidth);
  const persist = useThemeStore((s) => s.persist);
  const treeVisible = useUiStore((s) => s.treeVisible);
  const toggleTree = useUiStore((s) => s.toggleTree);

  const change = (apply: () => void) => {
    apply();
    void persist();
  };

  return (
    <section className="settings" aria-label="设置">
      <h2>设置</h2>

      <label className="settings__row">
        主题
        <select value={theme} onChange={(e) => change(() => setTheme(e.currentTarget.value as ThemeKind))}>
          {THEMES.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </select>
      </label>

      <label className="settings__row">
        字体
        <select value={fontPreset} onChange={(e) => change(() => setFontPreset(e.currentTarget.value as FontPreset))}>
          {FONTS.map((f) => (
            <option key={f.value} value={f.value}>
              {f.label}
            </option>
          ))}
        </select>
      </label>

      <label className="settings__row">
        行距
        <select value={lineHeight} onChange={(e) => change(() => setLineHeight(Number(e.currentTarget.value)))}>
          {LINE_HEIGHTS.map((lh) => (
            <option key={lh} value={lh}>
              {lh.toFixed(1)}
            </option>
          ))}
        </select>
      </label>

      <label className="settings__row">
        页宽
        <select value={pageWidth} onChange={(e) => change(() => setPageWidth(Number(e.currentTarget.value)))}>
          {PAGE_WIDTHS.map((w) => (
            <option key={w} value={w}>
              {w}px
            </option>
          ))}
        </select>
      </label>

      <label className="settings__row settings__row--checkbox">
        <input type="checkbox" checked={treeVisible} onChange={() => change(toggleTree)} />
        显示文件树
      </label>
    </section>
  );
}
