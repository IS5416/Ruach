import { useEffect, useRef } from "react";
import { useCodeMirror } from "./useCodeMirror";
import { useDocStore } from "../../stores/docStore";
import { useEditorStore } from "../../stores/editorStore";
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
  const setCursor = useEditorStore((s) => s.setCursor);

  const { getView } = useCodeMirror(containerRef, content, (doc, cursor) => {
    setCursor(cursor);
    setContent(doc);
  });

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

  return <div ref={containerRef} className="editor" aria-label="编辑器" />;
}
