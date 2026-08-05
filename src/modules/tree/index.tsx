import { useMemo, useRef, useState, type UIEvent } from "react";
import { useVaultStore } from "../../stores/vaultStore";
import { useDocStore } from "../../stores/docStore";
import type { TreeNode } from "../../lib/types";

const ROW_H = 26;
const OVERSCAN = 8;

type Row = { key: string; depth: number; node: TreeNode };

/** Group the flat scan result into a folder tree (files inside folders). */
function buildTree(flat: TreeNode[]): TreeNode[] {
  const roots: TreeNode[] = [];
  const dirs = new Map<string, TreeNode[]>();

  const getBucket = (dir: string): TreeNode[] => {
    if (dir === "") return roots;
    let bucket = dirs.get(dir);
    if (!bucket) {
      bucket = [];
      dirs.set(dir, bucket);
    }
    return bucket;
  };

  for (const node of flat) {
    const slash = node.rel_path.lastIndexOf("/");
    const dir = slash < 0 ? "" : node.rel_path.slice(0, slash);
    getBucket(dir).push(node);
  }

  // Attach children to dir nodes in a stable order (dirs before files).
  for (const [dir, bucket] of dirs) {
    const dirNode = flat.find((n) => n.is_dir && n.rel_path === dir);
    if (dirNode) {
      (dirNode as unknown as { children?: TreeNode[] }).children = bucket;
    }
  }
  for (const bucket of dirs.values()) {
    bucket.sort((a, b) => (a.is_dir !== b.is_dir ? (a.is_dir ? -1 : 1) : a.name.localeCompare(b.name)));
  }
  return roots;
}

/**
 * Vault file tree: folder collapse state + windowed rendering (virtual
 * scroll). Flat scan rows from Rust are grouped into a hierarchy here.
 */
export function FileTree() {
  const tree = useVaultStore((s) => s.tree);
  const openDoc = useDocStore((s) => s.openDoc);
  const current = useDocStore((s) => s.relPath);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportH, setViewportH] = useState(400);
  const containerRef = useRef<HTMLDivElement>(null);

  const rows = useMemo<Row[]>(() => {
    const out: Row[] = [];
    const walk = (nodes: TreeNode[], depth: number) => {
      for (const node of nodes) {
        out.push({ key: node.rel_path, depth, node });
        if (node.is_dir && expanded.has(node.rel_path)) {
          const children = (node as unknown as { children?: TreeNode[] }).children ?? [];
          walk(children, depth + 1);
        }
      }
    };
    walk(buildTree(tree), 0);
    return out;
  }, [tree, expanded]);

  const start = Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN);
  const end = Math.min(rows.length, Math.ceil((scrollTop + viewportH) / ROW_H) + OVERSCAN);
  const visible = rows.slice(start, end);

  const onScroll = (e: UIEvent<HTMLDivElement>) => setScrollTop(e.currentTarget.scrollTop);

  const toggle = (node: TreeNode) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(node.rel_path)) next.delete(node.rel_path);
      else next.add(node.rel_path);
      return next;
    });
  };

  const onContainerRef = (el: HTMLDivElement | null) => {
    containerRef.current = el;
    if (el) {
      setViewportH(el.clientHeight);
      // Re-measure on resize (simple observer, no layout lib).
      const ro = new ResizeObserver(() => setViewportH(el.clientHeight));
      ro.observe(el);
      (el as unknown as { __ro?: ResizeObserver }).__ro = ro;
    }
  };

  if (tree.length === 0) {
    return <nav className="tree" aria-label="文件树"><p className="tree__empty">打开一个 Vault 开始</p></nav>;
  }

  return (
    <nav
      ref={onContainerRef}
      className="tree"
      aria-label="文件树"
      onScroll={onScroll}
    >
      <div style={{ height: rows.length * ROW_H, position: "relative" }}>
        {visible.map((row, i) => (
          <div
            key={row.key}
            className={`tree__row${row.node.rel_path === current ? " tree__row--active" : ""}`}
            style={{ top: (start + i) * ROW_H }}
          >
            <button
              className={`tree__item${row.node.is_dir ? " tree__item--dir" : ""}`}
              style={{ paddingLeft: 8 + row.depth * 14 }}
              onClick={() => (row.node.is_dir ? toggle(row.node) : openDoc(row.node.rel_path))}
              title={row.node.rel_path}
            >
              <span
                className={`tree__chevron${expanded.has(row.node.rel_path) ? " tree__chevron--open" : ""}`}
              >
                {row.node.is_dir ? "▸" : ""}
              </span>
              {row.node.name}
            </button>
          </div>
        ))}
      </div>
    </nav>
  );
}
