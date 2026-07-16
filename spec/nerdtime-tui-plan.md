# nerdtime TUI — Implementation Plan

> **Pricing**: Free (AGPL). Part of the free CLI tier.
> **Platforms**: Linux, macOS (no Windows planned).
> **Depends on**: `nerdtime-db` shared crate (extracted from `nerd/src/db.rs`)

## Overview

An interactive terminal UI for nerdtime's full feature set — time tracking, task Eisenhower Matrix, devlog browsing, and the what-should-i-work-on advisor. Launched as `nerd tui` subcommand. Replaces CLI flag memorization with visual panels.

6 panels with Tab cycling, modal vim keybindings (Normal/Insert/Command modes), live elapsed timer, and overlay forms.

## Architecture

```
nerd tui
  │
  ├── Event Loop (crossterm event stream, 250ms poll)
  │   ├── tick 1s — timer refresh
  │   ├── tick 5s — background DB re-query
  │   └── keyboard → mode dispatcher (Normal / Insert / Command)
  │
  ├── App State
  │   ├── mode: Mode (Normal | Insert(target) | Command(String))
  │   ├── active_panel: Panel (Dashboard | Stats | Tasks | Matrix | Devlog | Advisor)
  │   ├── active_modal: Option<Modal> (NewSession | NewTask | Help | Confirm | Filter | AdvisorForm)
  │   ├── data: { active_session, sessions, tasks, stats, devlog_entries, advisor_result }
  │   ├── ui: { selected_index, scroll_offset, filter, toast }
  │   └── sync: { status, last_sync }
  │
  ├── Render (ratatui v0.30 + crossterm v0.29)
  │   └── draw() → status bar (mode indicator) + panel content + modal overlay
  │
  └── Data: nerdtime-db (shared crate)
      └── ~/.config/nerdtime/data.db
```

## Vim Mode System

Three modes, visible in status bar at all times:

| Mode | Trigger | Indicator | Behavior |
|------|---------|-----------|----------|
| **NORMAL** | Default, Esc | `NORMAL` (bold green) | Navigation, single-key commands (`n`, `s`, `dd`, `cc`, `hjkl`) |
| **INSERT** | `/`, form fields | `INSERT` (bold yellow) | Text input for filters, forms, search |
| **COMMAND** | `:` | `:command_text` | Vim-style commands (`:q`, `:w`, `:help`, `:dashboard`) |

Mode transitions:
```
NORMAL  ── [/] ──→  INSERT (filter/search)
NORMAL  ── [:] ──→  COMMAND (type command)
NORMAL  ── [n] ──→  INSERT (new session/task form)
INSERT  ── [Esc] →  NORMAL
COMMAND ── [Esc] →  NORMAL
COMMAND ── [Ent] →  Execute command → NORMAL
```

## Files

| File | Purpose |
|------|---------|
| `nerd/src/tui.rs` | Module root: `run()`, event loop, terminal init/restore |
| `nerd/src/tui/app.rs` | `App` struct — state, mode dispatch, refresh, tick |
| `nerd/src/tui/ui.rs` | `draw()` — top-level layout, mode indicator, panel dispatch |
| `nerd/src/tui/panels/mod.rs` | Re-exports |
| `nerd/src/tui/panels/dashboard.rs` | Active timer + recent sessions + footer stats |
| `nerd/src/tui/panels/stats.rs` | Per-project bar chart, heatmap, insights overlays |
| `nerd/src/tui/panels/tasks.rs` | Task list with status/quadrant/estimates |
| `nerd/src/tui/panels/matrix.rs` | Eisenhower Q1-Q4 2x2 grid |
| `nerd/src/tui/panels/devlog.rs` | Devlog entry list with search |
| `nerd/src/tui/panels/advisor.rs` | Advisor form + results display |
| `nerd/src/tui/modals.rs` | Overlay widgets: NewSession, NewTask, Help, Confirm, Filter, AdvisorForm |
| `nerd/src/tui/widgets.rs` | Reusable: ScrollableList, StatusBar, Toast, SparklineBar, ModeIndicator |

**14 new files.**

## Dependencies (`nerd/Cargo.toml` additions)

