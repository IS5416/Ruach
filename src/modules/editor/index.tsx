import { useEffect, useRef } from "react";
import { useCodeMirror } from "./useCodeMirror";
import { useDocStore } from "../../stores/docStore";
import { useEditorStore } from "../../stores/editorStore";

const AUTOSAVE_MS = 1500;

/**
 * Source-mode editor (CodeMirror 6). Typing marks the doc dirty; a 1.5s
 * debounce triggers docStore.save (disk save or draft flush).
 */
export function EditorPane() {
  const containerRef = useRef<HTMLDivElement>(null);
  const content = useDocStore((s) => s.content);
  const setContent = useDocStore((s) => s.setContent);
  const save = useDocStore((s) => s.save);
  const dirty = useDocStore((s) => s.dirty);
  const setCursor = useEditorStore((s) => s.setCursor);

  useCodeMirror(containerRef, content, (doc, cursor) => {
    setCursor(cursor);
    setContent(doc);
  });

  // Debounced autosave: only when dirty, 1.5s after the last change.
  useEffect(() => {
    if (!dirty) return;
    const timer = setTimeout(() => void save(), AUTOSAVE_MS);
    return () => clearTimeout(timer);
  }, [dirty, content, save]);

  return <div ref={containerRef} className="editor" aria-label="编辑器" />;
}
