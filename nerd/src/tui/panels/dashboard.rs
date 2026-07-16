// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use nerdtime_db as db;

use crate::tui::app::{App, Panel};
use crate::tui::widgets::truncate;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(8),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);

    render_active_session(f, chunks[0], app);
    render_recent_sessions(f, chunks[1], app);
    render_footer(f, chunks[2], app);
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
            Span::styled(
                truncate(&session.project_name, 12),
                row_style,
            ),
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
