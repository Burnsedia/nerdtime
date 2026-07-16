// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};
use nerdtime_db as db;

use crate::tui::app::App;
use crate::tui::widgets::truncate;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Eisenhower Matrix ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(inner);
    let top = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(chunks[0]);
    let bottom = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(chunks[1]);

    render_quadrant(f, top[0], app, 1, "Q1: Do First", "urgency >3, importance >3", Color::Red);
    render_quadrant(f, top[1], app, 2, "Q2: Schedule", "urgency ≤3, importance >3", Color::Yellow);
    render_quadrant(f, bottom[0], app, 3, "Q3: Delegate", "urgency >3, importance ≤3", Color::Blue);
    render_quadrant(f, bottom[1], app, 4, "Q4: Eliminate", "urgency ≤3, importance ≤3", Color::DarkGray);
}

fn render_quadrant(f: &mut Frame, area: Rect, app: &App, q: u8, title: &str, subtitle: &str, color: Color) {
    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::new().fg(color));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = vec![
        Line::from(Span::styled(subtitle, Style::new().fg(Color::DarkGray))),
        Line::from(""),
    ];

    let q_tasks: Vec<_> = app.tasks.iter().filter(|t| t.quadrant == q).collect();

    if q_tasks.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No tasks",
            Style::new().fg(Color::DarkGray),
        )));
    } else {
        for task in &q_tasks {
            let est = task
                .estimated_seconds
                .map(db::fmt_duration)
                .unwrap_or_else(|| "—".to_string());
            lines.push(Line::from(vec![
                Span::styled(" ● ", Style::new().fg(color)),
                Span::styled(truncate(&task.title, 20), Style::new().fg(color)),
                Span::styled(format!(" {}", est), Style::new().fg(color)),
            ]));
        }
    }

    let paragraph = ratatui::widgets::Paragraph::new(ratatui::text::Text::from(lines));
    f.render_widget(paragraph, inner);
}