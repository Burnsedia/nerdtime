// SPDX-License-Identifier: AGPL-3.0-only
use crate::config;
use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use nerdtime_core::Session;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use uuid::Uuid;

fn data_dir() -> Result<PathBuf> {
    let path = dirs::config_dir()
        .context("config directory not found")?
        .join("nerdtime");
    std::fs::create_dir_all(&path).context("failed to create nerdtime config directory")?;
    Ok(path)
}

pub fn get_connection() -> Result<Connection> {
    let db_path = data_dir()?.join("data.db");
    let conn = Connection::open(&db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY NOT NULL,
            project_name TEXT NOT NULL,
            branch_name TEXT,
            commit_hash TEXT,
            description TEXT,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            is_synced INTEGER DEFAULT 0
        );",
    )?;
    Ok(conn)
}

fn git_branch() -> Option<String> {
    std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
}

fn git_commit_hash() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
}

pub fn start_session(conn: &Connection, project: &str, desc: Option<&str>) -> Result<()> {
    let active: bool = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE ended_at IS NULL",
        [],
        |row| row.get::<_, i64>(0),
    )? > 0;

    if active {
        anyhow::bail!("An active session already exists. Stop it first with `nerd stop`.");
    }

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let branch = git_branch();
    let commit = git_commit_hash();

    conn.execute(
        "INSERT INTO sessions (id, project_name, branch_name, commit_hash, description, started_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, project, branch, commit, desc, now],
    )?;

    println!("{} Tracking started for {}", "✓".green(), project.bold());
    if let Some(ref b) = branch {
        println!("  branch: {}", b.cyan());
    }
    Ok(())
}

pub fn stop_session(conn: &Connection) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let updated = conn.execute(
        "UPDATE sessions SET ended_at = ?1 WHERE ended_at IS NULL",
        params![now],
    )?;

    if updated == 0 {
        anyhow::bail!("No active session to stop.");
    }

    // Calculate duration
    let duration: String = conn
        .query_row(
            "SELECT started_at FROM sessions WHERE ended_at = ?1",
            params![now],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|start| {
            let start = chrono::DateTime::parse_from_rfc3339(&start).ok()?;
            let end = chrono::DateTime::parse_from_rfc3339(&now).ok()?;
            let dur = end - start;
            let hours = dur.num_hours();
            let mins = dur.num_minutes() % 60;
            let secs = dur.num_seconds() % 60;
            Some(format!("{}h {}m {}s", hours, mins, secs))
        })
        .unwrap_or_default();

    println!("{} Tracking stopped ({})", "✓".green(), duration.bold());
    Ok(())
}

pub fn show_status(conn: &Connection) -> Result<()> {
    let session = conn
        .query_row(
            "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced FROM sessions WHERE ended_at IS NULL LIMIT 1",
            [],
            |row| {
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
                })
            },
        )
        .ok();

    match session {
        Some(s) => {
            let elapsed = Utc::now() - s.started_at;
            println!("{} Active session:", "▶".green());
            println!("  Project:    {}", s.project_name.bold());
            if let Some(ref b) = s.branch_name {
                println!("  Branch:     {}", b.cyan());
            }
            if let Some(ref d) = s.description {
                println!("  Description: {}", d);
            }
            println!(
                "  Elapsed:    {}h {}m {}s",
                elapsed.num_hours(),
                elapsed.num_minutes() % 60,
                elapsed.num_seconds() % 60
            );
        }
        None => println!("{} No active session.", "●".yellow()),
    }
    Ok(())
}

pub fn list_sessions(conn: &Connection, project: Option<&str>, limit: usize) -> Result<()> {
    let query = match project {
        Some(_) => "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced FROM sessions WHERE project_name = ?1 ORDER BY started_at DESC LIMIT ?2",
        None => "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced FROM sessions ORDER BY started_at DESC LIMIT ?1",
    };

    let mut stmt = conn.prepare(query)?;

    let rows = match project {
        Some(p) => stmt.query_map(params![p, limit as i64], map_session)?,
        None => stmt.query_map(params![limit as i64], map_session)?,
    };

    for row in rows {
        let s = row?;
        let status = if let Some(ended_at) = s.ended_at {
            format!(
                "{}",
                format!(
                    "{}h {}m",
                    (ended_at - s.started_at).num_hours(),
                    (ended_at - s.started_at).num_minutes() % 60
                )
                .green()
            )
        } else {
            "active".yellow().to_string()
        };
        let synced = if s.is_synced {
            "✓".green()
        } else {
            "○".yellow()
        };
        println!(
            "{} [{}] {} — {} ({})",
            synced,
            s.started_at.format("%Y-%m-%d %H:%M"),
            s.project_name.bold(),
            status,
            s.description.as_deref().unwrap_or("")
        );
    }
    Ok(())
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
    })
}

pub struct HeatmapCell {
    pub day: u32,
    pub hour: u32,
    pub total_seconds: i64,
}

