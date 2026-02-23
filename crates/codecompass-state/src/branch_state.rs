use codecompass_core::error::StateError;
use rusqlite::{Connection, params};

/// A branch state entry tracking indexing progress per repo/ref.
#[derive(Debug, Clone)]
pub struct BranchState {
    pub repo: String,
    pub r#ref: String,
    pub merge_base_commit: Option<String>,
    pub last_indexed_commit: String,
    pub overlay_dir: Option<String>,
    pub file_count: i64,
    pub created_at: String,
    pub last_accessed_at: String,
}

/// Upsert a branch state entry (INSERT OR REPLACE on composite PK).
pub fn upsert_branch_state(conn: &Connection, entry: &BranchState) -> Result<(), StateError> {
    conn.execute(
        "INSERT INTO branch_state (repo, \"ref\", merge_base_commit, last_indexed_commit, overlay_dir, file_count, created_at, last_accessed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(repo, \"ref\") DO UPDATE SET
           merge_base_commit = excluded.merge_base_commit,
           last_indexed_commit = excluded.last_indexed_commit,
           overlay_dir = excluded.overlay_dir,
           file_count = excluded.file_count,
           created_at = excluded.created_at,
           last_accessed_at = excluded.last_accessed_at",
        params![
            entry.repo,
            entry.r#ref,
            entry.merge_base_commit,
            entry.last_indexed_commit,
            entry.overlay_dir,
            entry.file_count,
            entry.created_at,
            entry.last_accessed_at,
        ],
    )
    .map_err(StateError::sqlite)?;
    Ok(())
}

