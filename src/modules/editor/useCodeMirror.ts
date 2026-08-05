import { useEffect, useRef, type RefObject } from "react";
import { EditorState } from "@codemirror/state";
import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLine,
  placeholder as cmPlaceholder,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";

/** Quiet look: no gutters, no active-line tint, theme-following colors. */
const ruachTheme = EditorView.theme({
  "&": {
    backgroundColor: "var(--bg)",
    color: "var(--text)",
    fontSize: "15px",
    height: "100%",
  },
  ".cm-scroller": {
    fontFamily: "var(--font-preset)",
    lineHeight: "var(--editor-lh)",
  },
  ".cm-content": {
    padding: "24px",
    maxWidth: "var(--page-w)",
    margin: "0 auto",
    caretColor: "var(--accent)",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-line": { padding: "0 4px" },
});

/**
 * Bind a CodeMirror view to a container. External `content` swaps in only
 * when it differs from what the view holds (doc switching, not typing).
 * Returns a getter so callers can dispatch into the view (e.g. paste).
 */
export function useCodeMirror(
  container: RefObject<HTMLElement | null>,
  content: string,
  onDocChange: (content: string, cursor: number) => void,
): { getView: () => EditorView | null } {
  const viewRef = useRef<EditorView | null>(null);
  const contentRef = useRef(content);
  const onDocChangeRef = useRef(onDocChange);
  onDocChangeRef.current = onDocChange;

  useEffect(() => {
    if (!container.current) return;
    const view = new EditorView({
      state: EditorState.create({
        doc: contentRef.current,
        extensions: [
          history(),
          keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
          markdown(),
          ruachTheme,
          lineNumbers({ formatNumber: () => "" }), // quiet ruler, no numbers
          highlightActiveLine(),
          cmPlaceholder("打开或新建一篇文档…"),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              contentRef.current = update.state.doc.toString();
              onDocChangeRef.current(
                contentRef.current,
                update.state.selection.main.head,
              );
            }
          }),
        ],
      }),
      parent: container.current,
    });
    viewRef.current = view;
    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // Mount once; content sync happens in the effect below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [container]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    if (content !== contentRef.current) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: content },
      });
      contentRef.current = content;
    }
  }, [content]);

  return { getView: () => viewRef.current };
}
