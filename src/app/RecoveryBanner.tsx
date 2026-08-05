import { useEffect, useState } from "react";
import type { SessionInfo } from "../lib/types";
import { useDocStore } from "../stores/docStore";
import { Button } from "../components/Button";

/**
 * Crash-recovery banner. On startup, list recovery-buffer entries
 * (dirty docs flushed at close + untitled drafts) and let the user
 * restore or discard each.
 */
export function RecoveryBanner() {
  const [items, setItems] = useState<SessionInfo[]>([]);
  const restoreSession = useDocStore((s) => s.restoreSession);

  useEffect(() => {
    let alive = true;
    import("../lib/ipc")
      .then(({ sessionList }) => sessionList())
      .then((list) => {
        if (alive) setItems(list);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);

  if (items.length === 0) return null;

  const restore = async (docKey: string) => {
    const { sessionRestore, sessionDiscard } = await import("../lib/ipc");
    try {
      const draft = await sessionRestore(docKey);
      await restoreSession(docKey, draft.content);
    } catch {
      // Restore failed (e.g. the row vanished) — keep the banner visible.
      return;
    }
    await sessionDiscard(docKey);
    setItems((prev) => prev.filter((i) => i.doc_key !== docKey));
  };

  const discard = async (docKey: string) => {
    const { sessionDiscard } = await import("../lib/ipc");
    await sessionDiscard(docKey);
    setItems((prev) => prev.filter((i) => i.doc_key !== docKey));
  };

  return (
    <div className="recovery" aria-live="polite">
      <span className="recovery__title">恢复区</span>
      {items.map((item) => (
        <div key={item.doc_key} className="recovery__item">
          <span className="recovery__name">{item.doc_key}</span>
          <span className="recovery__preview">{item.preview || "（空草稿）"}</span>
          <div className="recovery__actions">
            <Button onClick={() => void restore(item.doc_key)}>恢复</Button>
            <Button onClick={() => void discard(item.doc_key)}>丢弃</Button>
          </div>
        </div>
      ))}
    </div>
  );
}