/// Get branch state by primary key (repo, ref). Returns None if not found.
pub fn get_branch_state(
    conn: &Connection,
    repo: &str,
    r#ref: &str,
) -> Result<Option<BranchState>, StateError> {
    let result = conn.query_row(
        "SELECT repo, \"ref\", merge_base_commit, last_indexed_commit, overlay_dir, file_count, created_at, last_accessed_at
         FROM branch_state WHERE repo = ?1 AND \"ref\" = ?2",
        params![repo, r#ref],
        |row| {
            Ok(BranchState {
                repo: row.get(0)?,
                r#ref: row.get(1)?,
                merge_base_commit: row.get(2)?,
                last_indexed_commit: row.get(3)?,
                overlay_dir: row.get(4)?,
                file_count: row.get(5)?,
                created_at: row.get(6)?,
                last_accessed_at: row.get(7)?,
            })
        },
    );

    match result {
        Ok(entry) => Ok(Some(entry)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(StateError::sqlite(e)),
    }
}

/// Delete a branch state entry by primary key (repo, ref).
pub fn delete_branch_state(conn: &Connection, repo: &str, r#ref: &str) -> Result<(), StateError> {
    conn.execute(
        "DELETE FROM branch_state WHERE repo = ?1 AND \"ref\" = ?2",
        params![repo, r#ref],
    )
    .map_err(StateError::sqlite)?;
    Ok(())
}

/// List all branch states for a given repo.
pub fn list_branch_states(conn: &Connection, repo: &str) -> Result<Vec<BranchState>, StateError> {
    let mut stmt = conn
        .prepare(
            "SELECT repo, \"ref\", merge_base_commit, last_indexed_commit, overlay_dir, file_count, created_at, last_accessed_at
             FROM branch_state WHERE repo = ?1",
        )
        .map_err(StateError::sqlite)?;

    let entries = stmt
        .query_map(params![repo], |row| {
            Ok(BranchState {
                repo: row.get(0)?,
                r#ref: row.get(1)?,
                merge_base_commit: row.get(2)?,
                last_indexed_commit: row.get(3)?,
                overlay_dir: row.get(4)?,
                file_count: row.get(5)?,
                created_at: row.get(6)?,
                last_accessed_at: row.get(7)?,
            })
        })
        .map_err(StateError::sqlite)?;

    entries
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StateError::Sqlite(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::schema;
    use tempfile::tempdir;

    fn setup_test_db() -> Connection {
        let dir = tempdir().unwrap();
        let conn = db::open_connection(&dir.path().join("test.db")).unwrap();
        schema::create_tables(&conn).unwrap();
        conn
    }

    fn sample_entry() -> BranchState {
        BranchState {
            repo: "my-repo".to_string(),
            r#ref: "main".to_string(),
            merge_base_commit: Some("abc123".to_string()),
            last_indexed_commit: "def456".to_string(),
            overlay_dir: Some("/tmp/overlay".to_string()),
            file_count: 42,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_accessed_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_upsert_and_get() {
        let conn = setup_test_db();
        let entry = sample_entry();

        upsert_branch_state(&conn, &entry).unwrap();

        let result = get_branch_state(&conn, &entry.repo, &entry.r#ref).unwrap();
        assert!(result.is_some());
        let got = result.unwrap();
        assert_eq!(got.repo, "my-repo");
        assert_eq!(got.r#ref, "main");
        assert_eq!(got.merge_base_commit, Some("abc123".to_string()));
        assert_eq!(got.last_indexed_commit, "def456");
        assert_eq!(got.overlay_dir, Some("/tmp/overlay".to_string()));
        assert_eq!(got.file_count, 42);
        assert_eq!(got.created_at, "2026-01-01T00:00:00Z");
        assert_eq!(got.last_accessed_at, "2026-01-01T00:00:00Z");
    }

    #[test]
    fn test_get_returns_none_when_not_found() {
        let conn = setup_test_db();
        let result = get_branch_state(&conn, "no-repo", "main").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_upsert_updates_existing() {
        let conn = setup_test_db();
        let entry = sample_entry();
        upsert_branch_state(&conn, &entry).unwrap();

        // Upsert with updated fields for the same (repo, ref)
        let mut updated = entry.clone();
        updated.last_indexed_commit = "new_commit_789".to_string();
        updated.file_count = 100;
        updated.merge_base_commit = Some("new_base".to_string());
        updated.overlay_dir = None;
        updated.last_accessed_at = "2026-02-01T00:00:00Z".to_string();
        upsert_branch_state(&conn, &updated).unwrap();

        let got = get_branch_state(&conn, "my-repo", "main").unwrap().unwrap();
        assert_eq!(got.last_indexed_commit, "new_commit_789");
        assert_eq!(got.file_count, 100);
        assert_eq!(got.merge_base_commit, Some("new_base".to_string()));
        assert!(got.overlay_dir.is_none());
        assert_eq!(got.last_accessed_at, "2026-02-01T00:00:00Z");

        // Should still be only 1 entry for this repo
        let all = list_branch_states(&conn, "my-repo").unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_delete() {
        let conn = setup_test_db();
        let entry = sample_entry();
        upsert_branch_state(&conn, &entry).unwrap();

        delete_branch_state(&conn, &entry.repo, &entry.r#ref).unwrap();

        let result = get_branch_state(&conn, &entry.repo, &entry.r#ref).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_nonexistent_is_ok() {
        let conn = setup_test_db();
        // Deleting a non-existent entry should succeed (0 rows affected)
        let result = delete_branch_state(&conn, "no-repo", "main");
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_branch_states() {
        let conn = setup_test_db();

        let entry1 = sample_entry();
        upsert_branch_state(&conn, &entry1).unwrap();

        let mut entry2 = sample_entry();
        entry2.r#ref = "feature/xyz".to_string();
        entry2.last_indexed_commit = "commit_xyz".to_string();
        upsert_branch_state(&conn, &entry2).unwrap();

        let mut entry3 = sample_entry();
        entry3.r#ref = "develop".to_string();
        entry3.last_indexed_commit = "commit_dev".to_string();
        upsert_branch_state(&conn, &entry3).unwrap();

        let entries = list_branch_states(&conn, "my-repo").unwrap();
        assert_eq!(entries.len(), 3);

        let refs: Vec<&str> = entries.iter().map(|e| e.r#ref.as_str()).collect();
        assert!(refs.contains(&"main"));
        assert!(refs.contains(&"feature/xyz"));
        assert!(refs.contains(&"develop"));

        // Different repo should have 0
        let other = list_branch_states(&conn, "other-repo").unwrap();
        assert!(other.is_empty());
    }

    #[test]
    fn test_list_empty() {
        let conn = setup_test_db();
        let entries = list_branch_states(&conn, "no-repo").unwrap();
        assert!(entries.is_empty());
    }
}
