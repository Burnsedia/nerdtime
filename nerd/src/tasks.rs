// SPDX-License-Identifier: AGPL-3.0-only
use crate::db;
use crate::insights;
use colored::Colorize;

pub fn render_matrix(tasks: &[db::TaskRow]) -> String {
    if tasks.is_empty() {
        return "  No tasks found.\n".to_string();
    }

    let mut q1 = Vec::new();
    let mut q2 = Vec::new();
    let mut q3 = Vec::new();
    let mut q4 = Vec::new();

    for t in tasks {
        match t.quadrant {
            1 => q1.push(t),
            2 => q2.push(t),
            3 => q3.push(t),
            _ => q4.push(t),
        }
    }

    let mut out = String::new();
    out.push('\n');

    render_quadrant(&mut out, "Q1: Do First", "urgency > 3, importance > 3", &q1, "red");
    out.push('\n');
    render_quadrant(&mut out, "Q2: Schedule", "urgency ≤ 3, importance > 3", &q2, "yellow");
    out.push('\n');
    render_quadrant(&mut out, "Q3: Delegate", "urgency > 3, importance ≤ 3", &q3, "blue");
    out.push('\n');
    render_quadrant(&mut out, "Q4: Eliminate", "urgency ≤ 3, importance ≤ 3", &q4, "white");

    out
}

fn render_quadrant(out: &mut String, title: &str, subtitle: &str, tasks: &[&db::TaskRow], color: &str) {
    let header = match color {
        "red" => title.red(),
        "yellow" => title.yellow(),
        "blue" => title.cyan(),
        _ => title.normal(),
    };
    out.push_str(&format!("  {}  ({})\n", header.bold(), subtitle.dimmed()));

    if tasks.is_empty() {
        out.push_str("    (empty)\n");
        return;
    }

    for t in tasks {
        let pct = if t.estimated_seconds.unwrap_or(0) > 0 {
            format!(
                "{:.0}%",
                t.actual_seconds as f64 / t.estimated_seconds.unwrap_or(1) as f64 * 100.0
            )
        } else {
            String::new()
        };
        let est_str = t
            .estimated_seconds
            .map(insights::fmt_duration)
            .unwrap_or_default();
        let act_str = if t.actual_seconds > 0 { insights::fmt_duration(t.actual_seconds) } else { String::new() };
        out.push_str(&format!(
            "    {}  {}  urg:{} imp:{}",
            status_icon(t.status.as_str()),
            t.title.bold(),
            t.urgency.to_string().red(),
            t.importance.to_string().green(),
        ));
        if !est_str.is_empty() {
            out.push_str(&format!("  est {}", est_str.cyan()));
        }
        if !act_str.is_empty() {
            out.push_str(&format!("  act {}", act_str));
        }
        if !pct.is_empty() {
            out.push_str(&format!("  ({})", pct));
        }
        out.push('\n');
    }
}

pub fn render_task_list(tasks: &[db::TaskRow]) -> String {
    if tasks.is_empty() {
        return "  No tasks found.\n".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!(
        "  {:<8} {:<28} {:<10} {:<10} {:<10} {:<6}\n",
        "Status".bold(),
        "Title".bold(),
        "Est".bold(),
        "Actual".bold(),
        "Remaining".bold(),
        "Q".bold(),
    ));

    for t in tasks {
        let est_str = t
            .estimated_seconds
            .map(insights::fmt_duration)
            .unwrap_or_else(|| "—".to_string());
        let act_str = if t.actual_seconds > 0 {
            insights::fmt_duration(t.actual_seconds)
        } else {
            "—".to_string()
        };
        let rem = t.estimated_seconds.map(|e| {
            let r = (e - t.actual_seconds).max(0);
            insights::fmt_duration(r)
        });
        let rem_str = rem.as_deref().unwrap_or("—");
        out.push_str(&format!(
            "  {:<8} {:<28} {:<10} {:<10} {:<10} Q{}\n",
            status_icon(t.status.as_str()),
            truncate(&t.title, 26),
            est_str,
            act_str,
            rem_str,
            t.quadrant,
        ));
    }

    out.push('\n');
    let total_actual: i64 = tasks.iter().map(|t| t.actual_seconds).sum();
    let total_est: i64 = tasks.iter().filter_map(|t| t.estimated_seconds).sum();
    out.push_str(&format!(
        "  {} tasks | {} tracked | {} estimated\n",
        tasks.len(),
        insights::fmt_duration(total_actual).bold(),
        insights::fmt_duration(total_est).bold(),
    ));

    out
}

pub fn render_estimate(task: &db::TaskRow, sessions: &[(String, String, i64, Option<i64>)]) -> String {
    let mut out = String::new();
    out.push_str(&format!("\n  {}  ({})\n\n", task.title.bold(), task.project_name.cyan()));

    let est_str = task
        .estimated_seconds
        .map(insights::fmt_duration)
        .unwrap_or_else(|| "none".to_string());
    out.push_str(&format!("  Estimate:   {}\n", est_str.bold()));

    let total_actual: i64 = sessions.iter().map(|(_, _, d, _)| *d).sum();
    let pct = task.estimated_seconds.map(|e| {
        if e > 0 {
            format!("{:.0}%", total_actual as f64 / e as f64 * 100.0)
        } else {
            "—".to_string()
        }
    });
    out.push_str(&format!(
        "  Actual:     {} ({})\n\n",
        insights::fmt_duration(total_actual).bold(),
        pct.as_deref().unwrap_or("—"),
    ));

    if !sessions.is_empty() {
        out.push_str("  Sessions:\n");
        for (start, _end, dur, est) in sessions {
            let date = &start[..10];
            let est_tag = est.map(|e| format!(" (-{})", insights::fmt_duration(e))).unwrap_or_default();
            out.push_str(&format!(
                "    {}  {}{}\n",
                date,
                insights::fmt_duration(*dur),
                est_tag.dimmed(),
            ));
        }
        out.push('\n');
    }

    let remaining = task.estimated_seconds.map(|e| (e - total_actual).max(0));
    if let Some(r) = remaining {
        out.push_str(&format!("  Remaining:  {}\n", insights::fmt_duration(r).bold()));
    }

    out
}

fn status_icon(status: &str) -> colored::ColoredString {
    match status {
        "active" => "●".green(),
        "completed" => "○".white(),
        "cancelled" => "✗".red(),
        _ => "?".yellow(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
