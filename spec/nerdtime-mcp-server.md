# nerdtime MCP Server — Implementation Plan

> **Pricing**: Free (AGPL). Part of the free CLI tier.
> **Platforms**: Linux, macOS (no Windows planned).

## Overview

An MCP (Model Context Protocol) server that exposes nerdtime's time-tracking functionality as tools for AI coding agents (Cline, Claude Code, Cursor, etc.) and agent frameworks (Hermes, etc.).

The server communicates via **stdio transport** — the standard MCP transport for local tools. AI clients launch `nerdtime-mcp` as a subprocess and call tools via JSON-RPC over stdin/stdout.

## Architecture

```
┌──────────────────────────────────────────────────┐
│  AI Coding Agent / Hermes                        │
│  (Claude Desktop, Cline, Cursor, Claude Code)    │
│         │                                        │
│         │ spawns subprocess, JSON-RPC over stdio │
│         ▼                                        │
│  ┌─────────────────────────────────────────────┐ │
│  │            nerdtime-mcp                     │ │
│  │  ┌──────────────────────────────────────┐   │ │
│  │  │  rmcp (MCP SDK)                      │   │ │
│  │  │  • stdio transport                   │   │ │
│  │  │  • tool registration                 │   │ │
│  │  │  • capability negotiation            │   │ │
│  │  └──────────────┬───────────────────────┘   │ │
│  │                 │                            │ │
│  │  ┌──────────────▼───────────────────────┐   │ │
│  │  │  Tool Handlers                       │   │ │
│  │  │  start / stop / status / list / stats│   │ │
│  │  └──────────────┬───────────────────────┘   │ │
│  │                 │                            │ │
│  │  ┌──────────────▼───────────────────────┐   │ │
│  │  │     nerdtime-db (shared crate)       │   │ │
│  │  │  SQLite operations (sync rusqlite)   │   │ │
│  │  └─────────────────────────────────────┘   │ │
│  │                                             │ │
│  │  Data: ~/.config/nerdtime/data.db            │ │
│  │  Config: ~/.config/nerdtime/config.toml     │ │
│  └─────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

## New workspace member: `nerdtime-mcp`

### Files

```
nerdtime-mcp/
├── Cargo.toml          # deps: rmcp (server + transport-io), nerdtime-db, nerdtime-core, serde, tokio
├── src/
│   ├── main.rs         # Entrypoint: create MCP server with stdio transport, register tools
│   ├── tools/
│   │   ├── mod.rs      # Re-exports all tool handlers
│   │   ├── start.rs    # start_tracking tool
│   │   ├── stop.rs     # stop_tracking tool
│   │   ├── status.rs   # get_status tool
│   │   ├── list.rs     # list_sessions tool
│   │   ├── stats.rs    # get_stats tool
│   │   ├── sync.rs     # sync tool
│   │   ├── task.rs     # task_create, task_list, task_matrix, task_complete, task_edit tools
│   │   ├── devlog.rs   # devlog_log_session, devlog_query, devlog_get_decisions tools
│   │   └── suggest.rs  # what_should_i_work_on tool
│   └── state.rs        # AppState: db connection, config
```

### Dependencies (`Cargo.toml`)

```toml
[package]
name = "nerdtime-mcp"
version = "0.1.0"
edition = "2021"
license = "AGPL-3.0-only"

