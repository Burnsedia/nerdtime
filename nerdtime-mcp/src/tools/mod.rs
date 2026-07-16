mod advisor;
mod devlog;
mod sessions;
mod tasks;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::tool;
use rmcp::tool_router;

use crate::state::AppState;

fn ok(text: String) -> Result<CallToolResult, rmcp::ErrorData> {
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

fn err(text: String) -> Result<CallToolResult, rmcp::ErrorData> {
    Ok(CallToolResult::error(vec![ContentBlock::text(text)]))
}

fn wrap_err(e: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error(format!("{e}"), None)
}

#[tool_router(server_handler)]
impl AppState {
    #[tool(description = "Start tracking time for a project")]
    async fn start_tracking(
        &self,
        Parameters(input): Parameters<sessions::StartTrackingInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        sessions::handle_start_tracking(
            self,
            input.project,
            input.task_id,
            input.estimate,
            input.label,
        )
    }

    #[tool(description = "Stop the active session")]
    async fn stop_tracking(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let conn = self.conn.lock().map_err(wrap_err)?;
        let existing = nerdtime_db::show_status(&conn).map_err(wrap_err)?;
        match existing {
            Some(session) => {
                nerdtime_db::stop_session(&conn).map_err(wrap_err)?;
                ok(format!(
                    "Stopped session for project '{}'",
                    session.project_name
                ))
            }
            None => err("No active session".to_string()),
        }
    }

    #[tool(description = "Show the active session")]
    async fn get_status(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let conn = self.conn.lock().map_err(wrap_err)?;
        let active = nerdtime_db::show_status(&conn).map_err(wrap_err)?;
        match active {
            Some(session) => ok(format!(
                "Active session: project='{}' started={}",
                session.project_name, session.started_at
            )),
            None => ok("No active session".to_string()),
        }
    }

    #[tool(description = "List sessions, optionally filtered by project")]
    async fn list_sessions(
        &self,
        Parameters(input): Parameters<sessions::ListSessionsInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        sessions::handle_list_sessions(self, input.project, input.limit)
    }

    #[tool(description = "Get aggregated time stats per project")]
    async fn get_stats(
        &self,
        Parameters(input): Parameters<sessions::StatsInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        sessions::handle_get_stats(self, input.days, input.project)
    }

    #[tool(description = "Sync local sessions to the nerdtime API server")]
    async fn sync(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        sessions::handle_sync(self)
    }

    #[tool(description = "List tasks, optionally filtered by project")]
    async fn task_list(
        &self,
        Parameters(input): Parameters<tasks::TaskListInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        tasks::handle_task_list(self, input.project)
    }

    #[tool(description = "Create a new task with optional quadrant and estimate")]
    async fn task_create(
        &self,
        Parameters(input): Parameters<tasks::TaskCreateInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        tasks::handle_task_create(
            self,
            input.project,
            input.title,
            input.quadrant,
            input.estimate,
        )
    }

    #[tool(description = "Show the Eisenhower quadrant view of tasks")]
    async fn task_matrix(
        &self,
        Parameters(input): Parameters<tasks::TaskMatrixInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        tasks::handle_task_matrix(self, input.project)
    }

    #[tool(description = "Log a devlog entry")]
    async fn devlog_log(
        &self,
        Parameters(input): Parameters<devlog::DevlogLogInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        devlog::handle_devlog_log(self, input.text, input.tags, input.project)
    }

    #[tool(description = "Search devlog entries")]
    async fn devlog_query(
        &self,
        Parameters(input): Parameters<devlog::DevlogQueryInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        devlog::handle_devlog_query(self, input.text, input.tags, input.limit)
    }

    #[tool(description = "Render all devlog entries to a DEVLOG.md file")]
    async fn devlog_generate(
        &self,
        Parameters(input): Parameters<devlog::DevlogGenerateInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        devlog::handle_devlog_generate(self, input.output_path)
    }

    #[tool(description = "Get advice on what to work on")]
    async fn what_should_i_work_on(
        &self,
        Parameters(input): Parameters<advisor::AdvisorInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        advisor::handle_advisor(self, input.time, input.energy)
    }
}
