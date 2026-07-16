use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::ErrorData;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::AppState;

fn ok(text: String) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

fn err(text: String) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::error(vec![ContentBlock::text(text)]))
}

fn wrap_err(e: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(format!("{e}"), None)
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StartTrackingInput {
    pub project: String,
    pub task_id: Option<String>,
    pub estimate: Option<String>,
    pub label: Option<String>,
}

pub fn handle_start_tracking(
    state: &AppState,
    project: String,
    task_id: Option<String>,
    estimate: Option<String>,
    label: Option<String>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;

    let estimated_seconds = estimate
        .as_deref()
        .and_then(|s| nerdtime_db::parse_duration(s).ok())
        .flatten();
    let labels = label.as_deref();

    nerdtime_db::start_session(
        &conn,
        &project,
        None,
        task_id.as_deref(),
        estimated_seconds,
        labels,
    )
    .map_err(wrap_err)?;

    ok(format!("Started tracking project '{project}'"))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListSessionsInput {
    pub project: Option<String>,
    pub limit: Option<i32>,
}

pub fn handle_list_sessions(
    state: &AppState,
    project: Option<String>,
    limit: Option<i32>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;
    let limit = limit.unwrap_or(20) as usize;
    let sessions =
        nerdtime_db::list_sessions(&conn, project.as_deref(), limit).map_err(wrap_err)?;

    let mut lines = Vec::new();
    for s in &sessions {
        lines.push(format!("{} | {} | {}", s.id, s.project_name, s.started_at));
    }
    ok(lines.join("\n"))
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatsInput {
    pub days: Option<i32>,
    pub project: Option<String>,
}

pub fn handle_get_stats(
    state: &AppState,
    _days: Option<i32>,
    _project: Option<String>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;
    let stats = nerdtime_db::stats_by_project(&conn).map_err(wrap_err)?;

    let mut lines = Vec::new();
    for s in &stats {
        lines.push(format!("{}: {}", s.project, s.total_seconds));
    }
    ok(lines.join("\n"))
}

pub fn handle_sync(state: &AppState) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;

    let api_url = state
        .config
        .api_url
        .as_deref()
        .unwrap_or("http://localhost:5150");
    let token = state.config.token.as_deref().unwrap_or("");

    let sessions = nerdtime_db::get_unsynced_sessions(&conn).map_err(wrap_err)?;

    if sessions.is_empty() {
        return ok("Nothing to sync".to_string());
    }

    let payload: Vec<nerdtime_core::SyncPayload> = sessions
        .iter()
        .map(|s| nerdtime_core::SyncPayload {
            id: s.id,
            project_name: s.project_name.clone(),
            branch_name: s.branch_name.clone(),
            commit_hash: s.commit_hash.clone(),
            description: s.description.clone(),
            started_at: s.started_at,
            ended_at: s.ended_at,
            task_id: s.task_id.clone(),
            estimated_seconds: s.estimated_seconds,
            labels: s.labels.clone(),
        })
        .collect();

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/api/sync", api_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&payload)
        .send();

    match resp {
        Ok(_) => {
            let _ = nerdtime_db::mark_synced(&conn);
            ok(format!("Synced {} sessions", sessions.len()))
        }
        Err(e) => err(format!("Sync failed: {e}")),
    }
}
