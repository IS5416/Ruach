import { useEffect, useLayoutEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Button } from "./components/Button";
import { EditorPane } from "./modules/editor";
import { PreviewPane } from "./modules/preview";
import { FileTree } from "./modules/tree";
import { CommandPalette } from "./modules/command";
import { TabStrip } from "./modules/tabs";
import { SettingsPanel } from "./modules/settings";
import { RecoveryBanner } from "./app/RecoveryBanner";
import { useUiStore } from "./stores/uiStore";
import { FONT_STACK, useThemeStore } from "./stores/themeStore";
import { useDocStore } from "./stores/docStore";
import { useVaultStore } from "./stores/vaultStore";
import type { LayoutMode } from "./lib/types";
import "./App.css";

const MODE_LABEL: Record<LayoutMode, string> = {
  edit: "编辑",
  preview: "预览",
  split: "分屏",
  immersion: "沉浸",
};

function App() {
  const layoutMode = useUiStore((s) => s.layoutMode);
  const setLayoutMode = useUiStore((s) => s.setLayoutMode);
  const treeVisible = useUiStore((s) => s.treeVisible);
  const toggleTree = useUiStore((s) => s.toggleTree);
  const theme = useThemeStore((s) => s.theme);
  const hydrate = useThemeStore((s) => s.hydrate);
  const fontPreset = useThemeStore((s) => s.fontPreset);
  const lineHeight = useThemeStore((s) => s.lineHeight);
  const pageWidth = useThemeStore((s) => s.pageWidth);
  const docTitle = useDocStore((s) => s.meta?.title);
  const docError = useDocStore((s) => s.error);
  const newDraft = useDocStore((s) => s.newDraft);
  const status = useUiStore((s) => s.status);
  const setStatus = useUiStore((s) => s.setStatus);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Transient status messages auto-hide after 3s.
  useEffect(() => {
    if (!status) return;
    const timer = setTimeout(() => setStatus(null), 3000);
    return () => clearTimeout(timer);
  }, [status, setStatus]);

  // Load persisted settings once on mount.
  useEffect(() => {
    import("./lib/ipc")
      .then(({ configLoad }) => configLoad())
      .then((settings) => {
        hydrate(settings);
        useUiStore.setState({ treeVisible: settings.show_file_tree });
      })
      .catch(() => {});
  }, [hydrate]);

  // Apply typography as CSS variables (editor + preview both consume them).
  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--font-preset", FONT_STACK[fontPreset]);
    root.style.setProperty("--editor-lh", String(lineHeight));
    root.style.setProperty("--page-w", `${pageWidth}px`);
  }, [fontPreset, lineHeight, pageWidth]);

  // Themes live on `:root[data-theme=...]` in tokens.css — carry the
  // attribute on the root element so the palette actually applies.
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  // Last-resort flush on window close: the 1.5s debounce may still be
  // pending, so push the dirty doc into the recovery buffer. Best-effort —
  // the invoke is sent before the webview tears down.
  useEffect(() => {
    const onUnload = () => {
      const { relPath, draftKey, content, dirty } = useDocStore.getState();
      if (!dirty || (!relPath && !draftKey)) return;
      const docKey = draftKey ?? relPath!;
      void import("./lib/ipc").then(({ sessionFlush }) =>
        sessionFlush(docKey, content),
      );
    };
    window.addEventListener("beforeunload", onUnload);
    return () => window.removeEventListener("beforeunload", onUnload);
  }, []);

  // Ctrl+E: toggle edit/immersion. Ctrl+P: command palette.
  // Ctrl+Shift+N: open a new window with the current doc.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const key = e.key.toLowerCase();
      if (e.ctrlKey && key === "e") {
        e.preventDefault();
        const current = useUiStore.getState().layoutMode;
        setLayoutMode(current === "immersion" ? "edit" : "immersion");
      } else if (e.ctrlKey && key === "p") {
        e.preventDefault();
        useUiStore.getState().setCommandPaletteOpen(true);
      } else if (e.ctrlKey && e.shiftKey && key === "n") {
        e.preventDefault();
        const relPath = useDocStore.getState().relPath;
        const vaultPath = useVaultStore.getState().vaultPath;
        void import("./lib/ipc").then(({ windowCreate }) =>
          windowCreate(relPath ?? undefined, vaultPath ?? undefined),
        );
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setLayoutMode]);

  // New windows carry ?vault=<root>&doc=<rel_path> in their URL — open the
  // vault first (so the tree/editor render), then the doc.
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const doc = params.get("doc");
    const vault = params.get("vault");
    if (vault) {
      void useVaultStore
        .getState()
        .openVault(vault)
        .then(() => {
          if (doc) void useDocStore.getState().openDoc(doc);
        })
        .catch(() => {});
    } else if (doc) {
      void useDocStore.getState().openDoc(doc);
    }
  }, []);

  // Cross-window sync: reload the doc when another window saved it.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    void listen<string>("doc:changed", (event) => {
      const { relPath, dirty } = useDocStore.getState();
      if (event.payload === relPath && !dirty) {
        void useDocStore.getState().openDoc(relPath);
      }
    }).then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const openVault = async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const dir = await open({ directory: true });
    if (typeof dir === "string") {
      await useVaultStore.getState().openVault(dir);
    }
  };

  const vaultPath = useVaultStore((s) => s.vaultPath);
  const noVault = !vaultPath;

  const immersion = layoutMode === "immersion";

  return (
    <div
      className={`app ${immersion ? "app--immersion" : ""}`}
      data-theme={theme}
    >
      {!immersion && (
        <header className="topbar">
          <span className={`topbar__title${docTitle ? "" : " topbar__title--empty"}`}>
            {docTitle ?? "Ruach"}
          </span>
          <div className="segmented" role="group" aria-label="布局模式">
            {(Object.keys(MODE_LABEL) as LayoutMode[]).map((mode) => (
              <button
                key={mode}
                className={`segmented__btn${layoutMode === mode ? " segmented__btn--active" : ""}`}
                onClick={() => setLayoutMode(mode)}
              >
                {MODE_LABEL[mode]}
              </button>
            ))}
          </div>
          <div className="topbar__actions">
            <Button onClick={openVault}>打开 Vault</Button>
            <Button onClick={newDraft}>新建</Button>
            <Button onClick={toggleTree}>{treeVisible ? "藏树" : "树"}</Button>
            <Button active={settingsOpen} onClick={() => setSettingsOpen((v) => !v)}>
              设置
            </Button>
          </div>
        </header>
      )}

      <RecoveryBanner />

      {status && <div className="status-banner">{status}</div>}

      {immersion && (
        <button className="immersion-exit" onClick={() => setLayoutMode("edit")} title="退出沉浸（Ctrl+E）">
          退出沉浸
        </button>
      )}

      <div className="body">
        {treeVisible && !immersion && <FileTree />}

        <main className="main">
          {!immersion && <TabStrip />}
          {docError && (
            <div className="error-banner" role="alert">
              <span className="error-banner__text">{docError.message}</span>
              {docError.code === "file_changed" && (
                <div className="error-banner__actions">
                  <Button
                    onClick={() => {
                      const relPath = useDocStore.getState().relPath;
                      if (relPath) {
                        void useDocStore.getState().openDoc(relPath, { discardChanges: true });
                      }
                    }}
                  >
                    重新加载
                  </Button>
                  <Button
                    onClick={() => {
                      void useDocStore.getState().save(true);
                    }}
                  >
                    强制覆盖
                  </Button>
                </div>
              )}
            </div>
          )}
          {settingsOpen && !immersion ? (
            <SettingsPanel />
          ) : noVault ? (
            <div className="empty-state">
              <p className="empty-state__title">打开一个 Vault 开始</p>
              <p className="empty-state__hint">选择一个存放 Markdown 文档的文件夹</p>
              <Button variant="primary" onClick={openVault}>
                打开 Vault
              </Button>
            </div>
          ) : (
            <div className="pane-stack">
              {layoutMode !== "preview" && <EditorPane />}
              {(layoutMode === "preview" || layoutMode === "split") && <PreviewPane />}
            </div>
          )}
        </main>
      </div>

      <CommandPalette />
    </div>
  );
}

export default App;
