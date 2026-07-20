// SPDX-License-Identifier: AGPL-3.0-only
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::Wrap,
    Frame,
};

pub fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup = Layout::vertical([
        Constraint::Length((area.height * (100 - percent_y)) / 200),
        Constraint::Length((area.height * percent_y) / 100),
        Constraint::Length((area.height * (100 - percent_y)) / 200),
    ])
    .split(area)[1];
    Layout::horizontal([
        Constraint::Length((area.width * (100 - percent_x)) / 200),
        Constraint::Length((area.width * percent_x) / 100),
        Constraint::Length((area.width * (100 - percent_x)) / 200),
    ])
    .split(popup)[1]
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

pub struct ScrollableList {
    pub items: Vec<String>,
    pub selected: usize,
    pub offset: usize,
}

impl ScrollableList {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            items,
            selected: 0,
            offset: 0,
        }
    }

    pub fn selected(&self) -> Option<usize> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.selected)
        }
    }

    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected).map(|s| s.as_str())
    }

    pub fn move_down(&mut self, max_visible: usize) {
        if self.items.is_empty() {
            return;
        }
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
        if self.selected >= self.offset + max_visible {
            self.offset += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        if self.selected < self.offset {
            self.offset = self.offset.saturating_sub(1);
        }
    }

    pub fn go_top(&mut self) {
        self.selected = 0;
        self.offset = 0;
    }

    pub fn go_bottom(&mut self) {
        self.selected = self.items.len().saturating_sub(1);
    }

    pub fn page_down(&mut self, page_size: usize) {
        let len = self.items.len();
        if len == 0 {
            return;
        }
        self.selected = (self.selected + page_size).min(len - 1);
        if self.selected >= self.offset + page_size {
            self.offset = self.selected.saturating_sub(page_size.saturating_sub(1));
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected = self.selected.saturating_sub(page_size);
        if self.selected < self.offset {
            self.offset = self.selected;
        }
    }
}

pub struct StatusBar;

impl StatusBar {
    pub fn render(f: &mut Frame, area: ratatui::layout::Rect, app: &super::app::App) {
        let mode_style = match app.mode {
            super::app::Mode::Normal => Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
            super::app::Mode::Insert(_) => {
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            }
            super::app::Mode::Command(_) => {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            }
        };

        let mode_text = match &app.mode {
            super::app::Mode::Normal => " NORMAL ".to_string(),
            super::app::Mode::Insert(_) => " INSERT ".to_string(),
            super::app::Mode::Command(cmd) => format!(" :{}", cmd),
        };

        let panel_name = match app.active_panel {
            super::app::Panel::Dashboard => "Dashboard",
            super::app::Panel::Stats => "Stats",
            super::app::Panel::Tasks => "Tasks",
            super::app::Panel::Matrix => "Matrix",
            super::app::Panel::Devlog => "Devlog",
            super::app::Panel::Advisor => "Advisor",
        };

        let sync_text = match &app.sync_status {
            super::app::SyncStatus::Idle => {
                let unsynced = app.unsynced_count;
                if unsynced > 0 {
                    format!(" ● {} unsynced", unsynced)
                } else {
                    " ✓ all synced".to_string()
                }
            }
            super::app::SyncStatus::Syncing => " ↻ syncing...".to_string(),
            super::app::SyncStatus::Success(n) => format!(" ✓ synced {} sessions", n),
            super::app::SyncStatus::Failure(e) => format!(" ✗ {}", e),
            super::app::SyncStatus::NoConfig => " ⚠ not configured".to_string(),
        };

        let sync_style = if sync_text.contains('✓') || sync_text.contains("synced") {
            Style::new().fg(Color::Green)
        } else if sync_text.contains('●') || sync_text.contains("⚠") {
            Style::new().fg(Color::Yellow)
        } else if sync_text.contains('✗') {
            Style::new().fg(Color::Red)
        } else {
            Style::new().fg(Color::Cyan)
        };

        let version = env!("CARGO_PKG_VERSION");

        let left = format!("{}│ nerdtime v{} │ Panel: {}", mode_text, version, panel_name);
        let right = sync_text;

        let bar = Line::from(vec![
            Span::styled(left, mode_style),
            Span::raw(" "),
            Span::styled(right, sync_style),
        ]);

        let paragraph = ratatui::widgets::Paragraph::new(bar);
        f.render_widget(paragraph, area);
    }
}

pub struct Toast;

impl Toast {
    pub fn render(f: &mut Frame, area: ratatui::layout::Rect, toast: &super::app::Toast) {
        let style = match toast.style {
            super::app::ToastStyle::Info => Style::new().fg(Color::Green),
            super::app::ToastStyle::Success => Style::new().fg(Color::Green),
            super::app::ToastStyle::Error => {
                Style::new().fg(Color::Red)
            }
        };

        let text = format!(" {} ", toast.message);
        let width = text.len() as u16 + 2;
        let x = area.width.saturating_sub(width);
        let y = area.height.saturating_sub(1);
        let rect = Rect::new(x, y, width.min(area.width), 1);

        let paragraph = ratatui::widgets::Paragraph::new(text)
            .style(style)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, rect);
    }
}

pub struct SparklineBar;

impl SparklineBar {
    pub fn render(
        f: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        fraction: f64,
        color: Color,
    ) {
        let bar_width = area.width.saturating_sub(label.len() as u16 + value.len() as u16 + 3);
        let filled = (bar_width as f64 * fraction.clamp(0.0, 1.0)) as u16;
        let empty = bar_width.saturating_sub(filled);

        let bar = format!(
            "{} {} {}",
            label,
            "█".repeat(filled as usize),
            "░".repeat(empty as usize),
        );

        let style = Style::new().fg(color);
        let paragraph = ratatui::widgets::Paragraph::new(bar).style(style);
        f.render_widget(paragraph, area);
    }
}

pub struct ModeIndicator;

impl ModeIndicator {
    pub fn render(f: &mut Frame, area: Rect, mode: &super::app::Mode) {
        let (text, style) = match mode {
            super::app::Mode::Normal => ("NORMAL".to_string(), Style::new().fg(Color::Green).add_modifier(Modifier::BOLD)),
            super::app::Mode::Insert(_) => ("INSERT".to_string(), Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            super::app::Mode::Command(cmd) => {
                (format!(":{}", cmd), Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            }
        };
        let paragraph = ratatui::widgets::Paragraph::new(text).style(style);
        f.render_widget(paragraph, area);
    }
}