[dependencies]
rmcp = { version = "0.1", features = ["server", "transport-io"] }
nerdtime-db = { path = "../nerdtime-db" }
nerdtime-core = { path = "../nerdtime-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## MCP Tools

### 1. `start_tracking`

Start tracking time for a project.

| Property | Value |
|----------|-------|
| **name** | `start_tracking` |
| **description** | Start a time-tracking session for a project. Detects git branch and commit hash if run from a git repo. |
| **inputSchema** | `{ project: string (required), description?: string (optional) }` |
| **output** | `{ id: string, project: string, started_at: string, branch?: string, commit?: string }` |

**Implementation:**
```rust
#[tool(description = "Start tracking time for a project")]
async fn start_tracking(
    #[tool(description = "Project name to track")]
    project: String,
    #[tool(description = "Optional description for this session")]
    description: Option<String>,
) -> Result<ToolStartResponse, McpError> {
    let conn = state.db.lock().unwrap();
    let branch = detect_git_branch();
    let commit = detect_git_commit();
    let session = nerdtime_db::start_session(&conn, &project, description.as_deref(), branch, commit)?;
    Ok(ToolStartResponse { id: session.id, project, started_at: session.started_at, branch, commit })
}
```

### 2. `stop_tracking`

Stop the currently active session.

| Property | Value |
|----------|-------|
| **name** | `stop_tracking` |
| **description** | Stop the currently active time-tracking session. Returns the elapsed duration. |
| **inputSchema** | `{}` (no params) |
| **output** | `{ id: string, project: string, started_at: string, ended_at: string, duration_seconds: number }` |

### 3. `get_status`

Get info about the currently active session.

| Property | Value |
|----------|-------|
| **name** | `get_status` |
| **description** | Show the currently active session (if any) with elapsed time. |
| **inputSchema** | `{}` |
| **output** | `{ active: bool, project?: string, started_at?: string, elapsed_seconds?: number }` |

### 4. `list_sessions`

List recent time-tracking sessions.

| Property | Value |
|----------|-------|
| **name** | `list_sessions` |
| **description** | List time-tracking sessions, optionally filtered by project. |
| **inputSchema** | `{ project?: string (optional), limit?: number (optional, default 10) }` |
| **output** | `{ sessions: [{ id, project, started_at, ended_at?, description?, duration_seconds? }] }` |

### 5. `get_stats`

Get aggregate time per project.

| Property | Value |
|----------|-------|
| **name** | `get_stats` |
| **description** | Get total time tracked per project. |
| **inputSchema** | `{}` |
| **output** | `{ stats: [{ project: string, total_seconds: number, session_count: number }] }` |

### 6. `sync`

Sync unsynced sessions to the API backend.

| Property | Value |
|----------|-------|
| **name** | `sync` |
| **description** | Push unsynced (completed) time-tracking sessions to the API backend. Requires api_url and token to be configured. |
| **inputSchema** | `{}` |
| **output** | `{ synced: number, failed: number, message: string }` |

### 7. `task_create`

Create a task with Eisenhower Matrix prioritization.

| Property | Value |
|----------|-------|
| **name** | `task_create` |
| **description** | Create a new task with optional Eisenhower Matrix urgency/importance ratings. |
| **inputSchema** | `{ project: string, title: string, description?: string, estimate?: string, urgency?: number, importance?: number, quadrant?: number }` |
| **output** | `{ id: string, project: string, title: string, urgency?: number, importance?: number }` |

If neither `urgency`/`importance` nor `quadrant` is provided, defaults to 3/3.

### 8. `task_list`

List tasks, optionally filtered.

| Property | Value |
|----------|-------|
| **name** | `task_list` |
| **description** | List tasks, optionally filtered by project, status, or Eisenhower quadrant. |
| **inputSchema** | `{ project?: string, status?: string, quadrant?: number }` |
| **output** | `{ tasks: [{ id, project, title, status, urgency?, importance?, estimated_seconds?, actual_seconds? }] }` |

### 9. `task_matrix`

Get the Eisenhower Matrix view of all tasks.

| Property | Value |
|----------|-------|
| **name** | `task_matrix` |
| **description** | Get tasks organized by Eisenhower quadrant (Do First, Schedule, Delegate, Eliminate). |
| **inputSchema** | `{ project?: string }` |
| **output** | `{ q1: Task[], q2: Task[], q3: Task[], q4: Task[], unprioritized: Task[] }` |

### 10. `task_complete`

Mark a task as completed.

| Property | Value |
|----------|-------|
| **name** | `task_complete` |
| **description** | Mark a task as completed. |
| **inputSchema** | `{ task_id: string }` |
| **output** | `{ id: string, status: string, completed_at: string }` |

### 11. `task_edit`

Edit a task's fields.

| Property | Value |
|----------|-------|
| **name** | `task_edit` |
| **description** | Edit a task's title, estimate, urgency, or importance. |
| **inputSchema** | `{ task_id: string, title?: string, estimate?: string, urgency?: number, importance?: number, quadrant?: number }` |
| **output** | `{ id: string, title: string, urgency?: number, importance?: number }` |

### 12. `what_should_i_work_on`

Get a deterministic work recommendation based on available time and energy.

| Property | Value |
|----------|-------|
| **name** | `what_should_i_work_on` |
| **description** | Analyze open tasks and recommend what to work on based on available time, energy level, and Eisenhower priority. Deterministic — no LLM involved. |
| **inputSchema** | `{ time_minutes?: number, energy?: "low" | "medium" | "high", blocked?: boolean }` |
| **output** | `{ task?: { id, title, project, urgency, importance, quadrant, estimated_seconds }, reason: string, alternatives: Task[], fitting_tasks: Task[], oversized_tasks: Task[] }` |

Decision tree:
1. Filter tasks by quadrant (Q1 → Q2 → Q3 → Q4)
2. Prune by available time (task estimate > time available? → oversized)
3. If blocked, deprioritize blocked tasks
4. Energy: low → favor Q3/Q4 quick wins; medium → Q1; high → Q2 strategic
5. Return top candidate + alternatives

### 13. `devlog_log_session`

Log a development session entry.

| Property | Value |
|----------|-------|
| **name** | `devlog_log_session` |
| **description** | Log a structured development session entry with context, decisions, and author attribution. Called by AI agents after completing a task batch. |
| **inputSchema** | `{ title: string, role: "human" | "ai" | "hybrid", tags?: string[], context?: string, changes?: string[], decisions?: string[], commits?: string[] }` |
| **output** | `{ id: string, date: string, title: string }` |

### 14. `devlog_query`

Search development log entries.

| Property | Value |
|----------|-------|
| **name** | `devlog_query` |
| **description** | Search DEVLOG entries by keyword or tag. Uses SQLite FTS5 for full-text search. |
| **inputSchema** | `{ query: string, tags?: string[], limit?: number }` |
| **output** | `{ entries: [{ id, date, title, role, tags, snippet }] }` |

### 15. `devlog_get_decisions`

Retrieve all logged technical decisions.

| Property | Value |
|----------|-------|
| **name** | `devlog_get_decisions` |
| **description** | Get all technical decisions from DEVLOG entries, optionally filtered by tag. |
| **inputSchema** | `{ tag?: string }` |
| **output** | `{ decisions: [{ date, title, decisions: string[], tags: string[] }] }` |

## MCP Resources (optional enhancement)

Read-only data that agents can access without calling tools:

| Resource URI | Description | Returns |
|-------------|-------------|---------|
| `nerdtime://status` | Current tracking status | JSON with active session or null |
| `nerdtime://sessions/recent?limit=5` | Recent sessions | JSON array of sessions |
| `nerdtime://stats` | Time per project | JSON array of project stats |
| `nerdtime://tasks/matrix` | Eisenhower Matrix view | JSON with q1-q4 arrays |
| `nerdtime://devlog/recent?limit=5` | Recent devlog entries | JSON array |
| `nerdtime://devlog/decisions` | All technical decisions | JSON array of decisions |

## Entrypoint (`main.rs`)

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("nerdtime_mcp=info")
        .with_writer(std::io::stderr)
        .init();

    let state = AppState::new()?;

    let service = (
        start_tracking_tool(),
        stop_tracking_tool(),
        get_status_tool(),
        list_sessions_tool(),
        get_stats_tool(),
        sync_tool(),
        task_create_tool(),
        task_list_tool(),
        task_matrix_tool(),
        task_complete_tool(),
        task_edit_tool(),
        what_should_i_work_on_tool(),
        devlog_log_session_tool(),
        devlog_query_tool(),
        devlog_get_decisions_tool(),
    );

    let server = Server::new(service)
        .with_state(state);

    let handler = server.into_handler();
    let transport = StdioTransport::new(handler);
    transport.start().await?;

    Ok(())
}
```

## Config

Reuses the same config mechanism as the CLI:

- **Config file**: `~/.config/nerdtime/config.toml`
- **Fields**: `api_url` (string), `token` (optional string)
- **Database**: `~/.config/nerdtime/data.db`

Loaded via `nerdtime-db` or directly via the existing `config.rs` patterns.

## Integration with AI Clients

### Cline (VS Code extension)

Add to Cline's MCP config (`~/.config/cline/mcp.json` or VS Code settings):

```json
{
  "mcpServers": {
    "nerdtime": {
      "command": "nerdtime-mcp",
      "args": [],
      "env": {}
    }
  }
}
```

### Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "nerdtime": {
      "command": "nerdtime-mcp",
      "args": []
    }
  }
}
```

