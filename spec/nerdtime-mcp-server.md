# nerdtime MCP Server — Implementation Plan

> **Pricing**: Free (AGPL). Part of the free CLI tier.
> **Platforms**: Linux, macOS (no Windows planned).

## Overview

An MCP (Model Context Protocol) server that exposes nerdtime's full feature set as tools for AI coding agents (Cline, Claude Code, Cursor, OpenCode, etc.). Covers time tracking, tasks with Eisenhower Matrix, devlog, and the what-should-i-work-on advisor.

All tools are **deterministic and cost $0 in tokens** — they read/write local SQLite via stdio. The AI agent pays only to decide when to call them. No network, no API keys, no latency.

The server communicates via **stdio transport** — the standard MCP transport for local tools. AI clients launch `nerdtime-mcp` as a subprocess and call tools via JSON-RPC over stdin/stdout.

## Architecture

```
┌──────────────────────────────────────────────────┐
│  AI Coding Agent / Hermes                        │
│  (Claude Desktop, Cline, Cursor, OpenCode)       │
│         │                                        │
│         │ spawns subprocess, JSON-RPC over stdio │
│         │ $0 per tool call (AI pays for thinking) │
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
│  │  │  task_list / task_create / task_matrix│   │ │
│  │  │  devlog_log / devlog_query            │   │ │
│  │  │  what_should_i_work_on                │   │ │
│  │  └──────────────┬───────────────────────┘   │ │
│  │                 │                            │ │
│  │  ┌──────────────▼───────────────────────┐   │ │
│  │  │  MCP Server Helpers                  │   │ │
│  │  │  • Advisor engine (decision tree)    │   │ │
│  │  │  • Devlog renderer                   │   │ │
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
├── Cargo.toml          # deps: rmcp, nerdtime-db, nerdtime-core, serde, tokio
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
│   │   ├── task_list.rs    # task_list tool
│   │   ├── task_create.rs  # task_create tool
│   │   ├── task_matrix.rs  # task_matrix tool
│   │   ├── devlog_log.rs   # devlog_log_session tool
│   │   ├── devlog_query.rs # devlog_query tool
│   │   └── advisor.rs      # what_should_i_work_on tool
│   ├── advisor.rs      # Decision tree engine (shared with CLI)
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

All tools are deterministic, offline, and cost $0 in token usage. The AI agent pays only to decide when to call them.

### Tracking tools

| # | Tool | Input | Output |
|---|---|---|---|
| 1 | `start_tracking` | `project`, `description?`, `task_id?` | `{ id, project, started_at, branch?, commit? }` |
| 2 | `stop_tracking` | — | `{ id, project, started_at, ended_at, duration_seconds }` |
| 3 | `get_status` | — | `{ active: bool, project?, started_at?, elapsed_seconds? }` |
| 4 | `list_sessions` | `project?`, `limit?` | `{ sessions: [...] }` |
| 5 | `get_stats` | — | `{ stats: [{ project, total_seconds, session_count }] }` |
| 6 | `sync` | — | `{ synced, failed, message }` |

### Task & Matrix tools

| # | Tool | Input | Output |
|---|---|---|---|
| 7 | `task_list` | `project?`, `status?` | `{ tasks: [{ id, title, urgency, importance, quadrant, estimate, actual }] }` |
| 8 | `task_create` | `project`, `title`, `urgency?`, `importance?`, `estimate?`, `labels?` | `{ id, quadrant }` |
| 9 | `task_matrix` | `project?` | `{ q1: [...], q2: [...], q3: [...], q4: [...] }` — tasks grouped by quadrant |

### Devlog tools

| # | Tool | Input | Output |
|---|---|---|---|
| 10 | `devlog_log_session` | `title`, `role`, `tags?`, `context?`, `changes?`, `decisions?` | `{ id, date }` |
| 11 | `devlog_query` | `query`, `tags?`, `limit?` | `{ entries: [{ date, title, role, tags, snippet }] }` |

### Advisor tool

| # | Tool | Input | Output |
|---|---|---|---|
| 12 | `what_should_i_work_on` | `available_minutes`, `energy_level?`, `blocked?` | `{ suggestion, reasoning, tasks: [...] }` |

### 1. `start_tracking`

Start tracking time for a project, optionally linked to a task.

| Property | Value |
|----------|-------|
| **name** | `start_tracking` |
| **description** | Start a time-tracking session for a project. Detects git branch and commit hash. Optionally link to a task. |
| **inputSchema** | `{ project: string (required), description?: string, task_id?: string }` |
| **output** | `{ id: string, project: string, started_at: string, branch?: string, commit?: string }` |

### 2. `stop_tracking`

Stop the currently active session.

| Property | Value |
|----------|-------|
| **name** | `stop_tracking` |
| **description** | Stop the currently active time-tracking session. Returns elapsed duration. |
| **inputSchema** | `{}` |
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
| **inputSchema** | `{ project?: string, limit?: number (default 10) }` |
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
| **description** | Push unsynced (completed) sessions to the API backend. Requires api_url and token. |
| **inputSchema** | `{}` |
| **output** | `{ synced: number, failed: number, message: string }` |

### 7. `task_list`

List tasks with Eisenhower Matrix data.

| Property | Value |
|----------|-------|
| **name** | `task_list` |
| **description** | List tasks, optionally filtered by project and status. Returns urgency, importance, quadrant, estimates. |
| **inputSchema** | `{ project?: string, status?: string }` |
| **output** | `{ tasks: [{ id, project, title, urgency, importance, quadrant, estimated_seconds, actual_seconds, status }] }` |

### 8. `task_create`

Create a task with optional Eisenhower Matrix placement.

| Property | Value |
|----------|-------|
| **name** | `task_create` |
| **description** | Create a new task. If urgency/importance not provided, defaults to Q4. |
| **inputSchema** | `{ project: string, title: string, urgency?: number (1-5), importance?: number (1-5), estimate?: string, labels?: string[] }` |
| **output** | `{ id: string, quadrant: number }` |

### 9. `task_matrix`

Get tasks organized by Eisenhower quadrant.

| Property | Value |
|----------|-------|
| **name** | `task_matrix` |
| **description** | Return active tasks grouped by Eisenhower quadrant (Q1: do first, Q2: schedule, Q3: delegate, Q4: eliminate). |
| **inputSchema** | `{ project?: string }` |
| **output** | `{ q1: [...], q2: [...], q3: [...], q4: [...] }` |

### 10. `devlog_log_session`

Log a development session entry.

| Property | Value |
|----------|-------|
| **name** | `devlog_log_session` |
| **description** | Append a structured entry to the development log. AI agents should call this after completing a task batch with `role: "ai"`. |
| **inputSchema** | `{ title: string, role: "human" | "ai" | "hybrid", tags?: string[], context?: string, changes?: string[], decisions?: string[] }` |
| **output** | `{ id: string, date: string }` |

### 11. `devlog_query`

Search the development log.

| Property | Value |
|----------|-------|
| **name** | `devlog_query` |
| **description** | Search past devlog entries by keyword or tags. Useful for remembering past decisions, context, and reasoning. |
| **inputSchema** | `{ query: string, tags?: string[], limit?: number (default 5) }` |
| **output** | `{ entries: [{ date, title, role, tags, snippet }] }` |

### 12. `what_should_i_work_on`

Get a deterministic suggestion for what to work on next.

| Property | Value |
|----------|-------|
| **name** | `what_should_i_work_on` |
| **description** | Analyze open tasks and suggest what to work on based on Eisenhower priority, available time, energy level, and blockers. Deterministic — no LLM, no API calls, $0. |
| **inputSchema** | `{ available_minutes: number, energy_level?: "low" | "medium" | "high", blocked?: string }` |
| **output** | `{ suggestion: string, reasoning: string, tasks: [{ id, title, quadrant, estimated_minutes }] }` |

## MCP Resources (optional enhancement)

Read-only data that agents can access without calling tools:

| Resource URI | Description | Returns |
|-------------|-------------|---------|
| `nerdtime://status` | Current tracking status | JSON with active session or null |
| `nerdtime://sessions/recent?limit=5` | Recent sessions | JSON array of sessions |
| `nerdtime://stats` | Time per project | JSON array of project stats |

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
        task_list_tool(),
        task_create_tool(),
        task_matrix_tool(),
        devlog_log_session_tool(),
        devlog_query_tool(),
        what_should_i_work_on_tool(),
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

