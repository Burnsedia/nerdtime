# nerdtime TUI — Implementation Plan

> **Pricing**: Free (AGPL). Part of the free CLI tier.
> **Platforms**: Linux, macOS (no Windows planned).

## Architecture

```
nerd tui  (subcommand of existing CLI)
  │
  ├── Event Loop (crossterm event stream, non-blocking)
  │   ├── tick every 1 second (updates timer)
  │   └── keyboard input handler
  │
  ├── State (App struct)
  │   ├── active_session: Option<Session>
  │   ├── sessions: Vec<Session>
  │   ├── stats: Vec<ProjectStat>
  │   ├── filter: Option<String>
  │   ├── focused_panel: enum (Active | List | Stats)
  │   └── edit_mode: bool (for project name input)
  │
  ├── Render (ratatui + crossterm)
  │   └── draw() → renders current state as TUI layout
  │
  └── DB (rusqlite directly, same db.rs calls)
      └── ~/.config/nerdtime/data.db
```

## Approach: `nerd tui` subcommand

Add to the existing `nerd` CLI. Users already have the binary — `nerd tui` opens the interactive dashboard. Reuses the same SQLite database and config as `nerd start/stop`.

## New files

| File | Purpose |
|------|---------|
| `nerd/src/tui.rs` | Module root — `run()` function, event loop, state init |
| `nerd/src/tui/app.rs` | `App` struct — all state fields + `handle_event()`, `update()` |
| `nerd/src/tui/ui.rs` | `draw()` function — all ratatui layout, rendering, widgets |
| `nerd/src/tui/handlers.rs` | Keyboard input handler dispatch |

## Cargo.toml additions

```toml
[dependencies]
ratatui = "0.30"
crossterm = { version = "0.28", features = ["event-stream"] }
```

Add a `[[bin]]` section is not needed — same binary, new subcommand via `clap`.

## Screens

### Dashboard (default view)

```
┌──────────────────────────────────────────────────┐
│  nerdtime v0.1.0                   q quit  s sync │
├──────────────────────────────────────────────────┤
│  ┌─ Active Session ────────────────────────────┐ │
│  │                                              │ │
│  │    ● Tracking  my-project                    │ │
│  │                                              │ │
│  │       01:23:47                               │ │
│  │                                              │ │
│  │    started 2m ago  •  branch: main           │ │
│  │                                              │ │
│  │    [Enter] Stop  [n] New  [/] Filter         │ │
│  └──────────────────────────────────────────────┘ │
│                                                   │
│  ┌─ Recent Sessions ────────────────────────────┐ │
│  │  Project          Duration    When    Synced │ │
│  │  ─────────────────────────────────────────── │ │
│  │  my-project       1h 23m     today    ✓     │ │
│  │  another-proj     45m       today    ✓     │ │
│  │  website         2h 10m    yesterday  ○     │ │
│  │  docs            30m       2d ago    ✓     │ │
│  └──────────────────────────────────────────────┘ │
│                                                   │
│  4 projects  •  12h 30m total  •  1 unsynced     │
└──────────────────────────────────────────────────┘
```

### Stats panel (Tab to cycle)

```
┌─ Time per Project ────────────────────────────┐
│                                                │
│  my-project    ████████████████  4h 12m       │
│  another-proj  ██████████        2h 45m       │
│  website       ████████████████  4h 30m       │
│  docs          ██████             1h 33m       │
│                                                │
│  [Tab] back to dashboard                       │
└────────────────────────────────────────────────┘
```

### New Session prompt (modal overlay)

```
┌─ New Session ────────────────────────────────┐
│                                               │
│  Project: my-project                          │
│  Description (optional): fixing auth bug      │
│                                               │
│  [Enter] Start  [Esc] Cancel                  │
└───────────────────────────────────────────────┘
```

## States per panel

### Active Session

| State | Display |
|-------|---------|
| No active session | "No active session" — prompt to press `n` to start |
| Active session, <1 min | "● Tracking {project}" with live seconds counter |
| Active session, >1 min | "● Tracking {project}" with MM:SS or HH:MM:SS display |
| Stopped (just pressed Enter) | Flash "✓ Stopped — 1h 23m" for 2 seconds, then refresh |

### Sessions List

| State | Display |
|-------|---------|
| Empty database | "No sessions yet. Start tracking with [n]." |
| Has sessions | Table with header row, scrollable |
| Filter active | Header shows "Filter: {project}" — only matching rows shown |
| No matches for filter | "No sessions matching '{filter}'" |

### Stats Panel

