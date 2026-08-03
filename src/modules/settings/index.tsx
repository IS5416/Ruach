import { useThemeStore } from "../../stores/themeStore";
import type { ThemeKind } from "../../lib/types";

const THEMES: { value: ThemeKind; label: string }[] = [
  { value: "warm_paper", label: "暖纸" },
  { value: "cold_stone", label: "冷石" },
  { value: "night_ink", label: "墨夜" },
];

/** Settings panel placeholder: theme switcher already live. */
export function SettingsPanel() {
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);

  return (
    <section className="settings" aria-label="设置">
      <h2>设置</h2>
      <label className="settings__row">
        主题
        <select
          value={theme}
          onChange={(e) => setTheme(e.currentTarget.value as ThemeKind)}
        >
          {THEMES.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </select>
      </label>
      <p className="settings__hint">排版预设、页宽、窗口状态 — P7 接入 ConfigService</p>
    </section>
  );
}
