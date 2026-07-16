// SPDX-License-Identifier: AGPL-3.0-only
pub mod app;
pub mod modals;
pub mod ui;
pub mod widgets;
pub mod panels;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nerdtime_db as db;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, Write};

pub fn run(conn: &db::Connection) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config = crate::config::load().ok();
    let mut app = app::App::new(conn, &config);

    let res = run_loop(&mut terminal, &mut app, conn);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut app::App,
    conn: &db::Connection,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(250))? {
            match event::read()? {
                Event::Key(key) => {
                    if app.handle_key(key, conn)? {
                        break;
                    }
                }
                Event::Resize(w, h) => {
                    app.terminal_size = (w, h);
                }
                _ => {}
            }
        }

        app.tick();
        app.refresh_if_needed(conn);
    }
    Ok(())
}