| State | Display |
|-------|---------|
| Empty data | "No stats yet. Track some time first." |
| Has data | Bar chart, sorted by total_seconds descending |

### Sync

| State | Display |
|-------|---------|
| Nothing to sync | Status text unchanged — "All synced" |
| Syncing | Show spinner or "Syncing..."; block input briefly |
| Sync success | Flash "✓ Synced {n} sessions" |
| Sync failure | Flash "✗ Sync failed: {status}" |
| Not configured | "Not configured — run `nerd login`" |

## Key bindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit (restore terminal) |
| `n` | New session — open project name prompt |
| `Enter` | Stop active session |
| `s` | Sync unsynced sessions to API |
| `j` / `↓` | Move selection down in list |
| `k` / `↑` | Move selection up in list |
| `/` | Filter sessions by project — opens text input |
| `Tab` | Cycle panels: Dashboard → Stats → Dashboard |
| `d` | Delete selected session (with confirmation dialog) |
| `r` | Force refresh from database |
| `?` | Help overlay — shows all bindings |
| `Esc` | Close modal / clear filter |

## Event loop flow

```
fn run() -> Result<()> {
    let conn = open_db()?;
    let mut terminal = init_terminal()?;
    let mut app = App::new(&conn);

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(Duration::from_secs(1))? {
            match event::read()? {
                Event::Key(key) => {
                    if handlers::handle_key(key, &mut app, &conn)? {
                        break; // quit
                    }
                }
                _ => {}
            }
        }

        app.tick(); // update elapsed time
        app.refresh_if_needed(&conn); // check DB for external changes
    }
}
```

## Data model integration

The TUI calls the same functions the CLI already has in `db.rs`:

| CLI function | TUI usage | Returns |
|-------------|-----------|---------|
| `start_session(conn, project, desc)` | New Session modal → `[Enter]` | void (inserts row) |
| `stop_session(conn)` | `[Enter]` on active session | void (sets ended_at) |
| `show_status(conn)` | On load + every tick | Session struct |
| `list_sessions(conn, project, limit)` | Sessions list panel | Vec\<Session\> |
| `stats_by_project(conn)` | Stats panel | Vec\<ProjectStat\> |
| `sync_sessions(conn)` | `[s]` key | void (marks synced) |

These will eventually come from the `nerdtime-db` shared crate once extracted.

## Widget layout (ratatui)

```
Main layout (Vertical split):
  ┌─ Status Bar ─────────────────────────────────┐  1 line
  ├─ Active Session (or prompt to start) ────────┤  6 lines
  ├─ Recent Sessions (scrollable table) ─────────┤  remaining
  └─ Footer: hotkeys summary ────────────────────┘  1 line
```

Modal windows rendered on top via ratatui `Clear` widget + centered `Paragraph` block.

## Implementation order

| Step | What | Time |
|------|------|------|
| 1 | Add `tui` subcommand to clap; scaffold `tui.rs` with `run()` and terminal init | 30 min |
| 2 | `App` struct with active_session + recent sessions; event loop with tick | 1 hr |
| 3 | Dashboard render: timer display + sessions table + status bar | 2 hr |
| 4 | Keyboard handlers: start/stop, navigate, filter, quit | 1 hr |
| 5 | New Session modal prompt with text input | 1 hr |
| 6 | Stats panel with bar chart + Tab cycling | 1 hr |
| 7 | Sync integration + status feedback | 1 hr |
| 8 | Help overlay, confirmation dialogs, polish | 1 hr |
| **Total** | | **~9 hr** |

## Files modified

| File | Change |
|------|--------|
| `nerd/Cargo.toml` | Add `ratatui`, `crossterm` |
| `nerd/src/main.rs` | Add `Tui` variant to `Commands` enum + dispatch |

## Files created

| File | Purpose |
|------|---------|
| `nerd/src/tui.rs` | Module root: `run()`, event loop |
| `nerd/src/tui/app.rs` | App state struct + methods |
| `nerd/src/tui/ui.rs` | All ratatui draw/rendering code |
| `nerd/src/tui/handlers.rs` | Keyboard dispatch |

## Edge cases

- **No active session**: Timer panel shows "Not tracking" with start prompt
- **No sessions yet**: List shows empty-state message
- **No config for sync**: Shows "Not configured — run `nerd login`"
- **Terminal too small (< 80x24)**: Show resize warning
- **DB locked by another process**: SQLite retries; show brief wait message
- **External changes** (user runs `nerd start` in another terminal): Refresh on next tick
- **Unicode/emoji in project names**: ratatui handles Unicode; test with CJC
