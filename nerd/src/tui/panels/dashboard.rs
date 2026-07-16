// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use nerdtime_db as db;

use crate::tui::app::App;
use crate::tui::widgets::truncate;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let (top, recent, footer) = if app.heatmap_data.is_empty() && app.insights_data.is_none() {
        let chunks = Layout::vertical([
            Constraint::Length(8),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);
        (chunks[0], chunks[1], chunks[2])
    } else {
        let heatmap_rows = if app.heatmap_data.is_empty() { 0 } else { 10 };
        let insights_rows = if app.insights_data.is_some() { 6 } else { 0 };
        let chunks = Layout::vertical([
            Constraint::Length(8),
            Constraint::Length(heatmap_rows),
            Constraint::Length(insights_rows),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);
        if heatmap_rows > 0 {
            render_heatmap_section(f, chunks[1], app);
        }
        if insights_rows > 0 {
            render_insights_section(f, chunks[2], app);
        }
        (chunks[0], chunks[3], chunks[4])
    };

    render_active_session(f, top, app);
    render_recent_sessions(f, recent, app);
    render_footer(f, footer, app);
}

fn render_active_session(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Active Session ")
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::new().fg(ratatui::style::Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(ref session) = app.active_session {
        let elapsed = app.elapsed_seconds;
        let hours = elapsed / 3600;
        let mins = (elapsed % 3600) / 60;
        let secs = elapsed % 60;
        let timer_str = format!("{:02}:{:02}:{:02}", hours, mins, secs);

        let lines = vec![
            ratatui::text::Line::from(vec![
                ratatui::text::Span::styled(" ● ", ratatui::style::Style::new().fg(ratatui::style::Color::Green)),
                ratatui::text::Span::styled(
                    &session.project_name,
                    ratatui::style::Style::new().fg(ratatui::style::Color::White).add_modifier(ratatui::style::Modifier::BOLD),
                ),
                if let Some(ref branch) = session.branch_name {
                    ratatui::text::Span::styled(
                        format!(" ({})", branch),
                        ratatui::style::Style::new().fg(ratatui::style::Color::Cyan),
                    )
                } else {
                    ratatui::text::Span::raw("")
                },
            ]),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    format!("   {}", timer_str),
                    ratatui::style::Style::new()
                        .fg(ratatui::style::Color::White)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
            ),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    format!(
                        "   started {} ago",
                        db::fmt_duration(
                            (chrono::Utc::now() - session.started_at).num_seconds().max(0)
                        )
                    ),
                    ratatui::style::Style::new().fg(ratatui::style::Color::Green),
                ),
            ),
        ];
        let paragraph = ratatui::widgets::Paragraph::new(ratatui::text::Text::from(lines));
        f.render_widget(paragraph, inner);
    } else {
        let text = ratatui::text::Text::from(vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    "   ● Not tracking",
                    ratatui::style::Style::new().fg(ratatui::style::Color::Yellow),
                ),
            ),
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    "   [n] Start new session",
                    ratatui::style::Style::new().fg(ratatui::style::Color::DarkGray),
                ),
            ),
        ]);
        let paragraph = ratatui::widgets::Paragraph::new(text);
        f.render_widget(paragraph, inner);
    }
}

