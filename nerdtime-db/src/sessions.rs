// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use nerdtime_core::Session;
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::git;

pub fn start_session(
    conn: &Connection,
    project: &str,
    desc: Option<&str>,
    task_id: Option<&str>,
    estimated_seconds: Option<i64>,
    labels: Option<&str>,
) -> Result<Session> {
    let active: bool = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE ended_at IS NULL",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0;

    if active {
        anyhow::bail!("An active session already exists. Stop it first.");
    }

    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    let branch = git::git_branch();
    let commit = git::git_commit_hash();

    conn.execute(
        "INSERT INTO sessions (id, project_name, branch_name, commit_hash, description, started_at, task_id, estimated_seconds, labels) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id.to_string(), project, branch, commit, desc, now, task_id, estimated_seconds, labels],
    )?;

    Ok(Session {
        id,
        project_name: project.to_string(),
        branch_name: branch,
        commit_hash: commit,
        description: desc.map(|s| s.to_string()),
        started_at: DateTime::parse_from_rfc3339(&now)
            .map(|d| d.to_utc())
            .unwrap_or_default(),
        ended_at: None,
        is_synced: false,
        task_id: task_id.map(|s| s.to_string()),
        estimated_seconds,
        labels: labels.map(|s| s.to_string()),
    })
}

pub fn stop_session(conn: &Connection) -> Result<Session> {
    let now = Utc::now().to_rfc3339();
    let updated = conn.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE ended_at IS NULL",
        params![now],
    )?;

    if updated == 0 {
        anyhow::bail!("No active session to stop.");
    }

    let session = conn
        .query_row(
            "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced, task_id, estimated_seconds, labels FROM sessions WHERE ended_at = ?1",
            params![now],
            map_session,
        )
        .context("failed to read stopped session")?;

    Ok(session)
}

pub fn show_status(conn: &Connection) -> Result<Option<Session>> {
    let session = conn
        .query_row(
            "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced, task_id, estimated_seconds, labels FROM sessions WHERE ended_at IS NULL LIMIT 1",
            [],
            map_session,
        )
        .ok();
    Ok(session)
}

pub fn list_sessions(
    conn: &Connection,
    project: Option<&str>,
    limit: usize,
) -> Result<Vec<Session>> {
    let (query, param_count): (&str, u8) = if project.is_some() {
        ("SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced, task_id, estimated_seconds, labels FROM sessions WHERE project_name = ?1 ORDER BY started_at DESC LIMIT ?2", 2)
    } else {
        ("SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced, task_id, estimated_seconds, labels FROM sessions ORDER BY started_at DESC LIMIT ?1", 1)
    };

    let mut stmt = conn.prepare(query)?;
    let sessions = if param_count == 2 {
        stmt.query_map(params![project, limit as i64], map_session)?
            .filter_map(|r| r.ok())
            .collect()
    } else {
        stmt.query_map(params![limit as i64], map_session)?
            .filter_map(|r| r.ok())
            .collect()
    };

    Ok(sessions)
}

pub fn get_unsynced_sessions(conn: &Connection) -> Result<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced, task_id, estimated_seconds, labels FROM sessions WHERE is_synced = 0 AND ended_at IS NOT NULL",
    )?;

    let sessions = stmt
        .query_map([], map_session)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

pub fn mark_synced(conn: &Connection) -> Result<usize> {
    let count = conn.execute("UPDATE sessions SET is_synced = 1 WHERE is_synced = 0", [])?;
    Ok(count)
}

pub fn stats_by_project(conn: &Connection) -> Result<Vec<nerdtime_core::ProjectStat>> {
    let mut stmt = conn.prepare(
        "SELECT project_name, SUM(CAST((julianday(ended_at) - julianday(started_at)) * 86400 AS INTEGER)) as total_seconds, COUNT(*) as session_count FROM sessions WHERE ended_at IS NOT NULL GROUP BY project_name ORDER BY total_seconds DESC",
    )?;

    let stats = stmt
        .query_map([], |row| {
            Ok(nerdtime_core::ProjectStat {
                project: row.get(0)?,
                total_seconds: row.get(1)?,
                session_count: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(stats)
}

fn map_session(row: &rusqlite::Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
        project_name: row.get(1)?,
        branch_name: row.get(2)?,
        commit_hash: row.get(3)?,
        description: row.get(4)?,
        started_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
            .map(|d| d.to_utc())
            .unwrap_or_default(),
        ended_at: row.get::<_, Option<String>>(6)?.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .ok()
                .map(|d| d.to_utc())
        }),
        is_synced: row.get::<_, i64>(7)? != 0,
        task_id: row.get(8)?,
        estimated_seconds: row.get(9)?,
        labels: row.get(10)?,
    })
}
