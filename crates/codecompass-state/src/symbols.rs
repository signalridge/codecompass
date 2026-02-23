use codecompass_core::error::StateError;
use codecompass_core::types::SymbolRecord;
use rusqlite::{Connection, params};

/// Insert a symbol relation record.
pub fn insert_symbol(conn: &Connection, sym: &SymbolRecord) -> Result<(), StateError> {
    conn.execute(
        "INSERT OR REPLACE INTO symbol_relations
         (repo, \"ref\", \"commit\", path, symbol_id, symbol_stable_id, name, qualified_name, kind, language, line_start, line_end, signature, parent_symbol_id, visibility, content_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            sym.repo,
            sym.r#ref,
            sym.commit,
            sym.path,
            sym.symbol_id,
            sym.symbol_stable_id,
            sym.name,
            sym.qualified_name,
            sym.kind.as_str(),
            sym.language,
            sym.line_start,
            sym.line_end,
            sym.signature,
            sym.parent_symbol_id,
            sym.visibility,
            sym.content.as_deref().map(|c| {
                blake3::hash(c.as_bytes()).to_hex().to_string()
            }).unwrap_or_default(),
        ],
    ).map_err(StateError::sqlite)?;
    Ok(())
}

/// Look up symbols by name for a repo/ref, used for dual-index join.
pub fn find_symbols_by_location(
    conn: &Connection,
    repo: &str,
    r#ref: &str,
    path: &str,
    line_start: u32,
    line_end: u32,
) -> Result<Vec<SymbolRecord>, StateError> {
    let mut stmt = conn.prepare(
        "SELECT repo, \"ref\", \"commit\", path, symbol_id, symbol_stable_id, name, qualified_name, kind, language, line_start, line_end, signature, parent_symbol_id, visibility
         FROM symbol_relations
         WHERE repo = ?1 AND \"ref\" = ?2 AND path = ?3
         AND line_start <= ?5 AND line_end >= ?4"
    ).map_err(StateError::sqlite)?;

    let symbols = stmt
        .query_map(params![repo, r#ref, path, line_start, line_end], |row| {
            Ok(SymbolRecord {
                repo: row.get(0)?,
                r#ref: row.get(1)?,
                commit: row.get(2)?,
                path: row.get(3)?,
                symbol_id: row.get(4)?,
                symbol_stable_id: row.get(5)?,
                name: row.get(6)?,
                qualified_name: row.get(7)?,
                kind: codecompass_core::types::SymbolKind::parse_kind(&row.get::<_, String>(8)?)
                    .unwrap_or(codecompass_core::types::SymbolKind::Function),
                language: row.get(9)?,
                line_start: row.get(10)?,
                line_end: row.get(11)?,
                signature: row.get(12)?,
                parent_symbol_id: row.get(13)?,
                visibility: row.get(14)?,
                content: None,
            })
        })
        .map_err(StateError::sqlite)?;

    symbols
        .collect::<Result<Vec<_>, _>>()
        .map_err(StateError::sqlite)
}

