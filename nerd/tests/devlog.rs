// SPDX-License-Identifier: AGPL-3.0-only
use nerdtime_db as db;
use nerdtime_db::Connection;
use nerdtime_core::DevlogEntry;

fn temp_db() -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conn = Connection::open(&path).unwrap();
    db::init_schema(&conn).unwrap();
    (conn, dir)
}

fn seed_devlog(conn: &Connection, date: &str, title: &str, role: &str, tags: &[&str]) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO devlog_entries (id, date, title, role, tags, context, changes, decisions, commits, created_at) VALUES (?1, ?2, ?3, ?4, ?5, '', '', '', '[]', ?6)",
        rusqlite::params![id, date, title, role, serde_json::to_string(tags).unwrap(), "2026-07-15T10:00:00Z"],
    )
    .unwrap();
    id
}

#[test]
fn test_devlog_insert() {
    let (conn, _dir) = temp_db();
    let entry = DevlogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        date: "2026-07-15".to_string(),
        title: "Test entry".to_string(),
        role: "human".to_string(),
        tags: vec!["test".to_string()],
        context: "testing context".to_string(),
        changes: vec!["added tests".to_string()],
        decisions: vec!["use tempfile".to_string()],
        commits: vec![],
        session_id: None,
        created_at: "2026-07-15T10:00:00Z".to_string(),
    };
    db::insert_devlog_entry(&conn, &entry).unwrap();
    let entries = db::list_devlog_entries(&conn, 100).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "Test entry");
}

#[test]
fn test_devlog_list() {
    let (conn, _dir) = temp_db();
    seed_devlog(&conn, "2026-07-15", "First", "human", &["a"]);
    seed_devlog(&conn, "2026-07-14", "Second", "ai", &["b"]);
    let entries = db::list_devlog_entries(&conn, 10).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_devlog_list_limit() {
    let (conn, _dir) = temp_db();
    seed_devlog(&conn, "2026-07-15", "One", "human", &[]);
    seed_devlog(&conn, "2026-07-14", "Two", "ai", &[]);
    let entries = db::list_devlog_entries(&conn, 1).unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn test_devlog_search_text() {
    let (conn, _dir) = temp_db();
    seed_devlog(&conn, "2026-07-15", "Fix auth bug", "human", &["auth"]);
    seed_devlog(&conn, "2026-07-14", "Add dark mode", "ai", &["ui"]);
    seed_devlog(&conn, "2026-07-13", "Refactor auth", "hybrid", &["auth"]);
    let entries = db::search_devlog_entries(&conn, "auth", None).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_devlog_search_tags() {
    let (conn, _dir) = temp_db();
    seed_devlog(&conn, "2026-07-15", "MCP server", "ai", &["mcp", "server"]);
    seed_devlog(&conn, "2026-07-14", "CLI refactor", "human", &["cli"]);
    let entries = db::search_devlog_entries(&conn, "", Some("mcp")).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].tags.contains(&"mcp".to_string()));
}

#[test]
fn test_devlog_get() {
    let (conn, _dir) = temp_db();
    let id = seed_devlog(&conn, "2026-07-15", "Get me", "human", &[]);
    let entry = db::get_devlog_entry(&conn, &id).unwrap();
    assert_eq!(entry.title, "Get me");
}

#[test]
fn test_devlog_update() {
    let (conn, _dir) = temp_db();
    let id = seed_devlog(&conn, "2026-07-15", "Original", "human", &[]);
    let mut entry = db::get_devlog_entry(&conn, &id).unwrap();
    entry.title = "Updated".to_string();
    entry.role = "ai".to_string();
    db::update_devlog_entry(&conn, &entry).unwrap();
    let updated = db::get_devlog_entry(&conn, &id).unwrap();
    assert_eq!(updated.title, "Updated");
    assert_eq!(updated.role, "ai");
}

#[test]
fn test_devlog_render_md() {
    let (conn, _dir) = temp_db();
    seed_devlog(&conn, "2026-07-15", "Markdown test", "human", &["test"]);
    let md = db::render_devlog_md(&conn).unwrap();
    assert!(md.contains("2026-07-15"));
    assert!(md.contains("Markdown test"));
}

#[test]
fn test_devlog_render_md_empty() {
    let (conn, _dir) = temp_db();
    let md = db::render_devlog_md(&conn).unwrap();
    assert!(md.contains("Development Log"));
}

#[test]
fn test_devlog_get_last_date() {
    let (conn, _dir) = temp_db();
    seed_devlog(&conn, "2026-07-14", "Older", "human", &[]);
    let last = db::get_last_devlog_date(&conn).unwrap();
    assert_eq!(last, Some("2026-07-14".to_string()));
}
