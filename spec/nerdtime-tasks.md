# nerdtime Tasks — Implementation Plan

## Overview

Add task tracking as a first-class entity. Tasks are todo items within a project with estimates, status tracking, and session association. Every session can optionally belong to a task, enabling estimation accuracy at both the session and task level.

Labels provide cross-cutting categorization across projects and tasks. Every session and task can have zero or more labels. Labels enable aggregate summaries (e.g., "total time spent on all bug-related work across every project").

## Data model

### New `tasks` table

```sql
CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    project_name TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    estimated_seconds INTEGER,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL,
    completed_at TEXT
);
```

- `id` — UUID v4
- `project_name` — matches sessions.project_name
- `title` — short task name
- `description` — optional detail
- `estimated_seconds` — overall estimate (e.g., 7200 = 2h)
- `status` — `active` | `completed` | `cancelled`
- `created_at` — RFC 3339
- `completed_at` — RFC 3339 when status becomes `completed`

### Sessions table changes

```sql
ALTER TABLE sessions ADD COLUMN task_id TEXT;
ALTER TABLE sessions ADD COLUMN estimated_seconds INTEGER;
ALTER TABLE sessions ADD COLUMN labels TEXT;
```

- `task_id` — FK to tasks.id (nullable, no constraint enforcement)
- `estimated_seconds` — per-session estimate (e.g., estimate 30m for today's chunk)
- `labels` — JSON array of strings, e.g. `["bug","urgent"]`. NULL means no labels.

### Tasks table changes

```sql
ALTER TABLE tasks ADD COLUMN labels TEXT;
```

- `labels` — JSON array of strings. Task labels cascade to sessions by default.

### Sync payload changes

`SyncPayload` in `nerdtime-core` gets optional `task_id`, `estimated_seconds`, and `labels` fields:

```rust
pub struct SyncPayload {
    pub id: Uuid,
    pub project_name: String,
    pub branch_name: Option<String>,
    pub commit_hash: Option<String>,
    pub description: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub task_id: Option<String>,           // NEW
    pub estimated_seconds: Option<i64>,    // NEW
    pub labels: Option<Vec<String>>,       // NEW
}
```

## CLI commands

### Task CRUD

```sh
# Create a task
nerd task add <project> "task title" --estimate 2h --desc "details"

# List tasks for a project
nerd task list <project>
nerd task list <project> --status completed

# Mark complete / cancel
nerd task complete <task-id>
nerd task cancel <task-id>

# Edit a task
nerd task edit <task-id> --title "new title" --estimate 3h
```

### Labels

Labels are passed as a comma-separated or repeated list on `start`, `task add`, and `task edit`:

```sh
# On session start
nerd start project --label bug --label urgent
nerd start project -l "bug,urgent"

# On task create
nerd task add project "fix login" --label "bug,frontend" --estimate 2h

# On task edit (replaces all labels)
nerd task edit my-task --label "bug,ui"

# Clear labels
nerd task edit my-task --label ""
```

### Start/stop with tasks and labels

```sh
# Track time for a specific task
nerd start <project> --task <task-id>
nerd start <project> --task <task-id> --estimate 30m
nerd start <project> --task <task-id> --label "bug,urgent"

# Task labels auto-apply to session unless overridden

# Stop — shows task estimate comparison
nerd stop
# ✓ Tracking stopped (1h 15m) — task implement login, 2h 45m estimated remaining
```

### Estimates and insights

```sh
# Task-level estimation accuracy
nerd estimate <task-id>

# Project-level with task breakdown
nerd estimate <project>
```

## Detailed output formats

### `nerd task list <project>`

```
$ nerd task list nerdtime

  Status  Title                      Est      Actual    Remaining
  ──────  ─────────────────────      ───      ──────    ────────
  ●       implement login            4h 00m   2h 15m    1h 45m
  ●       design dashboard           3h 00m   0h 30m    2h 30m
  ○       fix sync bug               —        1h 15m    —
  ✗       refactor cli args          2h 00m   2h 45m    — (over by 45m)

  4 tasks | 6h 45m tracked | 9h 00m estimated
```

- `●` active (unfinished)
- `○` completed
- `✗` cancelled

### `nerd estimate <task-id>`

```
$ nerd estimate 550e8400-e29b-41d4-a716-446655440000

  implement login  (nerdtime)

  Estimate:   4h 00m
  Actual:     2h 15m  (56% of estimate)

  Sessions:
  Mar 10   1h 00m  (-estimate 30m)
  Mar 11   0h 45m  (-estimate 1h)
  Mar 12   0h 30m  (-estimate 30m)

  Remaining: 1h 45m
```

### `nerd estimate <project>` (with tasks)

```
$ nerd estimate nerdtime

  Project: nerdtime

  Tasks with estimates:
  implement login       4h 00m est → 2h 15m act   ✓ under
  design dashboard      3h 00m est → 0h 30m act   🔄 in progress
  refactor cli args     2h 00m est → 2h 45m act   ✗ over by 45m

  Tasks without estimates:
  fix sync bug          —                → 1h 15m act

  Project totals:
  6h 45m tracked across 4 tasks
  9h 00m total estimate
  3h 45m remaining (in active tasks)
```

## Time parsing

Human-readable time strings used for `--estimate` and `-e`:

| Input | Seconds |
|---|---|
| `30m` | 1800 |
| `2h` | 7200 |
| `1h30m` | 5400 |
| `1.5h` | 5400 |
| `90m` | 5400 |
| `0` / `none` | remove estimate |

Shared helper function in `nerd/src/parse.rs` or inline in `db.rs`.

## Task ID resolution

Tasks are identified by UUID. For convenience, `nerd task list` shows a short prefix or the user can use tab completion. The CLI accepts both full UUID and unique prefix:

```
$ nerd task complete 550e8400   # matches the task starting with this prefix
$ nerd task complete 550e8400-e29b-41d4-a716-446655440000  # full UUID
```

The task ID is also displayed in the first column of `nerd task list` for easy copy-paste.

## Integration with existing features

### Heat map and insights

Tasks get a breakdown column in `nerd insights`:

```
Top tasks (in progress):
  implement login       2h 15m   25%
  design dashboard      0h 30m    6%
```

`nerd heatmap` unchanged — still shows activity by weekday × hour. Tasks are a filter dimension, not a visualization dimension.

### Sync

- `task_id` and `estimated_seconds` fields are part of `SyncPayload`
- Backend stores them in the `sessions` table (backend schema also needs migration)
- Tasks themselves are NOT synced with the backend (local-only for now) — they're a CLI-side concept. The task_id is just a string reference on the session record.
- Later, tasks can be synced as a separate resource if needed.

### Config

No changes. Tasks table lives in the same `data.db`.

### Summary command

```
$ nerd summary

Summary (last 30 days):

Label         Time       %      Projects
bug           12h 30m   42%    nerdtime, nerdtime-api
frontend      8h 15m    28%    nerdtime-tauri
meeting       5h 00m    17%    all
research      3h 45m    13%    nerdtime-core
────────────────────────────────────────––
Total:       29h 30m   100%    4 labels across 7 sessions

$ nerd summary --label bug

Label         Time       %      Projects
bug           12h 30m   100%   nerdtime, nerdtime-api

$ nerd summary --project nerdtime

Label         Time       %      
bug           6h 15m    34%    
frontend      4h 00m    22%    
meeting       3h 00m    16%    
research      3h 45m    20%    
────────────────────────
Total:       17h 00m

$ nerd summary --from 2026-01-01 --to 2026-03-01
```

Flags:

| Flag | Default | Description |
|---|---|---|
| `--project` / `-p` | all | Filter to a specific project |
| `--label` / `-l` | all | Filter to a specific label |
| `--from` | 30d ago | Start date |
| `--to` | today | End date |
| `--days` / `-d` | 30 | Alternative to --from |
| `--json` | false | Output as JSON |

SQL query for label aggregation:

```sql
SELECT j.value as label,
       SUM(CAST((julianday(s.ended_at) - julianday(s.started_at)) * 86400 AS INTEGER)) as total_seconds
FROM sessions s, json_each(s.labels) AS j
WHERE s.ended_at IS NOT NULL
  AND s.started_at >= ?1
  AND (?2 IS NULL OR s.project_name = ?2)
GROUP BY j.value
ORDER BY total_seconds DESC;
```

## Edge cases

| Case | Behavior |
|---|---|
| Session without task | `task_id = NULL` — existing behavior unchanged |
| Task deleted while session active | Session still references the task UUID (no FK constraint). `nerd stop` works fine, `nerd task list` shows "(deleted)" |
| Complete task mid-session | `nerd task complete` while session is running → warn "Task has an active session" |
| Multiple active tasks | Tasks are independent. A session can only belong to one task at a time. |
| Estimate = 0 / NULL | Treated as "no estimate" — excluded from accuracy calculations |
| Very long projects | Tasks table indexes on `project_name` (no dedicated index needed — `project_name` is already used for lookup) |
| Session without labels | `labels = NULL` — excluded from label aggregation, works fine in all other commands |
| Session with task + labels | Labels from session override task labels; both stored independently |
| Label with special characters | JSON-escaped. `sqlite` json_each handles them. Slashes/colons/spaces allowed. |
| Empty label list (`--label ""`) | Sets `labels = NULL` — clears all labels |
| No sessions with a given label | `nerd summary --label X` shows "No sessions found" |

## New files and changes

| File | Change |
|---|---|
| `nerd/Cargo.toml` | No new deps (serde_json already included for sync) |
| `nerd/src/db.rs` | + task CRUD functions, + task_id/estimated_seconds/labels in start_session/stop_session/map_session, + helper for unique prefix resolution, + summary query |
| `nerd/src/insights.rs` | + task breakdown in estimate output, + label breakdown in insights |
| `nerd/src/main.rs` | + `Task` subcommand with `Add`/`List`/`Complete`/`Edit`/`Cancel` sub-subcommands, + `--task`, `--estimate`, `--label` on `Start`, + `Summary` subcommand |
| `nerdtime-core/src/lib.rs` | + `task_id`, `estimated_seconds`, `labels` to `SyncPayload` |
| `nerdtime-api/migration/src/` | New migration adding `task_id`, `estimated_seconds`, `labels` columns to `sessions` table |

### Functions in `db.rs`

```rust
// Task CRUD
pub fn add_task(conn: &Connection, project: &str, title: &str, desc: Option<&str>, est: Option<i64>) -> Result<String>  // returns task UUID
pub fn list_tasks(conn: &Connection, project: Option<&str>, status: Option<&str>) -> Result<Vec<TaskRow>>
pub fn complete_task(conn: &Connection, task_id: &str) -> Result<()>
pub fn cancel_task(conn: &Connection, task_id: &str) -> Result<()>
pub fn edit_task(conn: &Connection, task_id: &str, title: Option<&str>, est: Option<Option<i64>>) -> Result<()>

// Estimates
pub fn task_estimate(conn: &Connection, task_id: &str) -> Result<TaskEstimate>
pub fn project_estimate(conn: &Connection, project: &str) -> Result<ProjectEstimate>

// Summary
pub fn label_summary(conn: &Connection, filter: SummaryFilter) -> Result<Vec<LabelSummaryRow>>

pub struct SummaryFilter {
    pub project: Option<String>,
    pub label: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub json: bool,
}

pub struct LabelSummaryRow {
    pub label: String,
    pub total_seconds: i64,
    pub projects: Vec<String>,
}

// Helpers
fn resolve_task_id(conn: &Connection, partial: &str) -> Result<String>  // unique prefix resolution
fn parse_duration(input: &str) -> Result<i64>  // "2h30m" → 9000

// Data structs
pub struct TaskRow {
    pub id: String,
    pub project_name: String,
    pub title: String,
    pub description: Option<String>,
    pub estimated_seconds: Option<i64>,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub actual_seconds: i64,       // aggregated from sessions
}

pub struct TaskEstimate {
    pub task: TaskRow,
    pub sessions: Vec<SessionSummary>,
    pub remaining: Option<i64>,
}

pub struct ProjectEstimate {
    pub project: String,
    pub tasks: Vec<TaskRow>,
    pub total_estimated: i64,
    pub total_actual: i64,
}
```

### Changes to `start_session()` / `stop_session()`

```rust
pub fn start_session(
    conn: &Connection,
    project: &str,
    desc: Option<&str>,
    task_id: Option<&str>,          // NEW
    estimated_seconds: Option<i64>, // NEW
    labels: Option<Vec<String>>,    // NEW
) -> Result<()>
```

Insert includes `task_id`, `estimated_seconds`, and `labels` columns:

```sql
INSERT INTO sessions (id, project_name, branch_name, commit_hash, description,
                      started_at, task_id, estimated_seconds, labels)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
```

If `--task` is specified without `--label`, labels are inherited from the task. If both are specified, session labels override task labels.

```rust
// When both task and labels provided
let labels = match (task_id, &labels) {
    (Some(tid), None) => get_task_labels(conn, tid)?,  // inherit from task
    (_, Some(l)) => Some(serde_json::to_string(&l)?),  // explicit labels
    (None, None) => None,
};
```

Stop output shows task context when applicable:

```
✓ Tracking stopped (1h 15m) — task implement login, 2h 45m estimated remaining
```

### Changes to `main.rs` subcommands

```rust
enum Commands {
    Start {
        project: String,
        #[arg(short, long)]
        desc: Option<String>,
        #[arg(short = 't', long)]
        task: Option<String>,           // NEW
        #[arg(short = 'e', long)]
        estimate: Option<String>,       // NEW — parse_duration
        #[arg(short = 'l', long)]
        label: Option<Vec<String>>,     // NEW — comma-separated or repeated
    },
    // ... existing ...

    /// Show summary by label
    Summary {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        label: Option<String>,
        #[arg(short = 'd', long, default_value = "30")]
        days: i64,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Manage tasks
    #[command(subcommand)]
    Task(TaskCommands),

    /// Show estimation accuracy
    Estimate {
        id: Option<String>,          // task-id or project name
        #[arg(short, long)]
        project: Option<String>,
    },
}

#[derive(Subcommand)]
enum TaskCommands {
    Add {
        project: String,
        title: String,
        #[arg(short, long)]
        desc: Option<String>,
        #[arg(short = 'e', long)]
        estimate: Option<String>,
    },
    List {
        project: Option<String>,
        #[arg(short, long)]
        status: Option<String>,       // "active" | "completed" | "cancelled"
    },
    Complete {
        id: String,
    },
    Cancel {
        id: String,
    },
    Edit {
        id: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short = 'e', long)]
        estimate: Option<String>,     // pass "0" or "none" to clear
    },
}
```

## Implementation order

| Step | Files | Time |
|---|---|---|
| DB: `tasks` table schema + create in `get_connection()` | `db.rs` | 30 min |
| DB: task CRUD functions (add, list, complete, cancel, edit) | `db.rs` | 1.5 hr |
| DB: `parse_duration()` helper | `db.rs` or new `parse.rs` | 30 min |
| DB: `resolve_task_id()` unique prefix helper | `db.rs` | 15 min |
| DB: update `start_session()` / `stop_session()` + `map_session()` | `db.rs` | 30 min |
| DB: estimate queries (`task_estimate`, `project_estimate`) | `db.rs` | 1 hr |
| CLI: `Task` sub-subcommands + dispatch | `main.rs` | 1 hr |
| CLI: `--task` + `--estimate` flags on `Start` | `main.rs` | 15 min |
| CLI: `Estimate` subcommand | `main.rs` | 15 min |
| Formatting: task list, estimate output | `db.rs` (inline) | 1 hr |
| CLI: `Summary` subcommand | `main.rs` | 15 min |
| CLI: `--label` flag on `Start` and `Task Add`/`Edit` | `main.rs` | 15 min |
| DB: label aggregation + summary query | `db.rs` | 45 min |
| Formatting: task list, estimate output, label breakdown | `db.rs` / `insights.rs` | 1.5 hr |
| Backend: add `task_id`, `estimated_seconds`, `labels` to sessions + sync | `nerdtime-api` migration + model | 30 min |
| Core: update `SyncPayload` | `nerdtime-core/src/lib.rs` | 5 min |
| Manual testing | — | 1.5 hr |
| **Total** | | **~10 hrs** |