/// Delete all symbols for a given repo/ref/path.
pub fn delete_symbols_for_file(
    conn: &Connection,
    repo: &str,
    r#ref: &str,
    path: &str,
) -> Result<(), StateError> {
    conn.execute(
        "DELETE FROM symbol_relations WHERE repo = ?1 AND \"ref\" = ?2 AND path = ?3",
        params![repo, r#ref, path],
    )
    .map_err(StateError::sqlite)?;
    Ok(())
}

/// Get total symbol count for a repo/ref.
pub fn symbol_count(conn: &Connection, repo: &str, r#ref: &str) -> Result<u64, StateError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM symbol_relations WHERE repo = ?1 AND \"ref\" = ?2",
            params![repo, r#ref],
            |row| row.get(0),
        )
        .map_err(StateError::sqlite)?;
    Ok(count as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::schema;
    use codecompass_core::types::SymbolKind;
    use tempfile::tempdir;

    fn setup_test_db() -> Connection {
        let dir = tempdir().unwrap();
        let conn = db::open_connection(&dir.path().join("test.db")).unwrap();
        schema::create_tables(&conn).unwrap();
        conn
    }

    fn sample_symbol() -> SymbolRecord {
        SymbolRecord {
            repo: "my-repo".to_string(),
            r#ref: "main".to_string(),
            commit: Some("abc123".to_string()),
            path: "src/lib.rs".to_string(),
            symbol_id: "sym_001".to_string(),
            symbol_stable_id: "stable_001".to_string(),
            name: "my_function".to_string(),
            qualified_name: "crate::my_function".to_string(),
            kind: SymbolKind::Function,
            language: "rust".to_string(),
            line_start: 10,
            line_end: 25,
            signature: Some("fn my_function(x: i32) -> bool".to_string()),
            parent_symbol_id: None,
            visibility: Some("pub".to_string()),
            content: Some("fn my_function(x: i32) -> bool { x > 0 }".to_string()),
        }
    }

    #[test]
    fn test_insert_and_find_symbol() {
        let conn = setup_test_db();
        let sym = sample_symbol();

        insert_symbol(&conn, &sym).unwrap();

        let found = find_symbols_by_location(
            &conn,
            &sym.repo,
            &sym.r#ref,
            &sym.path,
            sym.line_start,
            sym.line_end,
        )
        .unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "my_function");
        assert_eq!(found[0].qualified_name, "crate::my_function");
        assert_eq!(found[0].kind, SymbolKind::Function);
        assert_eq!(found[0].language, "rust");
        assert_eq!(found[0].line_start, 10);
        assert_eq!(found[0].line_end, 25);
        assert_eq!(found[0].commit, Some("abc123".to_string()));
        assert_eq!(
            found[0].signature,
            Some("fn my_function(x: i32) -> bool".to_string())
        );
        assert_eq!(found[0].visibility, Some("pub".to_string()));
    }

    #[test]
    fn test_find_symbols_by_location_overlapping_range() {
        let conn = setup_test_db();
        let sym = sample_symbol(); // lines 10-25
        insert_symbol(&conn, &sym).unwrap();

        // Query a range that overlaps: the query uses line_start <= ?5 AND line_end >= ?4
        // so we query with a subrange inside the symbol
        let found =
            find_symbols_by_location(&conn, "my-repo", "main", "src/lib.rs", 15, 20).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "my_function");
    }

    #[test]
    fn test_find_symbols_by_location_no_overlap() {
        let conn = setup_test_db();
        let sym = sample_symbol(); // lines 10-25
        insert_symbol(&conn, &sym).unwrap();

        // Query a range completely outside the symbol
        let found =
            find_symbols_by_location(&conn, "my-repo", "main", "src/lib.rs", 30, 40).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn test_find_symbols_by_location_wrong_path() {
        let conn = setup_test_db();
        let sym = sample_symbol();
        insert_symbol(&conn, &sym).unwrap();

        let found = find_symbols_by_location(
            &conn,
            "my-repo",
            "main",
            "src/other.rs",
            sym.line_start,
            sym.line_end,
        )
        .unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn test_insert_symbol_replace_on_conflict() {
        let conn = setup_test_db();
        let sym = sample_symbol();
        insert_symbol(&conn, &sym).unwrap();

        // Insert a symbol with the same unique key but different name
        let mut sym2 = sym.clone();
        sym2.name = "renamed_function".to_string();
        insert_symbol(&conn, &sym2).unwrap();

        let found = find_symbols_by_location(
            &conn,
            &sym.repo,
            &sym.r#ref,
            &sym.path,
            sym.line_start,
            sym.line_end,
        )
        .unwrap();
        // Should have the updated name (INSERT OR REPLACE)
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "renamed_function");
    }

    #[test]
    fn test_delete_symbols_for_file() {
        let conn = setup_test_db();
        let sym = sample_symbol();
        insert_symbol(&conn, &sym).unwrap();

        delete_symbols_for_file(&conn, &sym.repo, &sym.r#ref, &sym.path).unwrap();

        let count = symbol_count(&conn, &sym.repo, &sym.r#ref).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_delete_symbols_for_file_nonexistent_is_ok() {
        let conn = setup_test_db();
        let result = delete_symbols_for_file(&conn, "no-repo", "main", "no-file.rs");
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_symbols_only_affects_target_file() {
        let conn = setup_test_db();

        let sym1 = sample_symbol();
        insert_symbol(&conn, &sym1).unwrap();

        let mut sym2 = sample_symbol();
        sym2.path = "src/other.rs".to_string();
        sym2.symbol_id = "sym_002".to_string();
        sym2.symbol_stable_id = "stable_002".to_string();
        sym2.qualified_name = "crate::other_function".to_string();
        sym2.name = "other_function".to_string();
        insert_symbol(&conn, &sym2).unwrap();

        assert_eq!(symbol_count(&conn, "my-repo", "main").unwrap(), 2);

        delete_symbols_for_file(&conn, "my-repo", "main", "src/lib.rs").unwrap();

        assert_eq!(symbol_count(&conn, "my-repo", "main").unwrap(), 1);
    }

    #[test]
    fn test_symbol_count() {
        let conn = setup_test_db();

        assert_eq!(symbol_count(&conn, "my-repo", "main").unwrap(), 0);

        let sym1 = sample_symbol();
        insert_symbol(&conn, &sym1).unwrap();

        let mut sym2 = sample_symbol();
        sym2.path = "src/main.rs".to_string();
        sym2.symbol_id = "sym_002".to_string();
        sym2.symbol_stable_id = "stable_002".to_string();
        sym2.qualified_name = "crate::main".to_string();
        sym2.name = "main".to_string();
        sym2.kind = SymbolKind::Function;
        sym2.line_start = 1;
        sym2.line_end = 5;
        insert_symbol(&conn, &sym2).unwrap();

        assert_eq!(symbol_count(&conn, "my-repo", "main").unwrap(), 2);
    }

    #[test]
    fn test_symbol_count_scoped_to_repo_and_ref() {
        let conn = setup_test_db();
        let sym = sample_symbol();
        insert_symbol(&conn, &sym).unwrap();

        assert_eq!(symbol_count(&conn, "other-repo", "main").unwrap(), 0);
        assert_eq!(symbol_count(&conn, "my-repo", "develop").unwrap(), 0);
    }

    #[test]
    fn test_symbol_with_no_optional_fields() {
        let conn = setup_test_db();
        let sym = SymbolRecord {
            repo: "repo".to_string(),
            r#ref: "main".to_string(),
            commit: None,
            path: "file.rs".to_string(),
            symbol_id: "sym_min".to_string(),
            symbol_stable_id: "stable_min".to_string(),
            name: "bare_fn".to_string(),
            qualified_name: "bare_fn".to_string(),
            kind: SymbolKind::Function,
            language: "rust".to_string(),
            line_start: 1,
            line_end: 3,
            signature: None,
            parent_symbol_id: None,
            visibility: None,
            content: None,
        };

        insert_symbol(&conn, &sym).unwrap();

        let found = find_symbols_by_location(&conn, "repo", "main", "file.rs", 1, 3).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].commit.is_none());
        assert!(found[0].signature.is_none());
        assert!(found[0].parent_symbol_id.is_none());
        assert!(found[0].visibility.is_none());
    }

    #[test]
    fn test_multiple_symbols_same_file() {
        let conn = setup_test_db();

        let sym1 = SymbolRecord {
            repo: "repo".to_string(),
            r#ref: "main".to_string(),
            commit: None,
            path: "src/lib.rs".to_string(),
            symbol_id: "sym_a".to_string(),
            symbol_stable_id: "stable_a".to_string(),
            name: "func_a".to_string(),
            qualified_name: "crate::func_a".to_string(),
            kind: SymbolKind::Function,
            language: "rust".to_string(),
            line_start: 1,
            line_end: 10,
            signature: None,
            parent_symbol_id: None,
            visibility: None,
            content: None,
        };

        let sym2 = SymbolRecord {
            repo: "repo".to_string(),
            r#ref: "main".to_string(),
            commit: None,
            path: "src/lib.rs".to_string(),
            symbol_id: "sym_b".to_string(),
            symbol_stable_id: "stable_b".to_string(),
            name: "func_b".to_string(),
            qualified_name: "crate::func_b".to_string(),
            kind: SymbolKind::Function,
            language: "rust".to_string(),
            line_start: 15,
            line_end: 25,
            signature: None,
            parent_symbol_id: None,
            visibility: None,
            content: None,
        };

        insert_symbol(&conn, &sym1).unwrap();
        insert_symbol(&conn, &sym2).unwrap();

        // Query spanning both symbols
        let found = find_symbols_by_location(&conn, "repo", "main", "src/lib.rs", 1, 25).unwrap();
        assert_eq!(found.len(), 2);

        // Query spanning only the first
        let found = find_symbols_by_location(&conn, "repo", "main", "src/lib.rs", 1, 10).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "func_a");
    }
}
