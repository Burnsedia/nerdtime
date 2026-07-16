use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::ErrorData;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::AppState;

fn ok(text: String) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

fn wrap_err(e: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(format!("{e}"), None)
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskListInput {
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskCreateInput {
    pub project: String,
    pub title: String,
    pub quadrant: Option<String>,
    pub estimate: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskMatrixInput {
    pub project: Option<String>,
}

pub fn handle_task_list(
    state: &AppState,
    project: Option<String>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;
    let tasks = nerdtime_db::list_tasks(&conn, project.as_deref(), None).map_err(wrap_err)?;

    let mut lines = Vec::new();
    for t in &tasks {
        lines.push(format!("[{}] {} — {}", t.id, t.title, t.project_name));
    }
    if lines.is_empty() {
        return ok("No tasks found".to_string());
    }
    ok(lines.join("\n"))
}

pub fn handle_task_create(
    state: &AppState,
    project: String,
    title: String,
    quadrant: Option<String>,
    estimate: Option<String>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;

    let (urgency, importance) = match quadrant.as_deref() {
        Some("q1") => (4, 4),
        Some("q2") => (2, 4),
        Some("q3") => (4, 2),
        Some("q4") => (2, 2),
        _ => (1, 1),
    };

    let estimated_seconds = estimate
        .as_deref()
        .and_then(|s| nerdtime_db::parse_duration(s).ok())
        .flatten();

    nerdtime_db::add_task(
        &conn,
        &project,
        &title,
        None,
        estimated_seconds,
        urgency,
        importance,
        None,
        None,
        None,
    )
    .map_err(wrap_err)?;

    ok(format!("Created task '{title}'"))
}

pub fn handle_task_matrix(
    state: &AppState,
    project: Option<String>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;
    let tasks = nerdtime_db::list_tasks(&conn, project.as_deref(), None).map_err(wrap_err)?;

    let mut q1 = Vec::new();
    let mut q2 = Vec::new();
    let mut q3 = Vec::new();
    let mut q4 = Vec::new();

    for t in &tasks {
        match t.quadrant {
            1 => q1.push(t.title.as_str()),
            2 => q2.push(t.title.as_str()),
            3 => q3.push(t.title.as_str()),
            4 => q4.push(t.title.as_str()),
            _ => {}
        }
    }

    let lines = [
        "=== Eisenhower Matrix ===".to_string(),
        format!("Q1 (Urgent & Important): {}", q1.join(", ")),
        format!("Q2 (Not Urgent & Important): {}", q2.join(", ")),
        format!("Q3 (Urgent & Not Important): {}", q3.join(", ")),
        format!("Q4 (Not Urgent & Not Important): {}", q4.join(", ")),
    ];
    ok(lines.join("\n"))
}
