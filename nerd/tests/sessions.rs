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

fn seed_session(conn: &Connection, project: &str, started: &str, ended: Option<&str>) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO sessions (id, project_name, started_at, ended_at, is_synced) VALUES (?1, ?2, ?3, ?4, 0)",
        rusqlite::params![id, project, started, ended],
    )
    .unwrap();
    id
}

fn seed_sessions(conn: &Connection) {
    seed_session(conn, "nerdtime", "2026-07-15T10:00:00Z", Some("2026-07-15T12:00:00Z"));
    seed_session(conn, "website", "2026-07-15T13:00:00Z", Some("2026-07-15T15:30:00Z"));
    seed_session(conn, "nerdtime", "2026-07-14T09:00:00Z", Some("2026-07-14T11:00:00Z"));
}

fn seed_task(conn: &Connection, project: &str, title: &str, quadrant: u8, estimate_secs: Option<i64>) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO tasks (id, project_name, title, estimated_seconds, urgency, importance, quadrant, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8)",
        rusqlite::params![id, project, title, estimate_secs, 3, 3, quadrant, "2026-07-15T10:00:00Z"],
    )
    .unwrap();
    id
}

#[test]
fn test_session_start_stop() {
    let (conn, _dir) = temp_db();
    let session = db::start_session(&conn, "test-project", None, None, None, None).unwrap();
    assert_eq!(session.project_name, "test-project");
    assert!(session.ended_at.is_none());

    let stopped = db::stop_session(&conn).unwrap();
    assert_eq!(stopped.id, session.id);
    assert!(stopped.ended_at.is_some());
}

#[test]
fn test_session_start_with_task() {
    let (conn, _dir) = temp_db();
    let task_id = seed_task(&conn, "nerdtime", "Fix bug", 1, Some(3600));

    let session = db::start_session(&conn, "nerdtime", None, Some(&task_id), Some(3600), None).unwrap();
    assert_eq!(session.task_id.unwrap(), task_id);
    assert_eq!(session.estimated_seconds, Some(3600));
}

#[test]
fn test_session_start_with_labels() {
    let (conn, _dir) = temp_db();
    let labels = r#"["feat","urgent"]"#;
    let session = db::start_session(&conn, "test", None, None, None, Some(labels)).unwrap();
    assert_eq!(session.labels.unwrap(), labels);
}

#[test]
fn test_session_status_active() {
    let (conn, _dir) = temp_db();
    db::start_session(&conn, "active-proj", None, None, None, None).unwrap();
    let status = db::show_status(&conn).unwrap().unwrap();
    assert_eq!(status.project_name, "active-proj");
}

#[test]
fn test_session_status_none() {
    let (conn, _dir) = temp_db();
    let status = db::show_status(&conn).unwrap();
    assert!(status.is_none());
}

#[test]
fn test_session_list() {
    let (conn, _dir) = temp_db();
    seed_sessions(&conn);

    let sessions = db::list_sessions(&conn, None, 100).unwrap();
    assert_eq!(sessions.len(), 3);
}

#[test]
fn test_session_list_project_filter() {
    let (conn, _dir) = temp_db();
    seed_sessions(&conn);

    let sessions = db::list_sessions(&conn, Some("website"), 100).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].project_name, "website");
}

#[test]
fn test_session_list_limit() {
    let (conn, _dir) = temp_db();
    seed_sessions(&conn);

    let sessions = db::list_sessions(&conn, None, 2).unwrap();
    assert_eq!(sessions.len(), 2);
}

#[test]
fn test_session_sync_mark() {
    let (conn, _dir) = temp_db();
    let id = seed_session(&conn, "proj", "2026-07-15T10:00:00Z", Some("2026-07-15T11:00:00Z"));

    let unsynced = db::get_unsynced_sessions(&conn).unwrap();
    assert_eq!(unsynced.len(), 1);
    assert_eq!(unsynced[0].id.to_string(), id);

    db::mark_synced(&conn).unwrap();
    let unsynced = db::get_unsynced_sessions(&conn).unwrap();
    assert_eq!(unsynced.len(), 0);
}

#[test]
fn test_stats_by_project() {
    let (conn, _dir) = temp_db();
    seed_sessions(&conn);

    let stats = db::stats_by_project(&conn).unwrap();
    assert!(stats.iter().any(|s| s.project == "nerdtime" && s.session_count == 2));
    assert!(stats.iter().any(|s| s.project == "website" && s.session_count == 1));
}
