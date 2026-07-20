// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders},
    Frame,
};

use crate::tui::app::App;
use crate::tui::widgets::truncate;

fn tags_str(tags: &[String]) -> String {
    if tags.is_empty() {
        "—".to_string()
    } else {
        tags.join(", ")
    }
}

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Devlog Entries ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.devlog_entries.is_empty() {
        let text = ratatui::text::Text::from(vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from(
                ratatui::text::Span::styled(
                    " No entries — log one with [n]",
                    Style::new().fg(Color::DarkGray),
                ),
            ),
        ]);
        let paragraph = ratatui::widgets::Paragraph::new(text);
        f.render_widget(paragraph, inner);
        return;
    }

    let header = Line::from(vec![
        Span::styled(" Date       ", ratatui::style::Style::new().add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled("Title                        ", ratatui::style::Style::new().add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled("Role    ", ratatui::style::Style::new().add_modifier(ratatui::style::Modifier::BOLD)),
        Span::styled("Tags", ratatui::style::Style::new().add_modifier(ratatui::style::Modifier::BOLD)),
    ]);

    let mut items = vec![Line::from("")];
    items.push(header);

    for (i, entry) in app.devlog_entries.iter().enumerate() {
        let tags = tags_str(&entry.tags);

        let is_selected = i == app.selected_index;
        let row_style = if is_selected {
            ratatui::style::Style::new().add_modifier(ratatui::style::Modifier::REVERSED)
        } else {
            ratatui::style::Style::new()
        };

        let row = Line::from(vec![
            Span::styled(format!(" {:<10} ", entry.date), row_style),
            Span::styled(truncate(&entry.title, 28), row_style),
            Span::styled(format!(" {:<8} ", entry.role), row_style),
            Span::styled(truncate(&tags, 15), row_style),
        ]);
        items.push(row);
    }

    let list = ratatui::widgets::List::new(items);
    f.render_widget(list, inner);
}