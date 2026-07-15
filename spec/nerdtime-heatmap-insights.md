# nerdtime Heat Map & Insights — Implementation Plan

## Overview

Two new CLI commands that visualize productivity patterns from session data. Uses SQLite `strftime` for time extraction and the existing `colored` crate for terminal rendering.

## Data model

Every session has `started_at` and `ended_at`. From these we extract:
- **Day of week**: `strftime('%w', started_at)` — 0=Sunday, 6=Saturday
- **Hour of day**: `strftime('%H', started_at)` — 0-23
- **Duration**: `ended_at - started_at` (in seconds)
- **Project**: `project_name`

### Heat map query

Aggregates total seconds per (weekday, hour) bin. Uses start hour as the slot.

```sql
SELECT CAST(strftime('%w', started_at) AS INTEGER) as day,
       CAST(strftime('%H', started_at) AS INTEGER) as hour,
       SUM(
           CAST((julianday(ended_at) - julianday(started_at)) * 86400 AS INTEGER)
       ) as total_seconds
FROM sessions
WHERE started_at >= datetime('now', ?1 || ' days', 'localtime')
  AND ended_at IS NOT NULL
  AND (?2 IS NULL OR project_name = ?2)
GROUP BY day, hour
ORDER BY day, hour
```

### Insights query

Separate queries for:
- **Per-block totals**: morning/afternoon/evening/night
- **Per-day totals**: total seconds by weekday
- **Per-project totals**: total seconds by project
- **Overall aggregates**: total tracked, session count, daily average

## Commands

### `nerd heatmap`

ASCII heat map with shaded blocks. Each cell represents one weekday + hour slot.

```
$ nerd heatmap --days 30

Hour   0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16 17 18 19 20 21 22 23
Mon                                   ░░ ██ ██ ██ ██ ░░ ░░
Tue                       ░░ ██ ██ ██ ██ ██ ██ ██ ██ ██ ▓▓
Wed                    ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██
Thu                       ██ ██ ██ ██ ██ ██ ██ ██ ██ ▓▓ ░░
Fri                    ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ██ ░░
Sat                                                                       ██
Sun                                                         ██ ██

       0-1h  ░░  |  1-3h  ▓▓  |  3h+  ██
```

**Thresholds:** Three density levels based on total minutes in each cell:
- `░` = less than threshold 1 (e.g., 1h)
- `▓` = between threshold 1 and 2 (e.g., 1-3h)
- `█` = above threshold 2 (e.g., 3h+)

**Thresholds are auto-calculated** from the data: max value determines the top bin, divide into thirds.

### `nerd insights`

Text summary of productivity patterns.

```
$ nerd insights --days 30

📊 Productivity Insights (last 30 days)

Peak hours:
  🌅 Morning (6-12)    18h 30m  ████████████████
  ☀️ Afternoon (12-18) 22h 15m  ████████████████████
  🌙 Evening (18-0)     5h 45m  █████
  🌃 Night (0-6)        0h 30m  ░

Most productive:   Wednesday (15h 20m)
Least productive:  Saturday  (2h 10m)
Top project:       nerdtime  (25h 30m / 54%)

Sessions:  42 completed (0 active)
Total:     47h 00m
Daily avg: 1h 34m
```

### Shared flags

| Flag | Default | Description |
|---|---|---|
| `--days` / `-d` | 30 | Lookback window in days |
| `--project` / `-p` | all | Filter to a single project |

## New files

```
nerd/src/
├── db.rs          # + heatmap_data(), insights_data()
├── insights.rs    # NEW: formatting + rendering for both commands
├── main.rs        # + Heatmap, Insights subcommands
```

## SQL queries

### `heatmap_data()`

```rust
pub struct HeatmapCell {
    pub day: u32,         // 0-6
    pub hour: u32,        // 0-23
    pub total_seconds: i64,
}

pub fn heatmap_data(conn: &Connection, days: i64, project: Option<&str>) -> Result<Vec<HeatmapCell>>
```

### `insights_data()`

```rust
pub struct Insights {
    pub total_seconds: i64,
    pub session_count: i64,
    pub per_block: [i64; 4],              // [morning, afternoon, evening, night]
    pub per_day: [(String, i64); 7],      // [(day_name, seconds)]
    pub per_project: Vec<(String, i64)>,  // [(project_name, seconds)]
}

pub fn insights_data(conn: &Connection, days: i64, project: Option<&str>) -> Result<Insights>
```

## Rendering (`insights.rs`)

```rust
pub fn render_heatmap(cells: &[HeatmapCell]) -> String
pub fn render_insights(data: &Insights) -> String
```

Both return a `String` that gets printed to stdout. No structural coupling to the CLI — easy to reuse in the TUI later.

### Heat map rendering

1. Create a 7×24 grid, fill from `cells` data
2. Find max value across all cells for auto-thresholding
3. For each day row: print day abbreviation, then 24 characters
4. Print legend at bottom

### Insights rendering

1. Compute per-block totals from raw data
2. Build per-day summaries
3. Sort projects by total descending
4. Format bar charts using `colored`

## Changes to `main.rs`

```rust
enum Commands {
    Start { ... },
    Stop,
    Status,
    Sync,
    Log { ... },
    Login { ... },
    Config,
    Heatmap {
        #[arg(short, long, default_value = "30")]
        days: i64,
        #[arg(short, long)]
        project: Option<String>,
    },
    Insights {
        #[arg(short, long, default_value = "30")]
        days: i64,
        #[arg(short, long)]
        project: Option<String>,
    },
}
```

Dispatch:

```rust
Commands::Heatmap { days, project } => {
    let data = db::heatmap_data(&conn, *days, project.as_deref())?;
    print!("{}", insights::render_heatmap(&data));
    Ok(())
}
Commands::Insights { days, project } => {
    let data = db::insights_data(&conn, *days, project.as_deref())?;
    print!("{}", insights::render_insights(&data));
    Ok(())
}
```

## Implementation order

| Step | Files | Time |
|---|---|---|
| SQL queries: `heatmap_data()`, `insights_data()` | `db.rs` | 1 hr |
| Rendering: `render_heatmap()` | `insights.rs` | 1 hr |
| Rendering: `render_insights()` | `insights.rs` | 1 hr |
| CLI subcommands + dispatch | `main.rs` | 30 min |
| Manual testing + edge cases | — | 30 min |
| **Total** | | **~4 hrs** |

## Edge cases

- **No data in range**: Both commands show empty state ("No sessions found in the last N days.")
- **Active sessions only (no finished)**: Excluded from aggregates (no `ended_at`). `list_sessions` already handles this.
- **Single session**: Should render correctly, no divide-by-zero in threshold calc.
- **Very large range (e.g., 365 days)**: SQLite handles it fine. Grid still shows 7×24.
- **Midnight crossover**: Session starting at 23:30 counts toward hour 23. Acceptable for MVP.