### Claude Code

```bash
claude mcp add nerdtime -- nerdtime-mcp
```

### Cursor

Via Cursor's MCP configuration (`.cursor/mcp.json` in project or global settings).

## MCP Client (optional)

For programmatic control from Rust applications (Hermes, custom agents, etc.), a lightweight client library can be provided:

**File:** `nerdtime-mcp/src/client.rs` (optional feature-gated)

```rust
pub struct NerdtimeClient {
    process: Child,
    transport: ChildProcessTransport,
}

impl NerdtimeClient {
    pub async fn new() -> Result<Self>;
    pub async fn start_tracking(&mut self, project: &str, desc: Option<&str>) -> Result<StartResponse>;
    pub async fn stop_tracking(&mut self) -> Result<StopResponse>;
    pub async fn get_status(&mut self) -> Result<StatusResponse>;
    pub async fn list_sessions(&mut self, project: Option<&str>, limit: Option<usize>) -> Result<Vec<Session>>;
    pub async fn get_stats(&mut self) -> Result<Vec<ProjectStat>>;
    pub async fn sync(&mut self) -> Result<SyncResponse>;
}
```

This would be behind a `client` feature flag in `nerdtime-mcp/Cargo.toml`:

```toml
[features]
default = ["server"]
server = ["rmcp/server", "rmcp/transport-io"]
client = ["rmcp/client", "rmcp/transport-child-process"]
```