### New files (17)

| File | Purpose |
|------|---------|
| `nerdtime-mcp/Cargo.toml` | Dependencies |
| `nerdtime-mcp/src/main.rs` | Entrypoint, server setup |
| `nerdtime-mcp/src/state.rs` | AppState with DB + config |
| `nerdtime-mcp/src/advisor.rs` | Decision tree engine (shared with CLI) |
| `nerdtime-mcp/src/tools/mod.rs` | Tool re-exports |
| `nerdtime-mcp/src/tools/start.rs` | start_tracking handler |
| `nerdtime-mcp/src/tools/stop.rs` | stop_tracking handler |
| `nerdtime-mcp/src/tools/status.rs` | get_status handler |
| `nerdtime-mcp/src/tools/list.rs` | list_sessions handler |
| `nerdtime-mcp/src/tools/stats.rs` | get_stats handler |
| `nerdtime-mcp/src/tools/sync.rs` | sync handler |
| `nerdtime-mcp/src/tools/task_list.rs` | task_list handler |
| `nerdtime-mcp/src/tools/task_create.rs` | task_create handler |
| `nerdtime-mcp/src/tools/task_matrix.rs` | task_matrix handler |
| `nerdtime-mcp/src/tools/devlog_log.rs` | devlog_log_session handler |
| `nerdtime-mcp/src/tools/devlog_query.rs` | devlog_query handler |
| `nerdtime-mcp/src/tools/advisor.rs` | what_should_i_work_on handler |

