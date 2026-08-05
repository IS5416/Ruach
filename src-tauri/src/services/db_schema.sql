-- Ruach vault sidecar schema (user_version 2; v2 adds the snapshots table
-- via SCHEMA_V2_SQL in db.rs).
-- Shared by Database::init and service tests via include_str!.

CREATE TABLE IF NOT EXISTS files (
  rel_path   TEXT PRIMARY KEY,
  title      TEXT NOT NULL,
  mtime      INTEGER NOT NULL,
  size       INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
  rel_path TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  tag      TEXT NOT NULL,
  PRIMARY KEY (rel_path, tag)
);

CREATE TABLE IF NOT EXISTS links (
  src_path TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  dst_path TEXT NOT NULL,
  label    TEXT,
  PRIMARY KEY (src_path, dst_path)
);

CREATE TABLE IF NOT EXISTS attachments (
  rel_path  TEXT NOT NULL REFERENCES files(rel_path) ON DELETE CASCADE,
  name      TEXT NOT NULL,
  orig_name TEXT,
  created_at INTEGER NOT NULL,
  PRIMARY KEY (rel_path, name)
);

CREATE TABLE IF NOT EXISTS recent (
  rel_path  TEXT PRIMARY KEY REFERENCES files(rel_path) ON DELETE CASCADE,
  opened_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
  doc_key    TEXT PRIMARY KEY,
  content    TEXT NOT NULL,
  cursor     INTEGER,
  updated_at INTEGER NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS docs_fts USING fts5(
  rel_path UNINDEXED, title, body, tokenize='trigram'
);