```toml
[dependencies]
ratatui = "0.30"
crossterm = { version = "0.29", features = ["event-stream"] }
nerdtime-db = { path = "../nerdtime-db" }    # already added
```

## Key Bindings

### NORMAL mode

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down (list nav) |
| `k` / `↑` | Move selection up |
| `h` | Go back / parent / previous panel context |
| `l` | Enter / select / drill down |
| `gg` | Go to top of list |
| `G` | Go to bottom of list |
| `Ctrl+d` / `PageDown` | Scroll down half page |
| `Ctrl+u` / `PageUp` | Scroll up half page |
| `dd` | Delete selected session/task (with confirm) |
| `cc` | Complete selected task |
| `n` | New session / task / devlog entry (context-dependent) |
| `s` | Sync unsynced sessions to API |
| `Enter` | Stop active session / confirm modal |
| `Tab` / `gt` | Next panel |
| `Shift+Tab` / `gT` | Previous panel |
| `/` | Enter filter/search (→ INSERT) |
| `?` | Help overlay |
| `:` | Enter command mode |
| `Esc` | Clear overlay / close help / cancel filter |
| `q` | Quit (with confirm if active session) |
| `r` | Force refresh from DB |

### Panel-specific NORMAL keys

| Panel | Key | Action |
|-------|-----|--------|
| Dashboard | `Enter` | Stop active session |
| Stats | `i` | Insights overlay |
| Stats | `h` | Heatmap overlay |
| Tasks | `a` | Add new task (modal) |
| Tasks | `c` | Complete selected |
| Tasks | `x` | Cancel selected |
| Tasks | `m` | Switch to Matrix view |
| Tasks | `Enter` | Start tracking selected |
| Tasks | `e` | Edit selected task (track estimate) |
| Matrix | `l` | Switch to task list view |
| Matrix | `Enter` | View task detail |
| Devlog | `e` | Edit selected entry (in $EDITOR) |
| Devlog | `Enter` | View entry detail |
| Devlog | `n` | New entry |
| Advisor | `Enter` | Run analysis with current inputs |
| Advisor | `s` | Start tracking suggested task |

### COMMAND mode commands

| Command | Action |
|---------|--------|
| `:q` | Quit |
| `:w` | Sync (write unsynced sessions to API) |
| `:wq` | Sync then quit |
| `:e` | Refresh DB (re-read all data) |
| `:help` | Help overlay |
| `:new` | New session form |
| `:dashboard` | Switch to Dashboard panel |
| `:stats` | Switch to Stats panel |
| `:tasks` | Switch to Tasks panel |
| `:matrix` | Switch to Matrix panel |
| `:devlog` | Switch to Devlog panel |
| `:advisor` | Switch to Advisor panel |
| `:sync` | Sync unsynced sessions |

## Screens

### Dashboard (default)

```
NORMAL  │  nerdtime v0.1.0  │  Panel: Dashboard  │  ● 1 unsynced
┌─ Active Session ────────────────────────────────────────────────┐
│                                                                  │
│    ● Tracking  my-project  (main)                                │
│                                                                  │
│       01:23:47                                                   │
│                                                                  │
│    started 1h 23m ago  •  task: fix-auth-bug                     │
│                                                                  │
│    [Enter] Stop    [n] New    [/] Filter                         │
└──────────────────────────────────────────────────────────────────┘
┌─ Recent Sessions ──────────────────────────────────────────────┐
│  #  Project     Duration   When        Task       Synced       │
│  1  website     2h 10m     yesterday   redesign   ✓            │
│  2  my-project  45m        today       fix-auth   ○            │
│  3  docs        30m        2d ago      —          ✓            │
│  4  nerdtime    1h 05m     today       —          ○            │
└──────────────────────────────────────────────────────────────────┘
4 projects  •  12h 30m  •  2 unsynced  •  last sync: 2m ago
[Tab] Cycle  [n] New  [s] Sync  [/] Filter  [?] Help
```

### Stats

