# nerdtime MCP Server — Implementation Plan

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
│   │   └── sync.rs     # sync tool
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
| Implement state + tool handlers | 2-3 hours |
| Add git detection helpers to `nerdtime-db` | 30 min |
| Testing with Cline/Claude Desktop | 1 hour |
| Documentation (AGENTS.md, config examples) | 30 min |
| **Total** | **~5-6 hours** |

## Edge cases & notes

- **No active session**: `stop_tracking` and `get_status` must handle the case gracefully (return `{ active: false }` or error)
- **Git detection**: Runs `git branch --show-current` and `git rev-parse HEAD`; fails silently outside git repos (same as CLI)
- **Config missing**: If no `api_url` is configured, `sync` returns a clear error telling the user to run `nerd login`
- **DB locking**: SQLite via `rusqlite` inside a `Mutex` — adequate for single-process access. If the CLI runs concurrently, use SQLite WAL mode
- **Logging**: All server logs go to stderr (never stdout), per MCP stdio spec
- **Error format**: Tool errors return structured `McpError` with `is_handled: true` so the AI sees a clear message
- **Security**: The MCP server has access to the local SQLite database and config. It inherits the user's permissions. No network exposure (stdio only)
