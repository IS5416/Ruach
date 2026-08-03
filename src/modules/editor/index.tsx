import { useDocStore } from "../../stores/docStore";

/**
 * Source-mode editor pane. CodeMirror 6 lands in P1; this placeholder
 * renders the document content as a plain textarea.
 */
export function EditorPane() {
  const content = useDocStore((s) => s.content);
  const setContent = useDocStore((s) => s.setContent);
  const relPath = useDocStore((s) => s.relPath);

  return (
    <textarea
      className="editor"
      value={content}
      onChange={(e) => setContent(e.currentTarget.value)}
      placeholder={relPath ? undefined : "打开或新建一篇文档…"}
      spellCheck={false}
      aria-label="编辑器"
    />
  );
}