```
NORMAL  │  nerdtime v0.1.0  │  Panel: Stats  │  ✓ all synced
┌─ Time per Project ──────────────────────────────────────────────┐
│                                                                  │
│  my-project    ██████████████████████████████  4h 12m           │
│  website       ██████████████████████████      3h 45m           │
│  another-proj  ███████████████████              2h 30m           │
│  docs          ████████████████                  2h 10m           │
│  nerdtime      ████████                          1h 05m           │
│                                                                  │
│  Total: 13h 42m across 5 projects  •  47 sessions                │
└──────────────────────────────────────────────────────────────────┘
[i] Insights  [h] Heatmap  [Tab] Next panel
```

### Tasks

```
NORMAL  │  nerdtime v0.1.0  │  Panel: Tasks  │  ✓ all synced
┌─ Tasks (active) ────────────────────────────────────────────────┐
│  Status  Title                 Est      Actual   Remaining  Q   │
│  ●       Fix login timeout     2h 00m   1h 15m   45m       Q1  │
│  ●       Refactor auth module  4h 00m   0                  Q2  │
│  ●       Update docs           1h 00m   30m       30m       Q2  │
│  ○       Deploy v1.0           —        —         —        Q1  │
│  ●       Optimize queries      3h 00m   0                  Q3  │
│                                                                  │
│  5 active  •  1 completed  •  1 cancelled                        │
└──────────────────────────────────────────────────────────────────┘
[a] Add  [c] Complete  [x] Cancel  [Enter] Start  [m] Matrix
```

### Eisenhower Matrix

```
NORMAL  │  nerdtime v0.1.0  │  Panel: Matrix  │  ✓ all synced
┌───────────────────────────────┬───────────────────────────────┐
│  Q1: Do First                │  Q2: Schedule                 │
│  urgency >3, importance >3    │  urgency ≤3, importance >3    │
│                               │                               │
│  ● Fix login timeout  2h     │  ● Refactor auth      4h     │
│  ○ Deploy v1.0         —     │  ● Write integration tests 3h │
├───────────────────────────────┼───────────────────────────────┤
│  Q3: Delegate                │  Q4: Eliminate                │
│  urgency >3, importance ≤3    │  urgency ≤3, importance ≤3    │
│                               │                               │
│  ● Optimize queries   3h     │  ● Update docs         1h     │
└───────────────────────────────┴───────────────────────────────┘
[↑↓←→] Navigate  [Enter] View  [l] Task list
```

### Devlog

```
NORMAL  │  nerdtime v0.1.0  │  Panel: Devlog  │  ✓ all synced
┌─ Devlog Entries ───────────────────────────────────────────────┐
│  Date       Title                        Role    Tags          │
│  2026-07-15  Fix auth token refresh      human   [auth]        │
│  2026-07-14  Implement MCP server        ai      [mcp]         │
│  2026-07-14  Design DB schema for tasks   hybrid  [design]     │
│  2026-07-13  Write integration tests      human   [testing]    │
│  2026-07-12  Refactor CLI argument parsing ai      [cli]       │
│                                                                 │
│  Showing 5 of 23 entries  •  [/] Search  •  [n] New entry      │
└──────────────────────────────────────────────────────────────────┘
[↑↓] Navigate  [Enter] View  [/] Search  [n] New  [e] Edit
```

### Advisor

```
NORMAL  │  nerdtime v0.1.0  │  Panel: Advisor  │  ✓ all synced
┌─ What Should I Work On? ───────────────────────────────────────┐
│                                                                 │
│  Available time: [2h         ]                                  │
│  Energy level:   [medium     ]  (low / medium / high)           │
│  Blocked on:     [           ]  (optional)                      │
│                                                                 │
│  ┌──────────────────────────── Analyze ───────────────────────┐ │
│  │  Suggestion: Fix login timeout                              │ │
│  │  Reasoning: Top priority Q1 task. Fits your 2h block.      │ │
│  │                                                             │ │
│  │  Related tasks:                                             │ │
│  │    ● Fix login timeout       Q1  est 2h                     │ │
│  │    ● Write integration tests Q2  est 3h                     │ │
│  │    ● Update docs             Q4  est 1h                     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│  [Enter] Analyze  [s] Start suggested task                      │
└──────────────────────────────────────────────────────────────────┘
```

## State Model

