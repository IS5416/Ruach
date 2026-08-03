import { useUiStore } from "../../stores/uiStore";

/**
 * Command palette (Ctrl+P). Fuzzy search + command registry land in P5;
 * this placeholder is a bare input overlay.
 */
export function CommandPalette() {
  const open = useUiStore((s) => s.commandPaletteOpen);
  const setOpen = useUiStore((s) => s.setCommandPaletteOpen);

  if (!open) return null;
  return (
    <div className="palette-overlay" onClick={() => setOpen(false)}>
      <div className="palette" onClick={(e) => e.stopPropagation()}>
        <input
          autoFocus
          className="palette__input"
          placeholder="搜索文档…（P5 接入）"
          aria-label="命令面板"
        />
      </div>
    </div>
  );
}
