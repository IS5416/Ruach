import { useEffect, useMemo, useRef, useState } from "react";
import { useDocStore } from "../../stores/docStore";
import { useThemeStore } from "../../stores/themeStore";
import { useUiStore } from "../../stores/uiStore";
import { useVaultStore } from "../../stores/vaultStore";

const RENDER_DEBOUNCE_MS = 300;

/**
 * Vault attachments are referenced by relative path (`attachments/x.png`).
 * The sandboxed iframe cannot load them directly, so each is read through
 * attach_read and inlined as a data URL.
 *
 * Attachments are immutable on disk, so data URLs are cached per
 * (vault, rel_path) — re-reading them on every keystroke would push MBs
 * over IPC in split mode.
 */
const attachCache = new Map<string, Promise<string>>();
let attachCacheVault: string | null = null;

async function inlineAttachments(html: string, vaultPath: string | null): Promise<string> {
  const refs = [...html.matchAll(/src="(attachments\/[^"]+)"/g)].map((m) => m[1]);
  if (refs.length === 0) return html;
  const unique = [...new Set(refs)];
  // Cache is per-vault: the same rel_path may exist in two vaults.
  if (attachCacheVault !== vaultPath) {
    attachCache.clear();
    attachCacheVault = vaultPath;
  }
  const resolved = new Map<string, string>();
  const { attachRead } = await import("../../lib/ipc");
  await Promise.all(
    unique.map(async (p) => {
      const key = `${vaultPath ?? ""}:${p}`;
      let entry = attachCache.get(key);
      if (!entry) {
        entry = (async () => {
          try {
            const d = await attachRead(p);
            return `data:${d.mime};base64,${d.base64}`;
          } catch {
            // Broken reference — cache the miss so we don't retry on
            // every keystroke; the image stays broken.
            return "";
          }
        })();
        attachCache.set(key, entry);
      }
      resolved.set(p, await entry);
    }),
  );
  return html.replace(/src="(attachments\/[^"]+)"/g, (_, p: string) => {
    return `src="${resolved.get(p) ?? ""}"`;
  });
}

/** Resolve current theme tokens into CSS for the iframe document. */
function themeCss(): string {
  const s = getComputedStyle(document.documentElement);
  const get = (name: string) => s.getPropertyValue(name).trim();
  return `:root {
  --bg: ${get("--bg")};
  --bg-panel: ${get("--bg-panel")};
  --bg-raised: ${get("--bg-raised")};
  --ink-strong: ${get("--ink-strong")};
  --ink: ${get("--ink")};
  --ink-soft: ${get("--ink-soft")};
  --ink-faint: ${get("--ink-faint")};
  --line: ${get("--line")};
  --accent: ${get("--accent")};
  --font-serif: ${get("--font-serif")};
  --font-sans: ${get("--font-sans")};
  --font-preset: ${get("--font-preset") || get("--font-serif")};
  --editor-lh: ${get("--editor-lh") || "1.8"};
  --page-w: ${get("--page-w") || "720px"};
}`;
}

/** Typography for rendered markdown, driven by the same tokens. */
const PREVIEW_CSS = `
body {
  margin: 0;
  background: var(--bg);
  color: var(--ink);
  font-family: var(--font-preset);
  font-size: 16px;
  line-height: var(--editor-lh);
  letter-spacing: 0.02em;
  -webkit-font-smoothing: antialiased;
}
.markdown-body { max-width: var(--page-w); margin: 0 auto; padding: 56px 28px 30vh; }
h1, h2, h3 { font-weight: 600; line-height: 1.4; margin: 1.6em 0 0.6em; }
h1 { font-size: 1.7em; letter-spacing: 0.04em; }
h2 { font-size: 1.35em; border-bottom: 1px solid var(--line); padding-bottom: 0.25em; }
p, ul, ol, blockquote, pre, table { margin: 0.9em 0; }
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
blockquote {
  margin-left: 0;
  padding-left: 1.2em;
  border-left: 2px solid var(--line);
  color: var(--ink-soft);
}
code {
  font-family: Consolas, "Cascadia Code", monospace;
  font-size: 0.88em;
  background: var(--bg-panel);
  padding: 0.15em 0.4em;
  border-radius: 3px;
}
pre {
  background: var(--bg-panel);
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 1em;
  overflow-x: auto;
}
pre code { background: transparent; padding: 0; }
table { border-collapse: collapse; font-size: 0.92em; }
th, td { border: 1px solid var(--line); padding: 0.4em 0.8em; }
th { background: var(--bg-panel); }
hr { border: none; border-top: 1px solid var(--line); margin: 2em 0; }
input[type="checkbox"] { margin-right: 0.4em; }
del { color: var(--ink-faint); }
sup a.footnote-ref { font-size: 0.7em; }
.footnote-definition { color: var(--ink-soft); font-size: 0.85em; }
img { max-width: 100%; }
`;

function buildSrcDoc(body: string, theme: string): string {
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>${theme}\n${PREVIEW_CSS}</style>
</head>
<body>
<div class="markdown-body">${body}</div>
</body>
</html>`;
}

/**
 * Preview pane: sandboxed iframe (no scripts) fed by the Rust render
 * pipeline. Rendering is debounced; theme colors are inherited from the
 * parent document's computed tokens.
 */
export function PreviewPane() {
  const content = useDocStore((s) => s.content);
  const relPath = useDocStore((s) => s.relPath);
  const theme = useThemeStore((s) => s.theme);
  const fontPreset = useThemeStore((s) => s.fontPreset);
  const lineHeight = useThemeStore((s) => s.lineHeight);
  const pageWidth = useThemeStore((s) => s.pageWidth);
  const layoutMode = useUiStore((s) => s.layoutMode);
  const vaultPath = useVaultStore((s) => s.vaultPath);
  const [body, setBody] = useState("");
  const [rendering, setRendering] = useState(false);
  // Monotonic render id — a late response from an older render must not
  // clobber a newer one (fast typing, slow attachment reads).
  const renderSeq = useRef(0);

  useEffect(() => {
    if (!relPath) {
      setBody("");
      return;
    }
    const seq = ++renderSeq.current;
    setRendering(true);
    const timer = setTimeout(() => {
      import("../../lib/ipc")
        .then(({ renderMarkdown }) => renderMarkdown(content))
        .then((html) => inlineAttachments(html, vaultPath))
        .then((b) => {
          if (renderSeq.current === seq) setBody(b);
        })
        .catch(() => {
          if (renderSeq.current === seq) setBody("<p>渲染失败</p>");
        })
        .finally(() => {
          if (renderSeq.current === seq) setRendering(false);
        });
    }, RENDER_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [content, relPath, vaultPath]);

  // themeCss() reads the root CSS variables set from the typography
  // settings — subscribe to them so the iframe style refreshes when they
  // change, not only on the next body change.
  const srcDoc = useMemo(() => buildSrcDoc(body, themeCss()), [body, theme, fontPreset, lineHeight, pageWidth]);

  if (!relPath) {
    return (
      <div className={`preview${layoutMode === "split" ? " preview--split" : ""}`} aria-label="预览">
        <p className="preview__placeholder">打开一篇文档查看预览</p>
      </div>
    );
  }

  return (
    <div className={`preview${layoutMode === "split" ? " preview--split" : ""}`} aria-label="预览">
      {rendering && <span className="preview__spinner" aria-hidden="true" />}
      <iframe
        className="preview__frame"
        sandbox=""
        srcDoc={srcDoc}
        title="预览"
      />
    </div>
  );
}