```rust
pub struct App {
    // Mode
    pub mode: Mode,

    // Panel
    pub active_panel: Panel,

    // Data
    pub active_session: Option<Session>,
    pub sessions: Vec<Session>,
    pub tasks: Vec<TaskRow>,
    pub stats: Vec<ProjectStat>,
    pub devlog_entries: Vec<DevlogEntry>,
    pub advisor_result: Option<Advice>,

    // UI state
    pub active_modal: Option<Modal>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub filter_text: String,
    pub toast: Option<Toast>,
    pub terminal_size: (u16, u16),

    // Timer
    pub session_start_instant: Option<Instant>,
    pub elapsed_seconds: u64,
    pub last_db_refresh: Instant,

    // Sync
    pub sync_status: SyncStatus,
    pub last_sync: Option<String>,

    // Config
    pub api_url: String,
    pub has_token: bool,
}

pub enum Mode {
    Normal,
    Insert(InsertTarget),  // filter | form_field | search
    Command(String),       // text after ":"
}

pub enum Panel {
    Dashboard,
    Stats,
    Tasks,
    Matrix,
    Devlog,
    Advisor,
}

pub enum InsertTarget {
    Filter,
    NewSessionProject,
    NewSessionDescription,
    NewTaskTitle,
    NewTaskEstimate,
    NewTaskLabels,
    DevlogSearch,
    AdvisorTime,
    AdvisorEnergy,
    AdvisorBlocked,
}

pub enum Modal {
    NewSession,
    NewTask,
    Help,
    Confirm { message: String, action: ConfirmAction },
    FilterInput,
    AdvisorForm,
    TaskDetail(usize),
    DevlogDetail(usize),
    Heatmap,
    Insights,
}

pub enum SyncStatus {
    Idle,
    Syncing,
    Success(usize),
    Failure(String),
    NoConfig,
}

pub struct Toast {
    pub message: String,
    pub style: ToastStyle,
    pub expires_at: Instant,
}

pub enum ToastStyle {
    Info,
    Success,
    Error,
}
```

## Event Loop

```rust
pub fn run(conn: &Connection) -> Result<()> {
    let mut terminal = init_terminal()?;
    let config = config::load().ok();
    let mut app = App::new(conn, &config);

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.handle_key(key, conn)? { break; }
                }
                Event::Resize(w, h) => app.terminal_size = (w, h),
                _ => {}
            }
        }

        app.tick();
        app.refresh_if_needed(conn);
    }

    restore_terminal()?;
    Ok(())
}
```

## Data Flow

| Panel | nerdtime-db calls | Frequency |
|-------|------------------|-----------|
| Dashboard | `show_status()`, `list_sessions(project, 50)` | status: 1s, list: 5s |
| Stats | `stats_by_project()` | 5s |
| Tasks | `list_tasks(project, status)` | 5s |
| Matrix | `list_tasks(project, Some("active"))` | 5s |
| Devlog | `list_devlog_entries(20)` / `search_devlog_entries(q, tags)` | on focus / on search |
| Advisor | `decide()` | on analyze action |
| Sync | `get_unsynced_sessions()`, `mark_synced()` | on `s` / `:w` |

## Modal System

Ratatui `Clear` widget + centered `Block` + `Paragraph`/`List`:

```
fn render_modal(f: &mut Frame, modal: &Modal, app: &App) {
    let area = centered_rect(f.size(), 60, 50);
    f.render_widget(Clear, area);
    match modal {
        Modal::NewSession   => render_new_session_form(f, area, app),
        Modal::NewTask      => render_new_task_form(f, area, app),
        Modal::Help         => render_help_overlay(f, area),
        Modal::Confirm { .. } => render_confirm_dialog(f, area, app),
        Modal::FilterInput  => render_filter_input(f, area, app),
        Modal::AdvisorForm  => render_advisor_form(f, area, app),
        Modal::TaskDetail(i) => render_task_detail(f, area, app, *i),
        Modal::DevlogDetail(i) => render_devlog_detail(f, area, app, *i),
        Modal::Heatmap      => render_heatmap(f, area, app),
        Modal::Insights     => render_insights_panel(f, area, app),
    }
}
```

## Refresh Strategy

