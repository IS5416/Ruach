import { useEffect, useRef } from "react";
import { useCodeMirror } from "./useCodeMirror";
import { useDocStore } from "../../stores/docStore";
import type { EditorView } from "@codemirror/view";

const AUTOSAVE_MS = 1500;

/** Read a clipboard File as a base64 data URL. */
function fileToDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

/** Paste images: save via attach_paste, insert `![](attachments/...)`. */
async function handleImagePaste(e: ClipboardEvent, view: EditorView) {
  const files = e.clipboardData?.files;
  if (!files || files.length === 0) return false;
  const image = [...files].find((f) => f.type.startsWith("image/"));
  if (!image) return false;

  e.preventDefault();
  try {
    const { attachPaste } = await import("../../lib/ipc");
    const dataUrl = await fileToDataUrl(image);
    const { rel_path } = await attachPaste(dataUrl, image.name);
    const insert = `![](${rel_path})`;
    view.dispatch({
      changes: { from: view.state.selection.main.from, insert },
      selection: { anchor: view.state.selection.main.from + insert.length },
    });
  } catch {
    // Leave the paste event swallowed; error surface is the editor state.
  }
  return true;
}

/**
 * Source-mode editor (CodeMirror 6). Typing marks the doc dirty; a 1.5s
 * debounce triggers docStore.save (disk save or draft flush). Pasting an
 * image saves it into the vault attachments dir and inserts a reference.
 */
export function EditorPane() {
  const containerRef = useRef<HTMLDivElement>(null);
  const content = useDocStore((s) => s.content);
  const setContent = useDocStore((s) => s.setContent);
  const save = useDocStore((s) => s.save);
  const dirty = useDocStore((s) => s.dirty);
  const hasDoc = useDocStore((s) => s.relPath !== null || s.draftKey !== null);

  const { getView } = useCodeMirror(containerRef, content, setContent);

  // Paste handler: intercept images before CodeMirror's default paste.
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const onPaste = (e: ClipboardEvent) => {
      const view = getView();
      if (view && e.clipboardData?.files?.length) {
        void handleImagePaste(e, view);
      }
    };
    el.addEventListener("paste", onPaste);
    return () => el.removeEventListener("paste", onPaste);
  }, [getView]);

  // Debounced autosave: only when dirty, 1.5s after the last change.
  useEffect(() => {
    if (!dirty) return;
    const timer = setTimeout(() => void save(), AUTOSAVE_MS);
    return () => clearTimeout(timer);
  }, [dirty, content, save]);

  // The container div must exist from the first render onward: useCodeMirror
  // creates the view once on mount, and an empty state rendered *instead* of
  // the container would leave the view never created (no editor content
  // after opening a doc). The empty state is an overlay on top instead.
  return (
    <div className="editor-wrap">
      <div ref={containerRef} className="editor" aria-label="编辑器" />
      {!hasDoc && (
        <div className="empty-state empty-state--overlay">
          <p className="empty-state__title">向虚空呼入气息</p>
          <p className="empty-state__hint">从树中选择一篇文档，或新建一篇草稿</p>
        </div>
      )}
    </div>
  );
}
