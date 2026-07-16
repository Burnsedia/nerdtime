// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use nerdtime_core::{Advice, AdvisorInput};
use rusqlite::Connection;

use crate::tasks;
use crate::util;

pub fn decide(conn: &Connection, input: &AdvisorInput) -> Result<Advice> {
    let tasks = tasks::unsynced_active_tasks(conn, input.available_seconds, &input.energy)?;

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
                    util::fmt_duration(input.available_seconds)
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
