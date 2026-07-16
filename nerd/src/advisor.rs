// SPDX-License-Identifier: AGPL-3.0-only
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};
use nerdtime_db::{self as db, Connection};

pub fn run_interactive(conn: &Connection) -> anyhow::Result<()> {
    println!(
        "\n{} Let's figure it out. I'll ask a few questions.\n",
        "🧭".cyan()
    );

    let time_str: String = Input::new()
        .with_prompt("How much time do you have")
        .default("1h".to_string())
        .interact_text()?;
    let available_seconds = db::parse_duration(&time_str)?.unwrap_or(3600);

    let energy_idx = Select::new()
        .with_prompt("Energy level")
        .items(&["low", "medium", "high"])
        .default(1)
        .interact()?;
    let energy = ["low", "medium", "high"][energy_idx].to_string();

    let blocked: String = Input::new()
        .with_prompt("Are you blocked on anything? (leave blank if not)")
        .allow_empty(true)
        .default(String::new())
        .interact_text()?;
    let blocked = if blocked.is_empty() {
        None
    } else {
        Some(blocked)
    };

    let input = db::AdvisorInput {
        available_seconds,
        energy,
        blocked,
    };

    let result = db::decide(conn, &input)?;

    println!("\n  {}", "Suggestion:".bold());
    println!("    {} — {}", result.task_title.bold(), result.reason);

    if let Some(tid) = result.task_id {
        let start = Confirm::new()
            .with_prompt("Start tracking this task?")
            .default(true)
            .interact()?;
        if start {
            let task_labels: Option<String> = conn
                .query_row(
                    "SELECT labels FROM tasks WHERE id = ?1",
                    rusqlite::params![tid],
                    |row| row.get(0),
                )
                .ok()
                .flatten();
            db::start_session(
                conn,
                &result.project,
                None,
                Some(&tid),
                None,
                task_labels.as_deref(),
            )?;
        }
    }

    Ok(())
}
