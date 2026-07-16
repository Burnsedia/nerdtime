// SPDX-License-Identifier: AGPL-3.0-only
use nerdtime_db as db;
use nerdtime_db::Connection;

fn temp_db() -> (Connection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conn = Connection::open(&path).unwrap();
    db::init_schema(&conn).unwrap();
    (conn, dir)
}

fn seed_session(conn: &Connection, project: &str, started: &str, ended: &str) {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO sessions (id, project_name, started_at, ended_at, is_synced) VALUES (?1, ?2, ?3, ?4, 0)",
        rusqlite::params![id, project, started, ended],
    )
    .unwrap();
}

#[test]
fn test_heatmap_empty() {
    let (conn, _dir) = temp_db();
    let cells = db::heatmap_data(&conn, 30, None).unwrap();
    assert!(cells.is_empty());
}

#[test]
fn test_heatmap_with_data() {
    let (conn, _dir) = temp_db();
    seed_session(&conn, "nerdtime", "2026-07-15T10:00:00Z", "2026-07-15T12:00:00Z");
    let cells = db::heatmap_data(&conn, 30, None).unwrap();
    assert!(!cells.is_empty());
}

#[test]
fn test_insights_empty() {
    let (conn, _dir) = temp_db();
    let data = db::insights_data(&conn, 30, None).unwrap();
    assert_eq!(data.total_seconds, 0);
    assert_eq!(data.session_count, 0);
}

#[test]
fn test_insights_with_data() {
    let (conn, _dir) = temp_db();
    seed_session(&conn, "nerdtime", "2026-07-15T10:00:00Z", "2026-07-15T12:00:00Z");
    seed_session(&conn, "website", "2026-07-14T14:00:00Z", "2026-07-14T16:00:00Z");
    let data = db::insights_data(&conn, 30, None).unwrap();
    assert_eq!(data.session_count, 2);
    assert!(data.total_seconds > 0);
}

#[test]
fn test_insights_project_filter() {
    let (conn, _dir) = temp_db();
    seed_session(&conn, "nerdtime", "2026-07-15T10:00:00Z", "2026-07-15T12:00:00Z");
    seed_session(&conn, "website", "2026-07-14T14:00:00Z", "2026-07-14T16:00:00Z");
    let data = db::insights_data(&conn, 30, Some("nerdtime")).unwrap();
    assert_eq!(data.session_count, 1);
}

#[test]
fn test_stats_by_project_aggregation() {
    let (conn, _dir) = temp_db();
    seed_session(&conn, "nerdtime", "2026-07-15T10:00:00Z", "2026-07-15T12:00:00Z");
    seed_session(&conn, "nerdtime", "2026-07-14T09:00:00Z", "2026-07-14T11:00:00Z");
    let stats = db::stats_by_project(&conn).unwrap();
    let nerdtime = stats.iter().find(|s| s.project == "nerdtime").unwrap();
    assert_eq!(nerdtime.session_count, 2);
    assert_eq!(nerdtime.total_seconds, 14400);
}

#[test]
fn test_label_summary_with_labels() {
    let (conn, _dir) = temp_db();
    let labels = r#"["bug"]"#;
    seed_session(&conn, "nerdtime", "2026-07-15T10:00:00Z", "2026-07-15T11:00:00Z");
    conn.execute(
        "UPDATE sessions SET labels = ?1 WHERE project_name = 'nerdtime'",
        rusqlite::params![labels],
    )
    .unwrap();
    let rows = db::label_summary(&conn, None, None, "2026-01-01", "2099-12-31").unwrap();
    let bug = rows.iter().find(|(l, _, _)| l == "bug");
    assert!(bug.is_some());
}
