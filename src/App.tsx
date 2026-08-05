import { useEffect, useState } from "react";
import { Button } from "./components/Button";
import { EditorPane } from "./modules/editor";
import { PreviewPane } from "./modules/preview";
import { FileTree } from "./modules/tree";
import { CommandPalette } from "./modules/command";
import { TabStrip } from "./modules/tabs";
import { SettingsPanel } from "./modules/settings";
import { RecoveryBanner } from "./app/RecoveryBanner";
import { useUiStore } from "./stores/uiStore";
import { useThemeStore } from "./stores/themeStore";
import { useDocStore } from "./stores/docStore";
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
  const setTheme = useThemeStore((s) => s.setTheme);
  const docTitle = useDocStore((s) => s.meta?.title);
  const docError = useDocStore((s) => s.error);
  const newDraft = useDocStore((s) => s.newDraft);
  const [settingsOpen, setSettingsOpen] = useState(false);

  // Load persisted settings once on mount (P7 wires the rest).
  useEffect(() => {
    import("./lib/ipc")
      .then(({ configLoad }) => configLoad())
      .then((settings) => setTheme(settings.theme))
      .catch(() => {});
  }, [setTheme]);

  // Ctrl+E: toggle between edit and immersion writing mode.
  // Ctrl+P: open the command palette.
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
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setLayoutMode]);

  const immersion = layoutMode === "immersion";

  return (
    <div
      className={`app ${immersion ? "app--immersion" : ""}`}
      data-theme={theme}
    >
      {!immersion && (
        <header className="topbar">
          <span className="topbar__title">{docTitle ?? "Ruach"}</span>
          <div className="topbar__modes" role="group" aria-label="布局模式">
            {(Object.keys(MODE_LABEL) as LayoutMode[]).map((mode) => (
              <Button
                key={mode}
                active={layoutMode === mode}
                onClick={() => setLayoutMode(mode)}
              >
                {MODE_LABEL[mode]}
              </Button>
            ))}
          </div>
          <div className="topbar__actions">
            <Button onClick={newDraft}>新建</Button>
            <Button onClick={toggleTree}>{treeVisible ? "藏树" : "树"}</Button>
            <Button active={settingsOpen} onClick={() => setSettingsOpen((v) => !v)}>
              设置
            </Button>
          </div>
        </header>
      )}

      <RecoveryBanner />

      {immersion && (
        <button className="immersion-exit" onClick={() => setLayoutMode("edit")} title="退出沉浸（Ctrl+E）">
          退出沉浸
        </button>
      )}

      <div className="body">
        {treeVisible && !immersion && <FileTree />}

        <main className="main">
          {!immersion && <TabStrip />}
          {docError && <div className="error-banner">{docError.message}</div>}
          {settingsOpen && !immersion ? (
            <SettingsPanel />
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