pub fn heatmap_data(conn: &Connection, days: i64, project: Option<&str>) -> Result<Vec<HeatmapCell>> {
    let mut sql = String::from(
        "SELECT CAST(strftime('%w', started_at) AS INTEGER) as day,
                CAST(strftime('%H', started_at) AS INTEGER) as hour,
                SUM(CAST((julianday(ended_at) - julianday(started_at)) * 86400 AS INTEGER)) as total_seconds
         FROM sessions
         WHERE started_at >= datetime('now', ?1 || ' days', 'localtime')
           AND ended_at IS NOT NULL",
    );
    if project.is_some() {
        sql.push_str(" AND project_name = ?2");
    }
    sql.push_str(" GROUP BY day, hour ORDER BY day, hour");

    let mut stmt = conn.prepare(&sql)?;
    let day_str = format!("-{}", days);

    let rows: Vec<HeatmapCell> = if let Some(p) = project {
        stmt.query_map(params![day_str, p], |row| {
            Ok(HeatmapCell {
                day: row.get(0)?,
                hour: row.get(1)?,
                total_seconds: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect()
    } else {
        stmt.query_map(params![day_str], |row| {
            Ok(HeatmapCell {
                day: row.get(0)?,
                hour: row.get(1)?,
                total_seconds: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect()
    };

    Ok(rows)
}

pub struct Insights {
    pub total_seconds: i64,
    pub session_count: i64,
    pub per_block: [i64; 4],
    pub per_day_of_week: [i64; 7],
    pub per_project: Vec<(String, i64)>,
}

pub fn insights_data(conn: &Connection, days: i64, project: Option<&str>) -> Result<Insights> {
    let mut sql = String::from(
        "SELECT project_name, started_at, ended_at FROM sessions
         WHERE started_at >= datetime('now', ?1 || ' days', 'localtime')
           AND ended_at IS NOT NULL",
    );
    if project.is_some() {
        sql.push_str(" AND project_name = ?2");
    }

    let mut stmt = conn.prepare(&sql)?;

    let rows: Vec<(String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> =
        if let Some(p) = project {
            stmt.query_map(params![format!("-{}", days), p], |row| {
                let started: String = row.get(1)?;
                let ended: String = row.get(2)?;
                Ok((
                    row.get::<_, String>(0)?,
                    chrono::DateTime::parse_from_rfc3339(&started)
                        .map(|d| d.to_utc())
                        .unwrap_or_default(),
                    chrono::DateTime::parse_from_rfc3339(&ended)
                        .map(|d| d.to_utc())
                        .unwrap_or_default(),
                ))
            })?
            .filter_map(|r| r.ok())
            .collect()
        } else {
            stmt.query_map(params![format!("-{}", days)], |row| {
                let started: String = row.get(1)?;
                let ended: String = row.get(2)?;
                Ok((
                    row.get::<_, String>(0)?,
                    chrono::DateTime::parse_from_rfc3339(&started)
                        .map(|d| d.to_utc())
                        .unwrap_or_default(),
                    chrono::DateTime::parse_from_rfc3339(&ended)
                        .map(|d| d.to_utc())
                        .unwrap_or_default(),
                ))
            })?
            .filter_map(|r| r.ok())
            .collect()
        };

    let session_count = rows.len() as i64;
    let mut total_seconds: i64 = 0;
    let mut per_block = [0i64; 4];
    let mut per_day_of_week = [0i64; 7];
    let mut project_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for (proj, started, ended) in &rows {
        let dur = (*ended - *started).num_seconds();
        total_seconds += dur;

        let hour = started.format("%H").to_string().parse::<u32>().unwrap_or(0);
        let block_idx = match hour {
            6..=11 => 0,  // morning
            12..=17 => 1, // afternoon
            18..=23 => 2, // evening
            _ => 3,       // night
        };
        per_block[block_idx] += dur;

        let dow = started.format("%w").to_string().parse::<usize>().unwrap_or(0);
        per_day_of_week[dow] += dur;

        *project_map.entry(proj.clone()).or_insert(0) += dur;
    }

    let mut per_project: Vec<(String, i64)> = project_map.into_iter().collect();
    per_project.sort_by_key(|b| std::cmp::Reverse(b.1));

    Ok(Insights {
        total_seconds,
        session_count,
        per_block,
        per_day_of_week,
        per_project,
    })
}

pub fn sync_sessions(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced FROM sessions WHERE is_synced = 0 AND ended_at IS NOT NULL",
    )?;

    let sessions: Vec<Session> = stmt
        .query_map([], map_session)?
        .filter_map(|r| r.ok())
        .collect();

    if sessions.is_empty() {
        println!("{} Nothing to sync.", "●".yellow());
        return Ok(());
    }

    println!("{} Syncing {} session(s)...", "↻".cyan(), sessions.len());

    let payload: Vec<nerdtime_core::SyncPayload> = sessions
        .iter()
        .map(|s| nerdtime_core::SyncPayload {
            id: s.id,
            project_name: s.project_name.clone(),
            branch_name: s.branch_name.clone(),
            commit_hash: s.commit_hash.clone(),
            description: s.description.clone(),
            started_at: s.started_at,
            ended_at: s.ended_at,
        })
        .collect();

    let cfg = config::load()?;
    let sync_url = format!("{}/sync", cfg.api_url.trim_end_matches('/'));

    let client = reqwest::blocking::Client::new();
    let mut request = client.post(&sync_url).json(&payload);

    if let Some(ref token) = cfg.token {
        request = request.bearer_auth(token);
    }

    match request.send() {
        Ok(resp) if resp.status().is_success() => {
            // Mark all as synced
            conn.execute("UPDATE sessions SET is_synced = 1 WHERE is_synced = 0", [])?;
            println!("{} Sync complete!", "✓".green());
        }
        Ok(resp) if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 => {
            anyhow::bail!(
                "Sync rejected ({}). An active subscription is required. Visit {} to upgrade.",
                resp.status(),
                "https://nerdtime.dev/settings"
            );
        }
        Ok(resp) => {
            anyhow::bail!("Sync failed with status: {}", resp.status());
        }
        Err(e) => {
            anyhow::bail!("Sync request failed: {}", e);
        }
    }

    Ok(())
}
