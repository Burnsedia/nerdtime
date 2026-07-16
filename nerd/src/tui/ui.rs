// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::{App, Mode, Panel, SyncStatus};
use crate::tui::modals;
use crate::tui::panels;
use crate::tui::widgets::truncate;

pub fn draw(f: &mut Frame, app: &App) {
    let terminal_size = f.size();
    if terminal_size.width < 80 || terminal_size.height < 24 {
        render_size_warning(f, terminal_size);
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(terminal_size);

    render_status_bar(f, chunks[0], app);
    render_panel_content(f, chunks[1], app);
    render_footer_help(f, chunks[2], app);

    if app.active_modal.is_some() {
        modals::render_modal(f, chunks[1], app);
    }
}

fn render_size_warning(f: &mut Frame, area: Rect) {
    let warning = Paragraph::new("Please resize terminal to at least 80x24")
        .style(Style::new().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(warning, area);
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let mode_style = match app.mode {
        Mode::Normal => Style::new()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        Mode::Insert(_) => Style::new()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        Mode::Command(_) => Style::new()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    };

    let mode_text = match &app.mode {
        Mode::Normal => " NORMAL ".to_string(),
        Mode::Insert(_) => " INSERT ".to_string(),
        Mode::Command(cmd) => {
            if cmd.is_empty() { " COMMAND ".to_string() } else { format!(" :{}", cmd) }
        }
    };

    let panel_name = match app.active_panel {
        Panel::Dashboard => "Dashboard",
        Panel::Stats => "Stats",
        Panel::Tasks => "Tasks",
        Panel::Matrix => "Matrix",
        Panel::Devlog => "Devlog",
        Panel::Advisor => "Advisor",
    };

    let sync_indicator = match app.sync_status {
        SyncStatus::Idle => {
            if app.unsynced_count > 0 {
                Span::styled(
                    format!(" ● {} unsynced", app.unsynced_count),
                    Style::new().fg(Color::Yellow),
                )
            } else {
                Span::styled(" ✓ all synced", Style::new().fg(Color::Green))
            }
        }
        SyncStatus::Syncing => Span::styled(" ↻ syncing...", Style::new().fg(Color::Cyan)),
        SyncStatus::Success(_) => Span::styled(" ✓ synced", Style::new().fg(Color::Green)),
        SyncStatus::Failure(ref msg) => {
            Span::styled(format!(" ✗ {}", truncate(msg, 20)), Style::new().fg(Color::Red))
        }
        SyncStatus::NoConfig => Span::styled(" ⚠ not configured", Style::new().fg(Color::Yellow)),
    };

    let left = format!(" {}  │  nerdtime v0.1.0  │  Panel: {}  ", mode_text, panel_name);
    let line = Line::from(vec![
        Span::styled(left, mode_style),
        sync_indicator,
    ]);

    let bar = Paragraph::new(line)
        .style(Style::new().bg(Color::Rgb(30, 30, 30)));
    f.render_widget(bar, area);
}

fn render_panel_content(f: &mut Frame, area: Rect, app: &App) {
    match app.active_panel {
        Panel::Dashboard => panels::dashboard::render(f, area, app),
        Panel::Stats => panels::stats::render(f, area, app),
        Panel::Tasks => panels::tasks::render(f, area, app),
        Panel::Matrix => panels::matrix::render(f, area, app),
        Panel::Devlog => panels::devlog::render(f, area, app),
        Panel::Advisor => panels::advisor::render(f, area, app),
    }
}

fn render_footer_help(f: &mut Frame, area: Rect, app: &App) {
    let help_text = match app.active_panel {
        Panel::Dashboard => "[Tab] Cycle  [n] New  [s] Sync  [/] Filter  [?] Help  [q] Quit",
        Panel::Stats => "[i] Insights  [h] Heatmap  [n] New  [Tab] Cycle",
        Panel::Tasks => "[a] Add  [c] Complete  [x] Cancel  [Enter] Start  [m] Matrix  [Tab] Cycle",
        Panel::Matrix => "[↑↓←→] Navigate  [Enter] View  [l] Task list  [Tab] Cycle",
        Panel::Devlog => "[↑↓] Navigate  [Enter] View  [/] Search  [n] New  [e] Edit  [Tab] Cycle",
        Panel::Advisor => "[Enter] Analyze  [s] Start suggested  [Tab] Cycle",
    };

    let paragraph = Paragraph::new(help_text)
        .style(Style::new().fg(Color::DarkGray));
    f.render_widget(paragraph, area);
}
