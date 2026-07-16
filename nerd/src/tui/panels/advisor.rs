// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(6),
        Constraint::Min(3),
    ])
    .split(area);

    render_form(f, chunks[0], app);
    render_result(f, chunks[1], app);
}

fn render_form(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" What Should I Work On? ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::raw(format!(
            "  Available time: [{}]",
            if app.advisor_time.is_empty() { "2h" } else { &app.advisor_time }
        ))),
        Line::from(Span::raw(format!(
            "  Energy level:   [{}]  (low / medium / high)",
            if app.advisor_energy.is_empty() { "medium" } else { &app.advisor_energy }
        ))),
        Line::from(Span::raw(format!(
            "  Blocked on:     [{}]",
            if app.advisor_blocked.is_empty() { "optional" } else { &app.advisor_blocked }
        ))),
    ];

    let paragraph = Paragraph::new(Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_result(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Suggestion ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if let Some(ref advice) = app.advisor_result {
        let items = vec![
            Line::from(Span::styled(
                format!("  {}", advice.task_title),
                Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("")),
            Line::from(Span::raw(format!("  Project: {}", advice.project))),
            Line::from(Span::raw("")),
            Line::from(Span::raw(format!("  {}", advice.reason))),
        ];

        let paragraph = Paragraph::new(Text::from(items));
        f.render_widget(paragraph, inner);
    } else {
        let empty = Paragraph::new(Text::from(vec![
            Line::from(Span::raw("")),
            Line::from(Span::styled(
                "  Press [Enter] or fill in the form above to get a suggestion",
                Style::new().fg(Color::DarkGray),
            )),
        ]));
        f.render_widget(empty, inner);
    }
}
