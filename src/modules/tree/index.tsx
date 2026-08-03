import { useVaultStore } from "../../stores/vaultStore";
import { useDocStore } from "../../stores/docStore";

/**
 * Vault file tree. Virtual scrolling + expand states land in P2;
 * this placeholder lists files flat from vaultScan.
 */
export function FileTree() {
  const tree = useVaultStore((s) => s.tree);
  const openDoc = useDocStore((s) => s.openDoc);
  const current = useDocStore((s) => s.relPath);

  const files = tree.filter((n) => !n.is_dir);

  return (
    <nav className="tree" aria-label="文件树">
      {files.length === 0 && <p className="tree__empty">打开一个 Vault 开始</p>}
      <ul className="tree__list">
        {files.map((n) => (
          <li key={n.rel_path}>
            <button
              className={`tree__item${n.rel_path === current ? " tree__item--active" : ""}`}
              onClick={() => openDoc(n.rel_path)}
              title={n.rel_path}
            >
              {n.name}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
