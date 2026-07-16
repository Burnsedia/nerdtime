use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::ErrorData;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::state::AppState;

fn wrap_err(e: impl std::fmt::Display) -> ErrorData {
    ErrorData::internal_error(format!("{e}"), None)
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AdvisorInput {
    pub time: Option<String>,
    pub energy: Option<String>,
}

pub fn handle_advisor(
    state: &AppState,
    time: Option<String>,
    energy: Option<String>,
) -> Result<CallToolResult, ErrorData> {
    let conn = state.conn.lock().map_err(wrap_err)?;

    let available_seconds = time
        .as_deref()
        .and_then(|s| nerdtime_db::parse_duration(s).ok())
        .flatten()
        .unwrap_or(3600);

    let input = nerdtime_core::AdvisorInput {
        available_seconds,
        energy: energy.unwrap_or_else(|| "medium".to_string()),
        blocked: None,
    };

    let advice = nerdtime_db::decide(&conn, &input).map_err(wrap_err)?;

    let result = serde_json::to_string_pretty(&advice).unwrap_or_else(|_| "No advice".to_string());
    Ok(CallToolResult::success(vec![ContentBlock::text(result)]))
}
