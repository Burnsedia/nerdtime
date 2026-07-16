# End-to-End Testing — Specification

> **Status**: 📝 Planned
> **Effort**: ~10h

## 1. Philosophy

- **Offline-first**: CLI tests use real temp SQLite, never mock the database
- **No mocking of external APIs**: `nerd sync` tests verify behavior against a real (ephemeral) backend or verify the `is_synced` flag without a real sync
- **Binary-level where possible**: CLI tests spawn the actual `nerd` binary via `assert_cmd`; MCP tests spawn the actual binary over stdio
- **Library-level for depth**: DB-layer tests call `nerdtime-db` functions directly for fine-grained assertions
- **API integration**: Existing Loco boot-test pattern extended to sessions, billing, sync

## 2. Test Infrastructure

| Component | Crate | Technique |
|---|---|---|
| CLI binary tests | `nerd` | `assert_cmd` + `tempfile` temp dir + `predicates` for stdout assertions |
| DB layer tests | `nerd` (library) | `nerdtime-db::connection::init_schema` on temp SQLite |
| MCP binary tests | `nerdtime-mcp` | `std::process::Command` + JSON-RPC over stdio |
| API integration | `nerdtime-api` | Loco `boot_test` + `request` patterns (existing) |

### 2.1 Temp DB Setup (shared helper)

```rust
fn with_temp_db<F>(f: F) where F: Fn(Connection) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conn = Connection::open(&path).unwrap();
    nerdtime_db::connection::init_schema(&conn).unwrap();
    f(conn);
}
```

### 2.2 CLI Binary Setup (shared helper)

```rust
fn with_temp_nerd<F>(f: F) where F: Fn(Command) {
    let dir = tempfile::tempdir().unwrap();
    let config_dir = dir.path().join(".config/nerdtime");
    fs::create_dir_all(&config_dir).unwrap();

    // Write minimal config pointing to temp DB
    let config = Config { db_path: config_dir.join("data.db"), ... };
    config::save_to(&config, &config_dir).unwrap();

    let mut cmd = Command::cargo_bin("nerd").unwrap();
    cmd.env("NERDTIME_CONFIG_DIR", &config_dir);
    f(cmd);
}
```

## 3. CLI End-to-End Tests

All in `nerd/tests/`.

### 3.1 `nerd/tests/sessions.rs`

Full session lifecycle via library API on temp DB:

| Test | What it verifies |
|---|---|
| `test_session_start_stop` | start creates row, stop sets ended_at |
| `test_session_start_with_task` | start with task_id links session to task |
| `test_session_start_with_labels` | labels saved correctly |
| `test_session_status_active` | show_status returns Some when active |
| `test_session_status_none` | show_status returns None when idle |
| `test_session_list` | list_sessions returns correct count |
| `test_session_list_project_filter` | filter by project name |
| `test_session_sync_mark` | mark_synced flips is_synced flag |
| `test_session_duration` | elapsed time computed correctly |
| `test_session_estimate` | estimate vs actual tracking |
| `test_sync_payload_roundtrip` | session → SyncPayload → JSON → back |

### 3.2 `nerd/tests/tasks.rs`

| Test | What it verifies |
|---|---|
| `test_task_add` | add_task creates row with correct fields |
| `test_task_add_with_quadrant` | quadrant computed from urgency/importance |
| `test_task_complete` | complete_task sets completed_at |
| `test_task_cancel` | cancel_task updates status |
| `test_task_list` | list_tasks returns correct subset |
| `test_task_list_filter_status` | filter by status string |
| `test_task_edit` | edit_task updates fields |
| `test_task_estimate_accuracy` | estimated vs actual comparison |
| `test_advisor_decide` | decide() returns Advice with reasoning |
| `test_advisor_no_tasks` | decide() returns "Take a break" |
| `test_eisenhower_matrix` | tasks grouped by quadrant correctly |
| `test_label_summary` | label_summary aggregates correctly |

### 3.3 `nerd/tests/devlog.rs`

| Test | What it verifies |
|---|---|
| `test_devlog_insert` | insert_devlog_entry creates entry |
| `test_devlog_list` | list_devlog_entries returns sorted |
| `test_devlog_search_text` | search by text matches |
| `test_devlog_search_tags` | search by tags matches |
| `test_devlog_update` | update_devlog_entry modifies fields |
| `test_devlog_get` | get_devlog_entry returns by id |
| `test_devlog_render_md` | render_devlog_md produces markdown |
| `test_devlog_cache_commit` | cache_commit stores commit data |
| `test_devlog_get_unlogged` | unlogged commits computed correctly |

### 3.4 `nerd/tests/heatmap.rs`

| Test | What it verifies |
|---|---|
| `test_heatmap_empty` | no data returns empty vec |
| `test_heatmap_with_data` | cells grouped by day/hour |
| `test_insights_empty` | no data returns zeros |
| `test_insights_with_data` | totals and per-project correct |
| `test_stats_by_project` | stats aggregated correctly |

### 3.5 `nerd/tests/cli.rs`

Binary-level tests (spawn `nerd` process):

| Test | What it verifies |
|---|---|
| `test_cli_start_stop` | `nerd start proj` then `nerd stop` succeeds |
| `test_cli_status_active` | `nerd status` shows project name |
| `test_cli_status_none` | `nerd status` shows "No active session" |
| `test_cli_log` | `nerd log --limit 5` lists sessions |
| `test_cli_help` | `nerd --help` prints usage |
| `test_cli_unknown_command` | exits with error |
| `test_cli_version` | `nerd --version` prints version |

