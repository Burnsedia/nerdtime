// SPDX-License-Identifier: AGPL-3.0-only
use colored::Colorize;
use nerdtime_core::{HeatmapCell, Insights};

const DAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const BLOCK_NAMES: [&str; 4] = [
    "🌅 Morning (6-12)",
    "☀️ Afternoon (12-18)",
    "🌙 Evening (18-0)",
    "🌃 Night (0-6)",
];

pub fn fmt_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, mins)
    } else if mins > 0 {
        format!("{}m", mins)
    } else {
        format!("{}s", secs)
    }
}

pub fn render_heatmap(cells: &[HeatmapCell]) -> String {
    let mut grid = [[0i64; 24]; 7];
    let mut max_val: i64 = 0;
    for cell in cells {
        let d = cell.day as usize;
        let h = cell.hour as usize;
        if d < 7 && h < 24 {
            grid[d][h] = cell.total_seconds;
            if cell.total_seconds > max_val {
                max_val = cell.total_seconds;
            }
        }
    }

    if max_val == 0 {
        return "  No sessions found in the specified period.\n".to_string();
    }

    let threshold_1 = max_val / 3;
    let threshold_2 = (max_val * 2) / 3;

    let mut out = String::new();
    out.push_str("\nHour    ");
    for h in 0..24 {
        out.push_str(&format!("{:2} ", h));
    }
    out.push('\n');

    for d in 0..7 {
        out.push_str(&format!("{}  ", DAY_NAMES[d]));
        for val in grid[d] {
            let block = if val == 0 {
                "   ".to_string()
            } else if val <= threshold_1 {
                " ░░".to_string()
            } else if val <= threshold_2 {
                " ▓▓".to_string()
            } else {
                " ██".to_string()
            };
            out.push_str(&block);
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "\n        {}  < {}  |  {}  < {}  |  {}  > {}\n",
        "░░".cyan(),
        fmt_duration(threshold_1),
        "▓▓".yellow(),
        fmt_duration(threshold_2),
        "██".green(),
        fmt_duration(max_val),
    ));

    out
}

pub fn render_insights(data: &Insights) -> String {
    if data.session_count == 0 {
        return "  No sessions found in the specified period.\n".to_string();
    }

    let mut out = String::new();
    out.push_str("\n📊 Productivity Insights\n\n");

    let max_block = *data.per_block.iter().max().unwrap_or(&1).max(&1);
    for (i, &seconds) in data.per_block.iter().enumerate() {
        let bar_len = if max_block > 0 {
            (seconds as f64 / max_block as f64 * 20.0).round() as usize
        } else {
            0
        };
        let bar: String = "█".repeat(bar_len);
        out.push_str(&format!(
            "  {}  {}  {}\n",
            BLOCK_NAMES[i],
            fmt_duration(seconds).bold(),
            bar.green(),
        ));
    }
    out.push('\n');

    let mut day_pairs: Vec<(usize, i64)> =
        data.per_day_of_week.iter().copied().enumerate().collect();
    day_pairs.sort_by_key(|b| std::cmp::Reverse(b.1));
    if let Some((best_day, best_seconds)) = day_pairs.first() {
        if *best_seconds > 0 {
            out.push_str(&format!(
                "  Most productive:   {} ({})\n",
                DAY_NAMES[*best_day].bold(),
                fmt_duration(*best_seconds),
            ));
        }
    }
    if let Some((worst_day, worst_seconds)) = day_pairs.last() {
        if *worst_seconds > 0 {
            out.push_str(&format!(
                "  Least productive:  {} ({})\n",
                DAY_NAMES[*worst_day].bold(),
                fmt_duration(*worst_seconds),
            ));
        }
    }

    if let Some((top_project, top_seconds)) = data.per_project.first() {
        if data.total_seconds > 0 {
            let pct = (*top_seconds as f64 / data.total_seconds as f64 * 100.0).round() as u64;
            out.push_str(&format!(
                "  Top project:       {} ({} / {}%)\n",
                top_project.bold(),
                fmt_duration(*top_seconds),
                pct,
            ));
        }
    }

    out.push('\n');
    let daily_avg = if data.session_count > 0 {
        data.total_seconds / data.session_count.max(1)
    } else {
        0
    };
    out.push_str(&format!("  Sessions:  {} completed\n", data.session_count));
    out.push_str(&format!(
        "  Total:     {}\n",
        fmt_duration(data.total_seconds).bold()
    ));
    out.push_str(&format!("  Daily avg: {}\n", fmt_duration(daily_avg)));

    out
}
