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
pub struct DevlogLogInput {
    pub text: String,
    pub tags: Option<String>,
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DevlogQueryInput {
    pub text: Option<String>,
    pub tags: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DevlogGenerateInput {
    pub output_path: Option<String>,
}

pub fn handle_devlog_log(
    state: &AppState,
    text: String,
    tags: Option<String>,
    project: Option<String>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;

    let now = chrono::Utc::now().to_rfc3339();
    let entry = nerdtime_core::DevlogEntry {
        id: uuid::Uuid::new_v4().to_string(),
        date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        title: text.clone(),
        role: "developer".to_string(),
        tags: tags
            .as_deref()
            .map(|s| s.split(',').map(|t| t.trim().to_string()).collect())
            .unwrap_or_default(),
        context: project.unwrap_or_default(),
        changes: Vec::new(),
        decisions: Vec::new(),
        commits: Vec::new(),
        session_id: None,
        created_at: now,
    };

    nerdtime_db::insert_devlog_entry(&conn, &entry).map_err(wrap_err)?;

    ok(format!("Logged devlog entry ({})", entry.id))
}

pub fn handle_devlog_query(
    state: &AppState,
    text: Option<String>,
    tags: Option<String>,
    _limit: Option<i32>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;

    let tags_str = tags.as_deref();

    let entries =
        nerdtime_db::search_devlog_entries(&conn, text.as_deref().unwrap_or(""), tags_str)
            .map_err(wrap_err)?;

    let count = entries.len().min(10);

    let mut lines = Vec::new();
    for e in &entries[..count] {
        let tags_str = e.tags.join(", ");
        lines.push(format!("{} | {} | {}", e.created_at, e.title, tags_str));
    }
    if lines.is_empty() {
        return ok("No devlog entries found".to_string());
    }
    ok(lines.join("\n"))
}

pub fn handle_devlog_generate(
    state: &AppState,
    output_path: Option<String>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;

    let markdown = nerdtime_db::render_devlog_md(&conn).map_err(wrap_err)?;
    let path = output_path.unwrap_or_else(|| "./DEVLOG.md".to_string());

    let line_count = markdown.lines().count();
    std::fs::write(&path, &markdown).map_err(|e| {
        ErrorData::internal_error(format!("failed to write {path}: {e}"), None)
    })?;

    ok(format!("DEVLOG.md written to {path} ({line_count} lines)"))
}