### Modified files

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `nerdtime-mcp` member |
| `nerdtime-db/src/lib.rs` | Add task CRUD, devlog CRUD, advisor query functions |
| `AGENTS.md` | Add MCP server section with tool docs and client config examples |
| `DEVLOG.md` | Document this session |
| `.env.example` | No changes needed (no new env vars) |

## Estimated effort

| Phase | Time |
|-------|------|
| Scaffold `nerdtime-mcp` crate + deps | 30 min |
| Implement tracking tool handlers (start, stop, status, list, stats, sync) | 2 hours |
| Implement task tools (list, create, matrix) | 1 hour |
| Implement devlog tools (log, query) | 1 hour |
| Implement advisor tool + decision tree engine | 1.5 hours |
| Add git detection, task CRUD, devlog CRUD to `nerdtime-db` | 1 hour |
| Testing with AI clients | 1 hour |
| Documentation (AGENTS.md, config examples) | 30 min |
| **Total** | **~8-9 hours** |

## Edge cases & notes

- **No active session**: `stop_tracking` and `get_status` must handle the case gracefully (return `{ active: false }` or error)
- **Git detection**: Runs `git branch --show-current` and `git rev-parse HEAD`; fails silently outside git repos (same as CLI)
- **Config missing**: If no `api_url` is configured, `sync` returns a clear error telling the user to run `nerd login`
- **DB locking**: SQLite via `rusqlite` inside a `Mutex` — adequate for single-process access. If the CLI runs concurrently, use SQLite WAL mode
- **Logging**: All server logs go to stderr (never stdout), per MCP stdio spec
- **Error format**: Tool errors return structured `McpError` with `is_handled: true` so the AI sees a clear message
- **Security**: The MCP server has access to the local SQLite database and config. It inherits the user's permissions. No network exposure (stdio only)