## 4. MCP End-to-End Tests

All in `nerdtime-mcp/tests/mcp.rs`.

Approach: spawn the MCP binary, send JSON-RPC requests via stdin, read JSON-RPC responses from stdout.

```rust
use std::process::{Command, Stdio};
use std::io::Write;

fn send_mcp_request(request: &str) -> String {
    let mut child = Command::cargo_bin("nerdtime-mcp")
        .unwrap()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.as_mut().unwrap().write_all(request.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}
```

| Test | What it verifies |
|---|---|
| `test_mcp_tool_list` | tools/list returns known tools |
| `test_mcp_session_start_stop` | start then stop, verify status changes |
| `test_mcp_session_list` | list returns created sessions |
| `test_mcp_task_add_list` | add then list returns task |
| `test_mcp_task_complete` | complete returns success |
| `test_mcp_devlog_log` | log entry returns id |
| `test_mcp_devlog_query` | query returns logged entry |
| `test_mcp_devlog_generate` | generate writes file |
| `test_mcp_advisor_decide` | decide returns suggestion |
| `test_mcp_error_invalid_input` | bad input returns error |
| `test_mcp_sync` | sync returns result |

## 5. API End-to-End Tests

All in `nerdtime-api/tests/requests/`, extending the existing Loco test pattern.

### 5.1 `nerdtime-api/tests/requests/sessions.rs`

| Test | What it verifies |
|---|---|
| `test_sync_requires_auth` | POST /api/sync without token returns 401 |
| `test_sync_creates_sessions` | valid sync creates session rows |
| `test_sync_updates_existing` | same id updates not duplicates |
| `test_list_sessions` | GET /api/sessions returns user's sessions |
| `test_list_sessions_project_filter` | ?project=param filters results |
| `test_list_sessions_limit` | ?limit= param controls count |
| `test_stats` | GET /api/stats returns per-project aggregates |

### 5.2 `nerdtime-api/tests/requests/billing.rs`

| Test | What it verifies |
|---|---|
| `test_billing_info_requires_auth` | GET /api/billing/info without token returns 401 |
| `test_billing_info` | returns current subscription tier/status |
| `test_billing_checkout` | POST /api/billing/checkout returns url |
| `test_billing_webhook` | POST /api/billing/webhook processes event |
| `test_billing_portal` | GET /api/billing/portal redirects |

## 6. CI Integration

File: `.github/workflows/ci.yml`

```yaml
name: CI
on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          components: clippy, rustfmt
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-features -- -D warnings

  test-api:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_USER: loco
          POSTGRES_PASSWORD: loco
          POSTGRES_DB: nerdtime-api_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      redis:
        image: redis:7
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo test -p nerdtime-api

  test-cli:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo test -p nerd -- --nocapture

  test-mcp:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo test -p nerdtime-mcp
```

## 7. Running Tests

```sh
# Everything
cargo test --workspace

# CLI only
cargo test -p nerd

# MCP only
cargo test -p nerdtime-mcp

# API only (needs PostgreSQL + Redis)
cargo test -p nerdtime-api

# Single test
cargo test -p nerd -- test_session_start_stop
```

## 8. Writing New Tests

### Conventions

1. **One test file per domain**: `sessions.rs`, `tasks.rs`, `devlog.rs`, `heatmap.rs`, `cli.rs`
2. **Naming**: `test_<action>_<expected_state>`
3. **Temp DB only**: never touch `~/.config/nerdtime/`
4. **Clean up**: tempfile `TempDir` drops automatically
5. **No mocking**: real SQLite, real functions — if it needs a network, skip or test the local-only path
6. **Isolation**: each test creates its own temp DB

### Fixtures

Pre-made test data in `nerd/tests/fixtures/`:

- `sessions.rs` — helper functions to seed sessions at various timestamps
- `tasks.rs` — helper to seed tasks in each quadrant
- `devlog.rs` — helper to seed devlog entries with various tags

```rust
// Example fixture helper
pub fn seed_session(conn: &Connection, project: &str, started: &str, ended: &str) {
    // ...
}
```

## 9. Implementation Order

| Step | What | Files | Time |
|---|---|---|---|
| 1 | Write this spec document | `spec/end-to-end-testing.md` | 1h |
| 2 | Add dev-deps + test helpers | `nerd/Cargo.toml`, `tests/mod.rs`, `tests/fixtures/` | 1h |
| 3 | Session lifecycle tests | `tests/sessions.rs` | 1h |
| 4 | Task + advisor tests | `tests/tasks.rs` | 1h |
| 5 | Devlog tests | `tests/devlog.rs` | 1h |
| 6 | Heatmap + insights + stats tests | `tests/heatmap.rs` | 0.5h |
| 7 | CLI binary tests | `tests/cli.rs` | 1h |
| 8 | MCP JSON-RPC tests | `nerdtime-mcp/tests/mcp.rs` | 2h |
| 9 | API session tests | `nerdtime-api/tests/requests/sessions.rs` | 1.5h |
| 10 | API billing tests | `nerdtime-api/tests/requests/billing.rs` | 1.5h |
| 11 | CI workflow | `.github/workflows/ci.yml` | 1h |
| | **Total** | **~15 files** | **~10h** |
