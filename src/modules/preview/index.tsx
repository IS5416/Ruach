/**
 * Preview pane. P3 wires sandboxed iframe + render_markdown IPC;
 * this placeholder shows the raw source until then.
 */
export function PreviewPane() {
  return (
    <div className="preview" aria-label="预览">
      <p className="preview__placeholder">预览尚未接入 — P3（comrak 渲染管线）</p>
    </div>
  );
}
