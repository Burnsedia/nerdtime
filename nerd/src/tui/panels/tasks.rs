// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};
use nerdtime_db as db;

use crate::tui::app::App;
use crate::tui::widgets::truncate;

fn quadrant_color(q: u8) -> Color {
    match q {
        1 => Color::Red,
        2 => Color::Yellow,
        3 => Color::Blue,
        _ => Color::DarkGray,
    }
}

fn quadrant_label(q: u8) -> &'static str {
    match q {
        1 => "Q1",
        2 => "Q2",
        3 => "Q3",
        _ => "Q4",
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Tasks (active) ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.tasks.is_empty() {
        let text = ratatui::text::Text::from(vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    " No tasks — add one with [a]",
                    Style::new().fg(Color::DarkGray),
                ),
            ),
        ]);
        let paragraph = ratatui::widgets::Paragraph::new(text);
        f.render_widget(paragraph, inner);
        return;
    }

    let header = Line::from(vec![
        Span::styled(" Status ", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("Title                 ", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("Est      ", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("Actual   ", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("Remaining", Style::new().add_modifier(Modifier::BOLD)),
        Span::styled("  Q", Style::new().add_modifier(Modifier::BOLD)),
    ]);

    let mut items = vec![Line::from("")];
    items.push(header);

    for (i, task) in app.tasks.iter().enumerate() {
        let q_color = match task.quadrant {
            1 => Color::Red,
            2 => Color::Yellow,
            3 => Color::Blue,
            _ => Color::DarkGray,
        };
        let q_label = match task.quadrant {
            1 => "Q1",
            2 => "Q2",
            3 => "Q3",
            _ => "Q4",
        };
        let est_str = task
            .estimated_seconds
            .map(db::fmt_duration)
            .unwrap_or_else(|| "—".to_string());
        let act_str = db::fmt_duration(task.actual_seconds);
        let remaining = task
            .estimated_seconds
            .map(|e| db::fmt_duration((e - task.actual_seconds).max(0)))
            .unwrap_or_else(|| "—".to_string());
        let status_icon = if task.status == "active" {
            "●"
        } else if task.status == "completed" {
            "✓"
        } else {
            "○"
        };

        let is_selected = i == app.selected_index;
        let row_style = if is_selected {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new()
        };

        let row = Line::from(vec![
            Span::styled(format!(" {}  ", status_icon), q_color),
            Span::styled(
                truncate(&task.title, 22),
                row_style.fg(q_color),
            ),
            Span::styled(format!(" {:<8} ", est_str), row_style),
            Span::styled(format!(" {:<8} ", act_str), row_style),
            Span::styled(format!(" {:<9}", remaining), row_style),
            Span::styled(q_label, Style::new().fg(q_color).add_modifier(Modifier::BOLD)),
        ]);
        items.push(row);
    }

    let list = ratatui::widgets::List::new(items);
    f.render_widget(list, inner);
}