| Trigger | Action |
|---------|--------|
| Every 1s | Update elapsed timer, expire toasts |
| Every 5s | Full DB refresh (all panels) |
| On panel switch | Refresh that panel's data |
| On user action | Immediate refresh |
| `r` key | Force immediate full refresh |
| No refresh while modal open | Prevents visual glitches |

## Implementation Order

| Step | What | Files | Time |
|------|------|-------|------|
| 1 | Add `Tui` variant to clap; scaffold `tui.rs` with terminal init/restore | `main.rs`, `tui.rs` | 30m |
| 2 | App struct, Mode enum, panel system, event loop with tick + 5s refresh | `app.rs` | 1.5h |
| 3 | Mode dispatch: Normal/Insert/Command key routing | `app.rs` | 1h |
| 4 | Dashboard panel: active timer, sessions table, status bar, footer stats | `panels/dashboard.rs`, `ui.rs` | 2h |
| 5 | Modal system: NewSession, Help overlay, Confirm dialog, Filter input | `modals.rs` | 1.5h |
| 6 | Stats panel: bar chart, total, heatmap/insights overlays | `panels/stats.rs` | 1.5h |
| 7 | Tasks panel: list with status/quadrant/estimates, add/complete/cancel | `panels/tasks.rs`, `modals.rs` | 1.5h |
| 8 | Matrix panel: 2x2 Eisenhower grid navigation | `panels/matrix.rs` | 1.5h |
| 9 | Devlog panel: entry list, search, detail view, new entry | `panels/devlog.rs` | 1.5h |
| 10 | Advisor panel: form fields, analyze action, results, start tracking | `panels/advisor.rs` | 1.5h |
| 11 | Command mode parser + dispatch (`:q`, `:w`, `:help`, panel names) | `app.rs` | 1h |
| 12 | Sync integration: status indicator, sync action, toast feedback | `app.rs`, `ui.rs` | 1h |
| 13 | Colors, layout tuning, resize handling, edge cases | All | 2h |
| **Total** | | **14 new, 2 modified** | **~18-20h** |

## Widget Layout (ratatui)

```
┌─ Status Bar (1 line) ──────────────────────────────────────────┐
│  NORMAL  │  nerdtime v0.1.0  │  Panel: Dashboard  │  ● 1 unsynced│
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Panel Content (remaining height)                                │
│  (varies by active_panel)                                        │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  Footer (1 line):  [Tab] Cycle  [n] New  [s] Sync  [?] Help     │
└─────────────────────────────────────────────────────────────────┘
```

## Color Scheme

| Element | Color |
|---------|-------|
| Mode indicator (NORMAL) | Bold green on default |
| Mode indicator (INSERT) | Bold yellow on default |
| Mode indicator (COMMAND) | Bold cyan on default |
| Active timer | Green |
| Timer digits | Bold white |
| No active session | Yellow |
| Panel headers | Bold cyan |
| Table headers | Bold white |
| Q1 tasks | Red foreground |
| Q2 tasks | Yellow foreground |
| Q3 tasks | Blue foreground |
| Q4 tasks | Dim white |
| Synced checkmark | Green |
| Unsynced | Yellow |
| Selected row | Reversed |
| Toast success | Green |
| Toast error | Red on dark red bg |
| Toast warning | Yellow |
| Modal background | Dimmed |

## Edge Cases

| Case | Handling |
|------|----------|
| No active session | Timer panel shows "Not tracking" with `[n]` prompt |
| No sessions | "No sessions — start tracking with `[n]`" |
| No tasks | "No tasks — add one with `[a]`" |
| No devlog entries | "No entries — log one with `[n]`" |
| Terminal < 80x24 | Overlay: "Resize to at least 80×24" |
| DB locked | Error toast, retry on next 5s refresh |
| Config missing | Status bar: "⚠ Not configured — run `nerd login`" |
| Sync in progress | Disable `[s]`, spinner in status bar |
| External DB changes | 5s background refresh catches them |
| Unicode/CJK names | ratatui handles Unicode |
| Resize during modal | Close modal, re-render on next draw |
| Very long project names | Truncate with `…` at panel width |
| Active session while quitting | Confirm dialog: "Active session running. Quit anyway?" |
| Rapid key presses | 250ms event poll prevents flooding |
| Fetching errors | Toast notification, never panic |