fn render_heatmap_section(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(" Heatmap (last {}d) ", app.days))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Green));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let max_val = app
        .heatmap_data
        .iter()
        .map(|c| c.total_seconds)
        .max()
        .unwrap_or(3600)
        .max(1);

    let mut lines = vec![Line::from(Span::raw("     "))];
    for h in 0..24 {
        lines[0].push_span(Span::raw(format!("{:>3}", h)));
    }

    for (d, day) in days.iter().enumerate() {
        let mut line = Line::from(Span::raw(format!(" {:>3} ", day)));
        for h in 0..24 {
            let seconds = app
                .heatmap_data
                .iter()
                .find(|c| c.day == d as u32 && c.hour == h as u32)
                .map(|c| c.total_seconds)
                .unwrap_or(0);

            let intensity = if seconds == 0 {
                " · "
            } else if seconds as f64 / max_val as f64 > 0.75 {
                " █ "
            } else if seconds as f64 / max_val as f64 > 0.5 {
                " ▓ "
            } else if seconds as f64 / max_val as f64 > 0.25 {
                " ▒ "
            } else {
                " ░ "
            };

            line.push_span(Span::styled(intensity, Style::new().fg(Color::Green)));
        }
        lines.push(line);
    }

    let paragraph = Paragraph::new(ratatui::text::Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_insights_section(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(format!(" Insights (last {}d) ", app.days))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Yellow));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(ref insights) = app.insights_data else {
        return;
    };

    let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let best_day_idx = insights
        .per_day_of_week
        .iter()
        .enumerate()
        .max_by_key(|&(_, &v)| v)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let peak_block = insights
        .per_block
        .iter()
        .enumerate()
        .max_by_key(|&(_, &v)| v)
        .map(|(i, _)| match i {
            0 => "Morning (6-12)",
            1 => "Afternoon (12-18)",
            2 => "Evening (18-24)",
            _ => "Night (0-6)",
        })
        .unwrap_or("—");

    let top_project = insights
        .per_project
        .iter()
        .max_by_key(|&(_, s)| s)
        .map(|(p, _)| p.as_str())
        .unwrap_or("—");

    let total_days: i64 = insights.per_day_of_week.iter().filter(|&&v| v > 0).count() as i64;
    let avg_daily = if total_days > 0 {
        db::fmt_duration(insights.total_seconds / total_days)
    } else {
        "—".to_string()
    };

    let lines = vec![
        Line::from(Span::raw(format!(
            " Top: {}  •  {} total  •  {} sessions  •  Avg/day: {}",
            top_project,
            db::fmt_duration(insights.total_seconds),
            insights.session_count,
            avg_daily,
        ))),
        Line::from(Span::raw(format!(
            " Peak: {}  •  Best day: {}",
            peak_block, day_names[best_day_idx],
        ))),
    ];

    let paragraph = Paragraph::new(ratatui::text::Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_recent_sessions(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Recent Sessions ")
        .borders(Borders::ALL)
        .border_style(ratatui::style::Style::new().fg(ratatui::style::Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.sessions.is_empty() {
        let text = ratatui::text::Text::from(vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    " No sessions — start tracking with [n]",
                    ratatui::style::Style::new().fg(ratatui::style::Color::DarkGray),
                ),
            ),
        ]);
        let paragraph = ratatui::widgets::Paragraph::new(text);
        f.render_widget(paragraph, inner);
        return;
    }

    let header = Line::from(vec![
        Span::styled(" #  ", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("Project     ", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("Duration   ", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("When        ", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("Task       ", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("Sync", Style::new().add_modifier(Modifier::BOLD)),
    ]);

    let mut items = vec![Line::from("")];
    items.push(header);

    for (i, session) in app.sessions.iter().enumerate() {
        let duration_str = if let Some(ended_at) = session.ended_at {
            let dur = (ended_at - session.started_at).num_seconds();
            db::fmt_duration(dur)
        } else {
            "active".to_string()
        };
        let when = format!("{}", session.started_at.format("%Y-%m-%d"));
        let synced = if session.is_synced {
            Span::styled("✓", Style::new().fg(Color::Green))
        } else {
            Span::styled("○", Style::new().fg(Color::Yellow))
        };
        let task_short = session
            .task_id
            .as_ref()
            .map(|t| {
                if t.len() > 7 {
                    &t[..7]
                } else {
                    t.as_str()
                }
            })
            .unwrap_or("—");

        let is_selected = i == app.selected_index;
        let row_style = if is_selected {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new()
        };

        let row = Line::from(vec![
            Span::styled(format!(" {:<3} ", i + 1), row_style),
            Span::styled(truncate(&session.project_name, 12), row_style),
            Span::styled(format!(" {:<10} ", duration_str), row_style),
            Span::styled(format!(" {:<10} ", when), row_style),
            Span::styled(format!(" {:<10} ", task_short), row_style),
            synced,
        ]);
        items.push(row);
    }

    let list = ratatui::widgets::List::new(items);
    f.render_widget(list, inner);
}

fn render_footer(f: &mut Frame, area: Rect, app: &App) {
    let total = db::fmt_duration(app.total_duration);
    let text = format!(
        " {} projects  •  {}  •  {} unsynced  •  last sync: {}",
        app.project_count,
        total,
        app.unsynced_count,
        app.last_sync.as_deref().unwrap_or("never"),
    );
    let paragraph = ratatui::widgets::Paragraph::new(text)
        .style(ratatui::style::Style::new().fg(ratatui::style::Color::DarkGray));
    f.render_widget(paragraph, area);
}
