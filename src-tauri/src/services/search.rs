use crate::error::AppError;
use rusqlite::Connection;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchHit {
    pub rel_path: String,
    pub title: String,
    /// 1 = title match, 0 = body match. Rows are already sorted by the
    /// Rust side (score DESC, then FTS rank).
    pub score: u8,
}

const LIMIT: usize = 50;

/// Full-text search over the sidecar FTS5 index (trigram tokenizer —
/// CJK substring friendly). Queries with fewer than 3 chars cannot use
/// trigram tokens, so they fall back to a LIKE scan over title+body.
pub struct SearchService;

impl SearchService {
    pub fn query(conn: &Connection, q: &str) -> Result<Vec<SearchHit>, AppError> {
        let q = q.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        if q.chars().count() >= 3 {
            Self::fts_query(conn, q)
        } else {
            Self::like_query(conn, q)
        }
    }

    fn fts_query(conn: &Connection, q: &str) -> Result<Vec<SearchHit>, AppError> {
        // Quote the phrase and escape embedded quotes (FTS5 string literal).
        let escaped = q.replace('"', "\"\"");
        let matcher = format!("\"{escaped}\"");
        let mut stmt = conn.prepare(
            "SELECT rel_path, title, (title LIKE ?2) AS score
             FROM docs_fts WHERE docs_fts MATCH ?1
             ORDER BY score DESC, rank
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(
            rusqlite::params![matcher, format!("%{q}%"), LIMIT as i64],
            |r| {
                Ok(SearchHit {
                    rel_path: r.get(0)?,
                    title: r.get(1)?,
                    score: if r.get::<_, bool>(2)? { 1 } else { 0 },
                })
            },
        )?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn like_query(conn: &Connection, q: &str) -> Result<Vec<SearchHit>, AppError> {
        // Escape LIKE wildcards so a literal % or _ in the query doesn't
        // match every row.
        let escaped = q
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%{escaped}%");
        let mut stmt = conn.prepare(
            "SELECT rel_path, title, (title LIKE ?1) AS score
             FROM docs_fts WHERE title LIKE ?1 ESCAPE '\\' OR body LIKE ?1 ESCAPE '\\'
             ORDER BY score DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![pattern, LIMIT as i64], |r| {
            Ok(SearchHit {
                rel_path: r.get(0)?,
                title: r.get(1)?,
                score: if r.get::<_, bool>(2)? { 1 } else { 0 },
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::db::SCHEMA_SQL;
    use crate::services::index::IndexService;

    fn seed() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        for (path, title) in [
            ("notes/a.md", "风的形状"),
            ("notes/b.md", "雨水落进石缝"),
            ("notes/c.md", "风的另一面"),
        ] {
            conn.execute(
                "INSERT INTO files (rel_path, title, mtime, size, created_at, updated_at)
                 VALUES (?1, ?2, 0, 0, 0, 0)",
                rusqlite::params![path, title],
            )
            .unwrap();
            IndexService::index_file_content(&conn, path, &format!("# {title}\n\n正文内容\n"))
                .unwrap();
        }
        conn
    }

    #[test]
    fn fts_finds_three_char_phrase() {
        let conn = seed();
        let hits = SearchService::query(&conn, "风").unwrap();
        // Single char -> LIKE fallback.
        let titles: Vec<&str> = hits.iter().map(|h| h.title.as_str()).collect();
        assert_eq!(titles.len(), 2);
        assert!(titles.contains(&"风的形状"));

        let hits = SearchService::query(&conn, "石缝").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "雨水落进石缝");
    }

    #[test]
    fn title_match_ranks_first() {
        let conn = seed();
        // "风" hits two docs; the title-bearing one ranks first.
        let hits = SearchService::query(&conn, "风").unwrap();
        assert_eq!(hits[0].score, 1);
        assert!(hits.iter().any(|h| h.score == 1));
    }

    #[test]
    fn empty_query_returns_nothing() {
        let conn = seed();
        assert!(SearchService::query(&conn, "  ").unwrap().is_empty());
    }
}
