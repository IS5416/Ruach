import { useDocStore } from "../../stores/docStore";

/** Tab strip placeholder. Tabs ↔ window sessions map in P6. */
export function TabStrip() {
  const relPath = useDocStore((s) => s.relPath);
  const title = useDocStore((s) => s.meta?.title);
  if (!relPath) return null;
  // Plain spans for now — real tabs (focusable, keyboard-switchable) land
  // with the P6 multi-tab work; faking the tab role here would mislead
  // screen-reader users into expecting interaction.
  return (
    <div className="tabstrip">
      <span className="tab">{title ?? relPath}</span>
    </div>
  );
}
