// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use chrono::Utc;
use nerdtime_core::TaskRow;
use rusqlite::{params, Connection};
use uuid::Uuid;

fn compute_quadrant(urgency: u8, importance: u8) -> u8 {
    if urgency > 3 && importance > 3 {
        1
    } else if urgency <= 3 && importance > 3 {
        2
    } else if urgency > 3 && importance <= 3 {
        3
    } else {
        4
    }
}

pub fn resolve_task_id(conn: &Connection, partial: &str) -> Result<String> {
    let mut stmt = conn.prepare("SELECT id FROM tasks WHERE id LIKE ?1 || '%'")?;
    let matches: Vec<String> = stmt
        .query_map(params![partial], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    match matches.len() {
        0 => anyhow::bail!("no task matches prefix: {}", partial),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => anyhow::bail!(
            "multiple tasks match prefix '{}': {}",
            partial,
            matches.join(", ")
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn add_task(
    conn: &Connection,
    project: &str,
    title: &str,
    desc: Option<&str>,
    est: Option<i64>,
    urgency: u8,
    importance: u8,
    labels: Option<&str>,
    github_repo: Option<&str>,
    github_issue_number: Option<i64>,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let quadrant = compute_quadrant(urgency, importance);
    conn.execute(
        "INSERT INTO tasks (id, project_name, title, description, estimated_seconds, urgency, importance, quadrant, labels, created_at, github_repo, github_issue_number) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![id, project, title, desc, est, urgency, importance, quadrant, labels, now, github_repo, github_issue_number],
    )?;
    Ok(id)
}

pub fn find_task_by_github_issue(
    conn: &Connection,
    repo: &str,
    number: i64,
) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT id FROM tasks WHERE github_repo = ?1 AND github_issue_number = ?2 AND status = 'active'",
        params![repo, number],
        |row| row.get(0),
    );
    match result {
        Ok(id) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn list_tasks(
    conn: &Connection,
    project: Option<&str>,
    status: Option<&str>,
) -> Result<Vec<TaskRow>> {
    let mut sql = String::from(
        "SELECT t.id, t.project_name, t.title, t.description, t.estimated_seconds, t.urgency, t.importance, t.quadrant, t.status, t.labels, t.created_at, t.completed_at, COALESCE(SUM(CAST((julianday(s.ended_at) - julianday(s.started_at)) * 86400 AS INTEGER)), 0) as actual_seconds, t.github_repo, t.github_issue_number FROM tasks t LEFT JOIN sessions s ON s.task_id = t.id AND s.ended_at IS NOT NULL",
    );
    let mut conditions: Vec<String> = Vec::new();
    if let Some(p) = project {
        conditions.push(format!("t.project_name = '{}'", p.replace('\'', "''")));
    }
    if let Some(st) = status {
        conditions.push(format!("t.status = '{}'", st.replace('\'', "''")));
    }
    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" GROUP BY t.id ORDER BY t.quadrant ASC, t.created_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(TaskRow {
                id: row.get(0)?,
                project_name: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                estimated_seconds: row.get(4)?,
                urgency: row.get(5)?,
                importance: row.get(6)?,
                quadrant: row.get(7)?,
                status: row.get(8)?,
                labels: row.get(9)?,
                created_at: row.get(10)?,
                completed_at: row.get(11)?,
                actual_seconds: row.get(12)?,
                github_repo: row.get(13)?,
                github_issue_number: row.get(14)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

pub fn complete_task(conn: &Connection, id: &str) -> Result<()> {
    let tid = resolve_task_id(conn, id)?;
    let active: bool = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE task_id = ?1 AND ended_at IS NULL",
        params![tid],
        |row| row.get::<_, i64>(0),
    )? > 0;
    if active {
        anyhow::bail!("Task has an active session. Stop it first.");
    }
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET status = 'completed', completed_at = ?1 WHERE id = ?2",
        params![now, tid],
    )?;
    Ok(())
}

pub fn cancel_task(conn: &Connection, id: &str) -> Result<()> {
    let tid = resolve_task_id(conn, id)?;
    conn.execute(
        "UPDATE tasks SET status = 'cancelled' WHERE id = ?1",
        params![tid],
    )?;
    Ok(())
}

pub fn edit_task(
    conn: &Connection,
    id: &str,
    title: Option<&str>,
    est: Option<Option<i64>>,
    urgency: Option<u8>,
    importance: Option<u8>,
    labels: Option<Option<&str>>,
) -> Result<()> {
    let tid = resolve_task_id(conn, id)?;
    let mut sets: Vec<String> = Vec::new();

    if let Some(t) = title {
        sets.push(format!("title = '{}'", t.replace('\'', "''")));
    }
    if let Some(e) = est {
        match e {
            Some(secs) => sets.push(format!("estimated_seconds = {}", secs)),
            None => sets.push("estimated_seconds = NULL".to_string()),
        }
    }
    if let Some(u) = urgency {
        let i = importance.unwrap_or({
            let cur: u8 = conn
                .query_row(
                    "SELECT importance FROM tasks WHERE id = ?1",
                    params![tid],
                    |row| row.get(0),
                )
                .unwrap_or(3);
            cur
        });
        let q = compute_quadrant(u, i);
        sets.push(format!("urgency = {}", u));
        sets.push(format!("quadrant = {}", q));
    }
    if let Some(i) = importance {
        let u = urgency.unwrap_or({
            let cur: u8 = conn
                .query_row(
                    "SELECT urgency FROM tasks WHERE id = ?1",
                    params![tid],
                    |row| row.get(0),
                )
                .unwrap_or(3);
            cur
        });
        let q = compute_quadrant(u, i);
        sets.push(format!("importance = {}", i));
        sets.push(format!("quadrant = {}", q));
    }
    if let Some(l) = labels {
        match l {
            Some(ls) => sets.push(format!("labels = '{}'", ls.replace('\'', "''"))),
            None => sets.push("labels = NULL".to_string()),
        }
    }

    if sets.is_empty() {
        anyhow::bail!("No changes specified.");
    }

    let sql = format!("UPDATE tasks SET {} WHERE id = ?", sets.join(", "));
    conn.execute(&sql, params![tid])?;
    Ok(())
}

pub fn task_estimate(
    conn: &Connection,
    id: &str,
) -> Result<(TaskRow, Vec<nerdtime_core::SessionEstimate>)> {
    let tid = resolve_task_id(conn, id)?;
    let task = list_tasks(conn, None, None)?
        .into_iter()
        .find(|t| t.id == tid)
        .ok_or_else(|| anyhow::anyhow!("task not found: {}", tid))?;

    let mut stmt = conn.prepare(
        "SELECT s.started_at, s.ended_at, s.estimated_seconds FROM sessions s WHERE s.task_id = ?1 AND s.ended_at IS NOT NULL ORDER BY s.started_at ASC",
    )?;
    let sessions = stmt
        .query_map(params![tid], |row| {
            let started: String = row.get(0)?;
            let ended: String = row.get(1)?;
            let dur = chrono::DateTime::parse_from_rfc3339(&ended)
                .unwrap_or_default()
                .signed_duration_since(
                    chrono::DateTime::parse_from_rfc3339(&started).unwrap_or_default(),
                );
            Ok((
                started,
                ended,
                dur.num_seconds(),
                row.get::<_, Option<i64>>(2)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok((task, sessions))
}

pub fn get_task_labels(conn: &Connection, id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT labels FROM tasks WHERE id = ?1",
            params![id],
            |row| row.get::<_, Option<String>>(0),
        )
        .unwrap_or(None))
}

pub fn get_task_github_info(conn: &Connection, id: &str) -> Result<(Option<String>, Option<i64>)> {
    let tid = resolve_task_id(conn, id)?;
    Ok(conn.query_row(
        "SELECT github_repo, github_issue_number FROM tasks WHERE id = ?1",
        params![tid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?)
}

pub fn unsynced_active_tasks(
    conn: &Connection,
    available_seconds: i64,
    energy: &str,
) -> Result<Vec<TaskRow>> {
    let all = list_tasks(conn, None, Some("active"))?;
    Ok(all
        .into_iter()
        .filter(|t| {
            if let Some(est) = t.estimated_seconds {
                est <= (available_seconds as f64 * 1.5) as i64
            } else {
                true
            }
        })
        .filter(|t| {
            if energy == "low" {
                t.quadrant != 1 || t.estimated_seconds.map_or(true, |e| e <= 1800)
            } else {
                true
            }
        })
        .collect())
}
