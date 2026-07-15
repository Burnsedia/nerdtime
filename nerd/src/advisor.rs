// SPDX-License-Identifier: AGPL-3.0-only
use crate::db;
use crate::insights;
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};
use rusqlite::Connection;

pub struct AdvisorInput {
    pub available_seconds: i64,
    pub energy: String,
    pub blocked: Option<String>,
}

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

    let input = AdvisorInput {
        available_seconds,
        energy,
        blocked,
    };

    let result = decide(conn, &input)?;

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

pub struct Advice {
    pub task_id: Option<String>,
    pub task_title: String,
    pub project: String,
    pub reason: String,
}

pub fn decide(conn: &Connection, input: &AdvisorInput) -> anyhow::Result<Advice> {
    let tasks = db::unsynced_active_tasks(conn, input.available_seconds, &input.energy)?;

    if tasks.is_empty() {
        return Ok(Advice {
            task_id: None,
            task_title: "Take a break".to_string(),
            project: String::new(),
            reason: "No tasks fit your available time and energy level.".to_string(),
        });
    }

    for task in &tasks {
        if task.quadrant == 1 {
            let fits = task
                .estimated_seconds
                .map(|e| e <= (input.available_seconds as f64 * 1.5) as i64)
                .unwrap_or(true);
            if fits {
                let mut reason = format!(
                    "Top priority Q1 task. Fits your {} block.",
                    insights::fmt_duration(input.available_seconds)
                );
                if let Some(ref b) = input.blocked {
                    if task.title.to_lowercase().contains(&b.to_lowercase()) {
                        continue;
                    }
                    reason.push_str(&format!(
                        " You mentioned you're blocked on \"{}\" — this task doesn't seem related.",
                        b
                    ));
                }
                if input.energy == "low" && task.estimated_seconds.is_some_and(|e| e > 1800) {
                    // Skip big Q1 tasks on low energy, check next
                    continue;
                }
                return Ok(Advice {
                    task_id: Some(task.id.clone()),
                    task_title: task.title.clone(),
                    project: task.project_name.clone(),
                    reason,
                });
            }
        }
    }

    for task in &tasks {
        if task.quadrant == 2 {
            let fits = task
                .estimated_seconds
                .map(|e| e <= (input.available_seconds as f64 * 1.5) as i64)
                .unwrap_or(true);
            if fits {
                return Ok(Advice {
                    task_id: Some(task.id.clone()),
                    task_title: task.title.clone(),
                    project: task.project_name.clone(),
                    reason: "Schedule it: important but not urgent. Good use of time.".to_string(),
                });
            }
        }
    }

    for task in &tasks {
        if task.quadrant == 3 {
            let fits = task
                .estimated_seconds
                .map(|e| e <= (input.available_seconds as f64 * 1.5) as i64)
                .unwrap_or(true);
            if fits {
                return Ok(Advice {
                    task_id: Some(task.id.clone()),
                    task_title: task.title.clone(),
                    project: task.project_name.clone(),
                    reason: "Quick task you can delegate later, but do it now if it's fast."
                        .to_string(),
                });
            }
        }
    }

    if let Some(task) = tasks.first() {
        return Ok(Advice {
            task_id: Some(task.id.clone()),
            task_title: task.title.clone(),
            project: task.project_name.clone(),
            reason: "Low-priority task. Consider if this needs doing at all.".to_string(),
        });
    }

    Ok(Advice {
        task_id: None,
        task_title: "Take a break".to_string(),
        project: String::new(),
        reason: "No tasks match your current context.".to_string(),
    })
}