## nerdtime-db additions

The `nerdtime-db` crate needs the following additions (from the existing CLI's `db.rs`):

- `start_session` with optional branch/commit params (for git detection in MCP server)
- `detect_git_branch()` and `detect_git_commit()` helper functions
- Session response types that include `duration_seconds` and elapsed time calculations
- Sync payload construction and API sync function (async, using `reqwest`)

These already exist in the CLI's `db.rs` — the extraction plan in `spec/tauri-mobile-app-plan.md` covers this.

## Files affected

### New files (9)

| File | Purpose |
|------|---------|
| `nerdtime-mcp/Cargo.toml` | Dependencies |
| `nerdtime-mcp/src/main.rs` | Entrypoint, server setup |
| `nerdtime-mcp/src/state.rs` | AppState with DB + config |
| `nerdtime-mcp/src/tools/mod.rs` | Tool re-exports |
| `nerdtime-mcp/src/tools/start.rs` | start_tracking handler |
| `nerdtime-mcp/src/tools/stop.rs` | stop_tracking handler |
| `nerdtime-mcp/src/tools/status.rs` | get_status handler |
| `nerdtime-mcp/src/tools/list.rs` | list_sessions handler |
| `nerdtime-mcp/src/tools/stats.rs` | get_stats handler |
| `nerdtime-mcp/src/tools/sync.rs` | sync handler |
| `nerdtime-mcp/src/tools/task.rs` | task_create, task_list, task_matrix, task_complete, task_edit handlers |
| `nerdtime-mcp/src/tools/devlog.rs` | devlog_log_session, devlog_query, devlog_get_decisions handlers |
| `nerdtime-mcp/src/tools/suggest.rs` | what_should_i_work_on handler |

### Modified files

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `nerdtime-mcp` member |
| `nerdtime-db/src/lib.rs` | Add `detect_git_branch`, `detect_git_commit` helpers |
| `AGENTS.md` | Add MCP server section with tool docs and client config examples |
| `DEVLOG.md` | Document this session |
| `.env.example` | No changes needed (no new env vars) |

## Estimated effort

| Phase | Time |
|-------|------|
| Scaffold `nerdtime-mcp` crate + deps | 30 min |
| Implement session tool handlers (start/stop/status/list/stats/sync) | 2-3 hours |
| Implement task tool handlers (create/list/matrix/complete/edit) | 2 hours |
| Implement devlog tool handlers (log_session/query/get_decisions) | 1 hour |
| Implement what_should_i_work_on handler | 1 hour |
| Add git detection helpers to `nerdtime-db` | 30 min |
| Testing with Cline/Claude Desktop | 1 hour |
| Documentation (AGENTS.md, config examples) | 30 min |
| **Total** | **~9-10 hours** |

## Edge cases & notes

- **No active session**: `stop_tracking` and `get_status` must handle the case gracefully (return `{ active: false }` or error)
- **Git detection**: Runs `git branch --show-current` and `git rev-parse HEAD`; fails silently outside git repos (same as CLI)
- **Config missing**: If no `api_url` is configured, `sync` returns a clear error telling the user to run `nerd login`
- **DB locking**: SQLite via `rusqlite` inside a `Mutex` — adequate for single-process access. If the CLI runs concurrently, use SQLite WAL mode
- **Logging**: All server logs go to stderr (never stdout), per MCP stdio spec
- **Error format**: Tool errors return structured `McpError` with `is_handled: true` so the AI sees a clear message
- **Security**: The MCP server has access to the local SQLite database and config. It inherits the user's permissions. No network exposure (stdio only)
- **Task fallbacks**: `task_create` with no urgency/importance defaults to 3/3 (center of matrix)
- **Devlog FTS5**: Create FTS5 virtual table on `devlog_entries` for fast `devlog_query`
- **Author attribution**: `devlog_log_session` called by AI agents sets `role: "ai"`; human CLI prompts default to `"human"`
