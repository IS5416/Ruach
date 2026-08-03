import { useDocStore } from "../../stores/docStore";

/** Tab strip placeholder. Tabs ↔ window sessions map in P6. */
export function TabStrip() {
  const relPath = useDocStore((s) => s.relPath);
  const title = useDocStore((s) => s.meta?.title);
  if (!relPath) return null;
  return (
    <div className="tabstrip" role="tablist">
      <span className="tab" role="tab">
        {title ?? relPath}
      </span>
    </div>
  );
}
