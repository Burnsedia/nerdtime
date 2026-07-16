// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use nerdtime_db as db;

use crate::tui::app::App;
use crate::tui::widgets::{truncate, SparklineBar};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Time per Project ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.stats.is_empty() {
        let text = ratatui::text::Text::from(vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    " No sessions yet — start tracking with [n]",
                    Style::new().fg(Color::DarkGray),
                ),
            ),
        ]);
        let paragraph = ratatui::widgets::Paragraph::new(text);
        f.render_widget(paragraph, inner);
        return;
    }

    let max_seconds = app
        .stats
        .first()
        .map(|s| s.total_seconds)
        .unwrap_or(1)
        .max(1) as f64;

    let mut lines = vec![ratatui::text::Line::from("")];
    for (i, stat) in app.stats.iter().enumerate() {
        let is_selected = i == app.selected_index;
        let prefix = if is_selected { "> " } else { "  " };
        let row_style = if is_selected {
            Style::new().fg(Color::Cyan).add_modifier(Modifier::REVERSED)
        } else {
            Style::new().fg(Color::Cyan)
        };
        let fraction = stat.total_seconds as f64 / max_seconds;
        let bar_width = inner.width.saturating_sub(32) as usize;
        let filled = (bar_width as f64 * fraction) as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar = format!(
            "{}{:<15} {}{}  {}",
            prefix,
            truncate(&stat.project, 15),
            "█".repeat(filled),
            "░".repeat(empty),
            db::fmt_duration(stat.total_seconds),
        );
        lines.push(Line::from(Span::styled(bar, row_style)));
    }

    let total = db::fmt_duration(app.total_duration);
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!(" Total: {} across {} projects  •  {} sessions", total, app.project_count, app.total_sessions_count),
            Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
    ]));

    let paragraph = ratatui::widgets::Paragraph::new(ratatui::text::Text::from(lines));
    f.render_widget(paragraph, inner);
}
