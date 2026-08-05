import { useEffect, useMemo, useRef, useState } from "react";
import { useUiStore } from "../../stores/uiStore";
import { useDocStore } from "../../stores/docStore";
import type { SearchHit } from "../../lib/types";

const SEARCH_DEBOUNCE_MS = 250;

interface Command {
  id: string;
  title: string;
  hint?: string;
  run: () => void;
}

/** Built-in command registry — extensible, searchable from the palette. */
const COMMANDS: Command[] = [
  {
    id: "new-draft",
    title: "新建文档",
    run: () => useDocStore.getState().newDraft(),
  },
  {
    id: "mode-edit",
    title: "切换到编辑",
    run: () => useUiStore.getState().setLayoutMode("edit"),
  },
  {
    id: "mode-preview",
    title: "切换到预览",
    run: () => useUiStore.getState().setLayoutMode("preview"),
  },
  {
    id: "mode-split",
    title: "分屏预览",
    run: () => useUiStore.getState().setLayoutMode("split"),
  },
  {
    id: "mode-immersion",
    title: "沉浸书写",
    hint: "Ctrl+E",
    run: () => useUiStore.getState().setLayoutMode("immersion"),
  },
  {
    id: "toggle-tree",
    title: "显示/隐藏文件树",
    run: () => useUiStore.getState().toggleTree(),
  },
];

interface Entry {
  key: string;
  title: string;
  hint: string;
  run: () => void;
}

/**
 * Command palette (Ctrl+P): document search via FTS + built-in commands.
 * Keyboard: up/down navigate, Enter run, Esc close.
 */
export function CommandPalette() {
  const open = useUiStore((s) => s.commandPaletteOpen);
  const setOpen = useUiStore((s) => s.setCommandPaletteOpen);
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const entries = useMemo<Entry[]>(() => {
    const q = query.trim().toLowerCase();
    const commands: Entry[] = q
      ? COMMANDS.filter((c) => c.title.toLowerCase().includes(q)).map((c) => ({
          key: `cmd:${c.id}`,
          title: c.title,
          hint: c.hint ?? "",
          run: c.run,
        }))
      : COMMANDS.map((c) => ({
          key: `cmd:${c.id}`,
          title: c.title,
          hint: c.hint ?? "",
          run: c.run,
        }));
    const docs: Entry[] = hits.map((h) => ({
      key: `doc:${h.rel_path}`,
      title: h.title,
      hint: h.rel_path,
      run: () => {
        void useDocStore.getState().openDoc(h.rel_path);
      },
    }));
    return [...commands, ...docs];
  }, [query, hits]);

  // Reset state each time the palette opens.
  useEffect(() => {
    if (open) {
      setQuery("");
      setHits([]);
      setSelected(0);
      inputRef.current?.focus();
    }
  }, [open]);

  // Debounced search once the user types >= 1 char.
  useEffect(() => {
    const q = query.trim();
    if (!open || q.length === 0) {
      setHits([]);
      return;
    }
    const timer = setTimeout(() => {
      import("../../lib/ipc")
        .then(({ searchQuery }) => searchQuery(q))
        .then(setHits)
        .catch(() => setHits([]));
    }, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [query, open]);

  useEffect(() => setSelected(0), [query, hits]);

  const runSelected = () => {
    const entry = entries[selected];
    if (!entry) return;
    setOpen(false);
    entry.run();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, entries.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => Math.max(s - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      runSelected();
    } else if (e.key === "Escape") {
      setOpen(false);
    }
  };

  if (!open) return null;
  return (
    <div className="palette-overlay" onClick={() => setOpen(false)}>
      <div className="palette" onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          className="palette__input"
          placeholder="搜索文档或输入命令…"
          value={query}
          onChange={(e) => setQuery(e.currentTarget.value)}
          onKeyDown={onKeyDown}
          aria-label="命令面板"
        />
        {entries.length > 0 && (
          <ul className="palette__list" role="listbox">
            {entries.map((entry, i) => (
              <li
                key={entry.key}
                role="option"
                aria-selected={i === selected}
                className={`palette__item${i === selected ? " palette__item--selected" : ""}`}
                onMouseEnter={() => setSelected(i)}
                onClick={() => {
                  setOpen(false);
                  entry.run();
                }}
              >
                <span className="palette__title">{entry.title}</span>
                {entry.hint && <span className="palette__hint">{entry.hint}</span>}
              </li>
            ))}
          </ul>
        )}
        {entries.length === 0 && <p className="palette__empty">无结果</p>}
      </div>
    </div>
  );
}
