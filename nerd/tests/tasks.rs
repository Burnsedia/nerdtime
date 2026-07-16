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

fn seed_session(conn: &Connection, project: &str, started: &str, ended: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO sessions (id, project_name, started_at, ended_at, is_synced) VALUES (?1, ?2, ?3, ?4, 0)",
        rusqlite::params![id, project, started, ended],
    )
    .unwrap();
    id
}

fn seed_task(conn: &Connection, project: &str, title: &str, quadrant: u8, estimate: Option<i64>) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO tasks (id, project_name, title, estimated_seconds, urgency, importance, quadrant, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8)",
        rusqlite::params![id, project, title, estimate, 3, 3, quadrant, "2026-07-15T10:00:00Z"],
    )
    .unwrap();
    id
}

#[test]
fn test_task_add() {
    let (conn, _dir) = temp_db();
    let id = db::add_task(&conn, "nerdtime", "Write tests", None, None, 4, 5, None, None, None).unwrap();
    let tasks = db::list_tasks(&conn, None, None).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, id);
    assert_eq!(tasks[0].title, "Write tests");
    assert_eq!(tasks[0].quadrant, 1);
}

#[test]
fn test_task_add_with_estimate() {
    let (conn, _dir) = temp_db();
    db::add_task(&conn, "nerdtime", "Est task", None, Some(7200), 2, 4, None, None, None).unwrap();
    let tasks = db::list_tasks(&conn, None, None).unwrap();
    assert_eq!(tasks[0].estimated_seconds, Some(7200));
    assert_eq!(tasks[0].quadrant, 2);
}

#[test]
fn test_task_complete() {
    let (conn, _dir) = temp_db();
    let id = seed_task(&conn, "nerdtime", "Do thing", 2, None);
    db::complete_task(&conn, &id).unwrap();
    let tasks = db::list_tasks(&conn, None, None).unwrap();
    assert!(tasks[0].completed_at.is_some());
}

#[test]
fn test_task_cancel() {
    let (conn, _dir) = temp_db();
    let id = seed_task(&conn, "nerdtime", "Cancel me", 3, None);
    db::cancel_task(&conn, &id).unwrap();
    let tasks = db::list_tasks(&conn, None, Some("cancelled")).unwrap();
    assert_eq!(tasks.len(), 1);
}

#[test]
fn test_task_list_filter_active() {
    let (conn, _dir) = temp_db();
    seed_task(&conn, "nerdtime", "Task 1", 1, None);
    seed_task(&conn, "nerdtime", "Task 2", 2, Some(3600));
    let active = db::list_tasks(&conn, None, Some("active")).unwrap();
    assert_eq!(active.len(), 2);
}

#[test]
fn test_task_list_project_filter() {
    let (conn, _dir) = temp_db();
    seed_task(&conn, "website", "Web task", 2, None);
    seed_task(&conn, "nerdtime", "Nerd task", 1, None);
    let tasks = db::list_tasks(&conn, Some("nerdtime"), None).unwrap();
    assert_eq!(tasks.len(), 1);
}

#[test]
fn test_task_edit() {
    let (conn, _dir) = temp_db();
    let id = seed_task(&conn, "nerdtime", "Original", 2, None);
    db::edit_task(&conn, &id, Some("Updated"), Some(Some(3600i64)), Some(5), Some(5), None).unwrap();
    let tasks = db::list_tasks(&conn, None, None).unwrap();
    assert_eq!(tasks[0].title, "Updated");
    assert_eq!(tasks[0].estimated_seconds, Some(3600));
}

#[test]
fn test_advisor_decide() {
    let (conn, _dir) = temp_db();
    seed_task(&conn, "nerdtime", "Q1 task", 1, Some(3600));
    let input = db::AdvisorInput {
        available_seconds: 7200,
        energy: "high".to_string(),
        blocked: None,
    };
    let advice = db::decide(&conn, &input).unwrap();
    assert!(!advice.task_title.is_empty());
    assert!(!advice.reason.is_empty());
}

#[test]
fn test_advisor_no_tasks() {
    let (conn, _dir) = temp_db();
    let input = db::AdvisorInput {
        available_seconds: 3600,
        energy: "low".to_string(),
        blocked: None,
    };
    let advice = db::decide(&conn, &input).unwrap();
    assert_eq!(advice.task_title, "Take a break");
}

#[test]
fn test_label_summary() {
    let (conn, _dir) = temp_db();
    let labels = r#"["bug","frontend"]"#;
    db::start_session(&conn, "website", None, None, None, Some(labels)).unwrap();
    db::stop_session(&conn).unwrap();
    let rows = db::label_summary(&conn, None, None, "2026-01-01", "2099-12-31").unwrap();
    assert!(!rows.is_empty());
}
