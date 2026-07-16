// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, Modal};
use crate::tui::widgets::centered_rect;

pub fn render_modal(f: &mut Frame, area: Rect, app: &App) {
    let Some(ref modal) = app.active_modal else {
        return;
    };

    let modal_area = centered_rect(area, 60, 50);
    f.render_widget(Clear, modal_area);

    match modal {
        Modal::NewSession => render_new_session_form(f, modal_area, app),
        Modal::NewTask => render_new_task_form(f, modal_area, app),
        Modal::NewDevlogEntry => render_new_devlog_form(f, modal_area, app),
        Modal::Help => render_help_overlay(f, modal_area),
        Modal::Confirm { message, .. } => render_confirm_dialog(f, modal_area, message),
        Modal::FilterInput => render_filter_input(f, modal_area, app),
        Modal::AdvisorForm => render_advisor_form(f, modal_area, app),
        Modal::TaskDetail(idx) => render_task_detail(f, modal_area, app, *idx),
        Modal::DevlogDetail(idx) => render_devlog_detail(f, modal_area, app, *idx),
        Modal::Heatmap => render_heatmap(f, modal_area, app),
        Modal::Insights => render_insights_panel(f, modal_area, app),
    }
}

fn render_new_session_form(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" New Session ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled("Project:", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.new_session_project))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Description:", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.new_session_desc))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(" [Enter] Save  [Esc] Cancel", Style::new().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_new_devlog_form(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" New Devlog Entry ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled("Title:", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.new_devlog_title))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Role:", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.new_devlog_role))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Tags (comma-separated):", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.new_devlog_tags))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(" [Enter] Next field  [Esc] Cancel", Style::new().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_new_task_form(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" New Task ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled("Title:", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.new_task_title))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Estimate (e.g. 2h):", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.new_task_estimate))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Labels (comma-separated):", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.new_task_labels))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(" [Enter] Save  [Esc] Cancel", Style::new().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_help_overlay(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let help_items = vec![
        "NORMAL MODE:",
        "  j/k/↑/↓  Navigate lists",
        "  Tab/gt   Next panel",
        "  Shift+Tab/gT  Previous panel",
        "  n        New item (session/task/entry)",
        "  s        Sync sessions",
        "  r        Force refresh",
        "  /        Filter/Search",
        "  :        Command mode",
        "  ?        This help",
        "  q        Quit",
        "  dd       Delete (with confirm)",
        "  cc       Complete selected task",
        "  Enter    Stop session / View detail",
        "",
        "PANEL KEYS:",
        "  Tasks: a=Add, c=Complete, x=Cancel, m=Matrix",
        "  Stats: i=Insights, h=Heatmap",
        "  Devlog: e=Edit",
        "  Advisor: s=Start suggested task",
        "",
        "COMMANDS: :q, :w (sync), :wq, :e (refresh), :dashboard, :stats, :tasks, :matrix, :devlog, :advisor",
        "",
        "Press Esc to close",
    ];

    let text: Vec<Line> = help_items.iter().map(|s| Line::from(Span::raw(*s))).collect();
    let paragraph = Paragraph::new(Text::from(text));
    f.render_widget(paragraph, inner);
}

fn render_confirm_dialog(f: &mut Frame, area: Rect, message: &str) {
    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Yellow));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::raw("")),
        Line::from(Span::styled(message, Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(" [Enter] Confirm  [Esc] Cancel", Style::new().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_filter_input(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Filter ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled("Filter text:", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.filter_text))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(" [Enter] Apply  [Esc] Cancel", Style::new().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_advisor_form(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Advisor ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(Span::styled("Available time (e.g. 2h):", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.advisor_time))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Energy level (low/medium/high):", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.advisor_energy))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Blocked on (optional):", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw(format!("  {}", app.advisor_blocked))),
        Line::from(Span::raw("")),
        Line::from(Span::styled(" [Enter] Analyze  [Esc] Cancel", Style::new().fg(Color::DarkGray))),
    ];

    let paragraph = Paragraph::new(Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_task_detail(f: &mut Frame, area: Rect, app: &App, idx: usize) {
    let block = Block::default()
        .title(" Task Detail ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    if let Some(task) = app.tasks.get(idx) {
        let q_label = match task.quadrant {
            1 => "Q1: Do First",
            2 => "Q2: Schedule",
            3 => "Q3: Delegate",
            _ => "Q4: Eliminate",
        };
        let est = task.estimated_seconds
            .map(|s| nerdtime_db::fmt_duration(s))
            .unwrap_or_else(|| "—".to_string());
        let act = nerdtime_db::fmt_duration(task.actual_seconds);
        let status = if task.completed_at.is_some() { "✓ Completed" } else if task.status == "cancelled" { "✗ Cancelled" } else { "● Active" };

        let items = vec![
            Line::from(Span::styled(&task.title, Style::new().add_modifier(Modifier::BOLD))),
            Line::from(Span::raw("")),
            Line::from(Span::raw(format!("Project:   {}", task.project_name))),
            Line::from(Span::raw(format!("Status:    {}", status))),
            Line::from(Span::raw(format!("Quadrant:  {}", q_label))),
            Line::from(Span::raw(format!("Estimate:  {}", est))),
            Line::from(Span::raw(format!("Actual:    {}", act))),
            Line::from(Span::raw("")),
            Line::from(Span::styled(" [Esc] Close", Style::new().fg(Color::DarkGray))),
        ];
        let paragraph = Paragraph::new(Text::from(items));
        f.render_widget(paragraph, inner);
    }
}

fn render_devlog_detail(f: &mut Frame, area: Rect, app: &App, idx: usize) {
    let block = Block::default()
        .title(" Devlog Entry ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    if let Some(entry) = app.devlog_entries.get(idx) {
        let tags = if entry.tags.is_empty() {
            "—".to_string()
        } else {
            entry.tags.join(", ")
        };

        let mut items = vec![
            Line::from(Span::styled(format!("{}: {}", entry.date, entry.title), Style::new().add_modifier(Modifier::BOLD))),
            Line::from(Span::raw("")),
            Line::from(Span::raw(format!("Role: {}", entry.role))),
            Line::from(Span::raw(format!("Tags: {}", tags))),
            Line::from(Span::raw("")),
        ];

        if !entry.context.is_empty() {
            items.push(Line::from(Span::styled("Context:", Style::new().add_modifier(Modifier::BOLD))));
            items.push(Line::from(Span::raw(&entry.context)));
            items.push(Line::from(Span::raw("")));
        }

        if !entry.changes.is_empty() {
            items.push(Line::from(Span::styled("Changes:", Style::new().add_modifier(Modifier::BOLD))));
            for c in &entry.changes {
                items.push(Line::from(Span::raw(format!("  - {}", c))));
            }
            items.push(Line::from(Span::raw("")));
        }

        if !entry.decisions.is_empty() {
            items.push(Line::from(Span::styled("Decisions:", Style::new().add_modifier(Modifier::BOLD))));
            for d in &entry.decisions {
                items.push(Line::from(Span::raw(format!("  - {}", d))));
            }
            items.push(Line::from(Span::raw("")));
        }

        items.push(Line::from(Span::styled(" [Esc] Close", Style::new().fg(Color::DarkGray))));

        let paragraph = Paragraph::new(Text::from(items));
        f.render_widget(paragraph, inner);
    }
}

fn render_heatmap(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Heatmap ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    if app.heatmap_data.is_empty() {
        let text = Paragraph::new("No heatmap data available.")
            .style(Style::new().fg(Color::DarkGray));
        f.render_widget(text, inner);
        return;
    }

    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let max_val = app.heatmap_data.iter().map(|c| c.total_seconds).max().unwrap_or(3600).max(1);

    let mut lines = vec![Line::from(Span::raw("     "))];
    for h in 0..24 {
        lines[0].push_span(Span::raw(format!("{:>3}", h)));
    }

    for (d, day) in days.iter().enumerate() {
        let mut line = Line::from(Span::raw(format!(" {:>3} ", day)));
        for h in 0..24 {
            let seconds = app.heatmap_data
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

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(" [Esc] Close", Style::new().fg(Color::DarkGray))));

    let paragraph = Paragraph::new(Text::from(lines));
    f.render_widget(paragraph, inner);
}

fn render_insights_panel(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" Insights ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let Some(ref insights) = app.insights_data else {
        let text = Paragraph::new("No insights data available.")
            .style(Style::new().fg(Color::DarkGray));
        f.render_widget(text, inner);
        return;
    };

    let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let best_day_idx = insights.per_day_of_week.iter().enumerate()
        .max_by_key(|&(_, &v)| v)
        .map(|(i, _)| i)
        .unwrap_or(0);
    let best_hour_block = insights.per_block.iter().enumerate()
        .max_by_key(|&(_, &v)| v)
        .map(|(i, _)| match i { 0 => "6-12", 1 => "12-18", 2 => "18-24", _ => "0-6" })
        .unwrap_or("—");

    let total_days: i64 = insights.per_day_of_week.iter().filter(|&&v| v > 0).count() as i64;
    let avg_daily = if total_days > 0 { insights.total_seconds / total_days } else { 0 };

    let top_project = insights.per_project.iter()
        .max_by_key(|&(_, s)| s)
        .map(|(p, _)| p.as_str())
        .unwrap_or("—");

    let mut items = vec![
        Line::from(Span::styled("Productivity Insights", Style::new().add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("")),
        Line::from(Span::raw(format!("Total tracked:  {}", nerdtime_db::fmt_duration(insights.total_seconds)))),
        Line::from(Span::raw(format!("Total sessions: {}", insights.session_count))),
        Line::from(Span::raw(format!("Avg per day:    {}", nerdtime_db::fmt_duration(avg_daily)))),
        Line::from(Span::raw(format!("Best day:       {}", day_names[best_day_idx]))),
        Line::from(Span::raw(format!("Peak block:     {}", best_hour_block))),
        Line::from(Span::raw("")),
        Line::from(Span::raw(format!("Top project:    {}", top_project))),
        Line::from(Span::raw("")),
    ];

    if !insights.per_project.is_empty() {
        items.push(Line::from(Span::styled("Per project:", Style::new().add_modifier(Modifier::BOLD))));
        for (project, seconds) in &insights.per_project {
            items.push(Line::from(Span::raw(format!("  {}  {}", project, nerdtime_db::fmt_duration(*seconds)))));
        }
        items.push(Line::from(Span::raw("")));
    }

    items.push(Line::from(Span::styled(" [Esc] Close", Style::new().fg(Color::DarkGray))));

    let paragraph = Paragraph::new(Text::from(items));
    f.render_widget(paragraph, inner);
}
