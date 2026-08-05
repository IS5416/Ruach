/** Mirrors Rust DTOs. Keep in sync with src-tauri/src/services/*. */

export interface DocumentMeta {
  rel_path: string;
  title: string;
  mtime: number;
  size: number;
}

export interface DocOpenResult {
  content: string;
  meta: DocumentMeta;
}

export interface TreeNode {
  rel_path: string;
  name: string;
  is_dir: boolean;
}

export interface SearchHit {
  rel_path: string;
  title: string;
  score: number;
}

export type ThemeKind = "warm_paper" | "cold_stone" | "night_ink";
export type FontPreset = "serif" | "sans_serif";

export interface AppSettings {
  theme: ThemeKind;
  font_preset: FontPreset;
  line_height: number;
  page_width: number;
  show_file_tree: boolean;
}

export type LayoutMode = "edit" | "preview" | "split" | "immersion";
export type ExportFormat = "html" | "pdf";

/** Recovery buffer entry (crash recovery). */
export interface SessionInfo {
  doc_key: string;
  updated_at: number;
  preview: string;
}

export interface SessionDraft {
  content: string;
  cursor: number | null;
}

export interface AttachmentData {
  mime: string;
  base64: string;
}
