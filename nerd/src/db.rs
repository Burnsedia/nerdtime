// SPDX-License-Identifier: AGPL-3.0-only
use crate::config;
use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use nerdtime_core::Session;
use rusqlite::{params, Connection};
use std::io::Write;
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
        );

        CREATE TABLE IF NOT EXISTS devlog_entries (
            id TEXT PRIMARY KEY NOT NULL,
            date TEXT NOT NULL,
            title TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'human',
            tags TEXT NOT NULL DEFAULT '[]',
            context TEXT NOT NULL DEFAULT '',
            changes TEXT NOT NULL DEFAULT '',
            decisions TEXT NOT NULL DEFAULT '',
            commits TEXT NOT NULL DEFAULT '[]',
            session_id TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS cached_commits (
            sha TEXT PRIMARY KEY NOT NULL,
            subject TEXT NOT NULL,
            branch TEXT NOT NULL,
            files_changed INTEGER DEFAULT 0,
            lines_added INTEGER DEFAULT 0,
            lines_removed INTEGER DEFAULT 0,
            committed_at TEXT NOT NULL,
            cached_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY NOT NULL,
            project_name TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            estimated_seconds INTEGER,
            urgency INTEGER DEFAULT 3,
            importance INTEGER DEFAULT 3,
            quadrant INTEGER DEFAULT 4,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL,
            completed_at TEXT,
            labels TEXT
        );",
    )?;

    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN task_id TEXT", []);
    let _ = conn.execute(
        "ALTER TABLE sessions ADD COLUMN estimated_seconds INTEGER",
        [],
    );
    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN labels TEXT", []);
    let _ = conn.execute("ALTER TABLE tasks ADD COLUMN github_repo TEXT", []);
    let _ = conn.execute(
        "ALTER TABLE tasks ADD COLUMN github_issue_number INTEGER",
        [],
    );

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

pub fn start_session(
    conn: &Connection,
    project: &str,
    desc: Option<&str>,
    task_id: Option<&str>,
    estimated_seconds: Option<i64>,
    labels: Option<&str>,
) -> Result<()> {
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
        "INSERT INTO sessions (id, project_name, branch_name, commit_hash, description, started_at, task_id, estimated_seconds, labels) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id, project, branch, commit, desc, now, task_id, estimated_seconds, labels],
    )?;

    print!("{} Tracking started for {}", "✓".green(), project.bold());
    if let Some(ref b) = branch {
        print!("  branch: {}", b.cyan());
    }
    if let Some(tid) = task_id {
        let short = if tid.len() > 7 { &tid[..7] } else { tid };
        print!("  task: {}", short.cyan());
    }
    println!();
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

    print!("{} Tracking stopped ({})", "✓".green(), duration.bold());

    if let Ok(Some(tid)) = conn.query_row::<Option<String>, _, _>(
        "SELECT task_id FROM sessions WHERE ended_at = ?1",
        params![now],
        |row| row.get(0),
    ) {
        if let Ok(task) = conn.query_row(
            "SELECT title, estimated_seconds FROM tasks WHERE id = ?1",
            params![tid],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
        ) {
            let (title, task_est) = task;
            print!(" — task {}", title.cyan());
            if let Some(est_total) = task_est {
                let actual: i64 = conn
                    .query_row(
                        "SELECT COALESCE(SUM(CAST((julianday(ended_at) - julianday(started_at)) * 86400 AS INTEGER)), 0) FROM sessions WHERE task_id = ?1 AND ended_at IS NOT NULL",
                        params![tid],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                let remaining = (est_total - actual).max(0);
                print!(", {} estimated remaining", fmt_dur(remaining).bold());
            }
        }
    }

    println!();
    Ok(())
}

fn fmt_dur(seconds: i64) -> String {
    let hours = seconds / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, mins)
    } else if mins > 0 {
        format!("{}m {:02}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

pub fn show_status(conn: &Connection) -> Result<()> {
    let session = conn
        .query_row(
            "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced, task_id, estimated_seconds, labels FROM sessions WHERE ended_at IS NULL LIMIT 1",
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
                    task_id: row.get(8)?,
                    estimated_seconds: row.get(9)?,
                    labels: row.get(10)?,
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
            if let Some(ref t) = s.task_id {
                if let Ok(title) =
                    conn.query_row("SELECT title FROM tasks WHERE id = ?1", params![t], |row| {
                        row.get::<_, String>(0)
                    })
                {
                    println!("  Task:       {}", title.cyan());
                }
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
        Some(_) => "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced, task_id, estimated_seconds, labels FROM sessions WHERE project_name = ?1 ORDER BY started_at DESC LIMIT ?2",
        None => "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced, task_id, estimated_seconds, labels FROM sessions ORDER BY started_at DESC LIMIT ?1",
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
        let task_tag = s
            .task_id
            .as_ref()
            .and_then(|tid| {
                conn.query_row(
                    "SELECT title FROM tasks WHERE id = ?1",
                    params![tid],
                    |row| row.get::<_, String>(0),
                )
                .ok()
                .map(|t| format!(" [{}]", t.cyan()))
            })
            .unwrap_or_default();
        println!(
            "{} [{}] {} — {} ({}){}",
            synced,
            s.started_at.format("%Y-%m-%d %H:%M"),
            s.project_name.bold(),
            status,
            s.description.as_deref().unwrap_or(""),
            task_tag,
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
        task_id: row.get(8)?,
        estimated_seconds: row.get(9)?,
        labels: row.get(10)?,
    })
}

pub struct HeatmapCell {
    pub day: u32,
    pub hour: u32,
    pub total_seconds: i64,
}

pub fn heatmap_data(
    conn: &Connection,
    days: i64,
    project: Option<&str>,
) -> Result<Vec<HeatmapCell>> {
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

    let rows: Vec<(
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    )> = if let Some(p) = project {
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

        let dow = started
            .format("%w")
            .to_string()
            .parse::<usize>()
            .unwrap_or(0);
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

pub struct DevlogEntry {
    pub id: String,
    pub date: String,
    pub title: String,
    pub role: String,
    pub tags: Vec<String>,
    pub context: String,
    pub changes: Vec<String>,
    pub decisions: Vec<String>,
    pub commits: Vec<String>,
    pub session_id: Option<String>,
    pub created_at: String,
}

pub struct CachedCommit {
    pub sha: String,
    pub subject: String,
    pub branch: String,
    pub files_changed: i64,
    pub lines_added: i64,
    pub lines_removed: i64,
    pub committed_at: String,
    pub cached_at: String,
}

fn entry_from_row(row: &rusqlite::Row) -> rusqlite::Result<DevlogEntry> {
    let tags_str: String = row.get(4)?;
    let changes_str: String = row.get(6)?;
    let decisions_str: String = row.get(7)?;
    let commits_str: String = row.get(8)?;
    Ok(DevlogEntry {
        id: row.get(0)?,
        date: row.get(1)?,
        title: row.get(2)?,
        role: row.get(3)?,
        tags: serde_json::from_str(&tags_str).unwrap_or_default(),
        context: row.get(5)?,
        changes: changes_str
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect(),
        decisions: decisions_str
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect(),
        commits: serde_json::from_str(&commits_str).unwrap_or_default(),
        session_id: row.get(9)?,
        created_at: row.get(10)?,
    })
}

pub fn insert_devlog_entry(conn: &Connection, entry: &DevlogEntry) -> Result<()> {
    conn.execute(
        "INSERT INTO devlog_entries (id, date, title, role, tags, context, changes, decisions, commits, session_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            entry.id,
            entry.date,
            entry.title,
            entry.role,
            serde_json::to_string(&entry.tags)?,
            entry.context,
            entry.changes.join("\n"),
            entry.decisions.join("\n"),
            serde_json::to_string(&entry.commits)?,
            entry.session_id,
            entry.created_at,
        ],
    )?;
    Ok(())
}

pub fn update_devlog_entry(conn: &Connection, entry: &DevlogEntry) -> Result<()> {
    conn.execute(
        "UPDATE devlog_entries SET date=?1, title=?2, role=?3, tags=?4, context=?5, changes=?6, decisions=?7, commits=?8, session_id=?9 WHERE id=?10",
        params![
            entry.date,
            entry.title,
            entry.role,
            serde_json::to_string(&entry.tags)?,
            entry.context,
            entry.changes.join("\n"),
            entry.decisions.join("\n"),
            serde_json::to_string(&entry.commits)?,
            entry.session_id,
            entry.id,
        ],
    )?;
    Ok(())
}

pub fn get_devlog_entry(conn: &Connection, id: &str) -> Result<DevlogEntry> {
    let entry = conn.query_row(
        "SELECT id, date, title, role, tags, context, changes, decisions, commits, session_id, created_at FROM devlog_entries WHERE id = ?1",
        params![id],
        entry_from_row,
    )?;
    Ok(entry)
}

pub fn list_devlog_entries(conn: &Connection, limit: usize) -> Result<Vec<DevlogEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, date, title, role, tags, context, changes, decisions, commits, session_id, created_at FROM devlog_entries ORDER BY date DESC, created_at DESC LIMIT ?1",
    )?;
    let entries = stmt
        .query_map(params![limit as i64], entry_from_row)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(entries)
}

pub fn search_devlog_entries(
    conn: &Connection,
    query: &str,
    tags: Option<&str>,
) -> Result<Vec<DevlogEntry>> {
    let like = format!("%{}%", query);
    let mut sql = String::from(
        "SELECT id, date, title, role, tags, context, changes, decisions, commits, session_id, created_at FROM devlog_entries WHERE (title LIKE ?1 OR context LIKE ?1 OR changes LIKE ?1 OR decisions LIKE ?1)",
    );
    if tags.is_some() {
        sql.push_str(" AND tags LIKE ?2");
    }
    sql.push_str(" ORDER BY date DESC, created_at DESC LIMIT 50");

    let mut stmt = conn.prepare(&sql)?;
    let entries = if let Some(tag_filter) = tags {
        let tag_pattern = format!("%\"{}\"%", tag_filter);
        stmt.query_map(params![like, tag_pattern], entry_from_row)?
            .filter_map(|r| r.ok())
            .collect()
    } else {
        stmt.query_map(params![like], entry_from_row)?
            .filter_map(|r| r.ok())
            .collect()
    };
    Ok(entries)
}

pub fn cache_commit(conn: &Connection, commit: &CachedCommit) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO cached_commits (sha, subject, branch, files_changed, lines_added, lines_removed, committed_at, cached_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            commit.sha,
            commit.subject,
            commit.branch,
            commit.files_changed,
            commit.lines_added,
            commit.lines_removed,
            commit.committed_at,
            commit.cached_at,
        ],
    )?;
    Ok(())
}

pub fn get_cached_commits(conn: &Connection) -> Result<Vec<CachedCommit>> {
    let mut stmt = conn.prepare(
        "SELECT sha, subject, branch, files_changed, lines_added, lines_removed, committed_at, cached_at FROM cached_commits ORDER BY committed_at DESC",
    )?;
    let commits = stmt
        .query_map([], |row| {
            Ok(CachedCommit {
                sha: row.get(0)?,
                subject: row.get(1)?,
                branch: row.get(2)?,
                files_changed: row.get(3)?,
                lines_added: row.get(4)?,
                lines_removed: row.get(5)?,
                committed_at: row.get(6)?,
                cached_at: row.get(7)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(commits)
}

pub fn get_used_commit_shas(conn: &Connection) -> Result<std::collections::HashSet<String>> {
    let mut stmt = conn.prepare("SELECT commits FROM devlog_entries")?;
    let shas = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .flat_map(|s| serde_json::from_str::<Vec<String>>(&s).unwrap_or_default())
        .collect();
    Ok(shas)
}

pub fn get_last_devlog_date(conn: &Connection) -> Result<Option<String>> {
    let date = conn
        .query_row(
            "SELECT date FROM devlog_entries ORDER BY date DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();
    Ok(date)
}

pub fn get_cached_commit_map(
    conn: &Connection,
) -> Result<std::collections::HashMap<String, (i64, i64, i64)>> {
    let mut stmt =
        conn.prepare("SELECT sha, files_changed, lines_added, lines_removed FROM cached_commits")?;
    let map = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(map)
}

#[allow(dead_code)]
pub struct TaskRow {
    pub id: String,
    pub project_name: String,
    pub title: String,
    pub description: Option<String>,
    pub estimated_seconds: Option<i64>,
    pub urgency: u8,
    pub importance: u8,
    pub quadrant: u8,
    pub status: String,
    pub labels: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub actual_seconds: i64,
    pub github_repo: Option<String>,
    pub github_issue_number: Option<i64>,
}

pub fn parse_duration(input: &str) -> Result<Option<i64>> {
    let input = input.trim().to_lowercase();
    if input == "0" || input == "none" || input.is_empty() {
        return Ok(None);
    }
    if let Some(s) = input.strip_suffix('h') {
        let s = s.trim();
        if let Ok(h) = s.parse::<f64>() {
            return Ok(Some((h * 3600.0) as i64));
        }
        if s.contains('m') {
            let parts: Vec<&str> = s.splitn(2, 'm').collect();
            if parts.len() == 2 {
                let h = parts[0].parse::<f64>().unwrap_or(0.0);
                let m = parts[1].parse::<f64>().unwrap_or(0.0);
                return Ok(Some((h * 3600.0 + m * 60.0) as i64));
            }
        }
    }
    if let Some(s) = input.strip_suffix('m') {
        let s = s.trim();
        if let Ok(m) = s.parse::<f64>() {
            return Ok(Some((m * 60.0) as i64));
        }
    }
    anyhow::bail!("invalid duration: {}", input)
}

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

pub fn complete_task(conn: &Connection, id: &str, close_issue: bool) -> Result<()> {
    let tid = resolve_task_id(conn, id)?;
    let active: bool = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE task_id = ?1 AND ended_at IS NULL",
        params![tid],
        |row| row.get::<_, i64>(0),
    )? > 0;
    if active {
        anyhow::bail!("Task has an active session. Stop it first with `nerd stop`.");
    }
    // check if this task has a linked github issue
    let (gh_repo, gh_number): (Option<String>, Option<i64>) = conn.query_row(
        "SELECT github_repo, github_issue_number FROM tasks WHERE id = ?1",
        params![tid],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if let (Some(repo), Some(number)) = (&gh_repo, gh_number) {
        let should_close = if close_issue {
            true
        } else {
            print!(
                "{} Close GitHub issue {}/{}? [y/N]: ",
                "?".yellow(),
                repo,
                number
            );
            std::io::stdout().flush().ok();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
        };
        if should_close {
            if let Err(e) = crate::github::close_issue(repo, number) {
                eprintln!(
                    "{} Warning: failed to close GitHub issue: {}",
                    "!".yellow(),
                    e
                );
            } else {
                println!("{} Closed GitHub issue {}/#{}.", "✓".green(), repo, number);
            }
        }
    }
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE tasks SET status = 'completed', completed_at = ?1 WHERE id = ?2",
        params![now, tid],
    )?;
    println!("{} Task completed.", "✓".green());
    Ok(())
}

pub fn cancel_task(conn: &Connection, id: &str) -> Result<()> {
    let tid = resolve_task_id(conn, id)?;
    conn.execute(
        "UPDATE tasks SET status = 'cancelled' WHERE id = ?1",
        params![tid],
    )?;
    println!("{} Task cancelled.", "●".yellow());
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
    println!("{} Task updated.", "✓".green());
    Ok(())
}

pub type SessionEstimate = (String, String, i64, Option<i64>);
pub fn task_estimate(conn: &Connection, id: &str) -> Result<(TaskRow, Vec<SessionEstimate>)> {
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

pub fn label_summary(
    conn: &Connection,
    project: Option<&str>,
    label_filter: Option<&str>,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<(String, i64, Vec<String>)>> {
    let mut sql = String::from(
        "SELECT j.value as label, SUM(CAST((julianday(s.ended_at) - julianday(s.started_at)) * 86400 AS INTEGER)) as total_seconds, s.project_name FROM sessions s, json_each(COALESCE(s.labels, '[]')) AS j WHERE s.ended_at IS NOT NULL AND s.started_at >= ?1 AND s.started_at <= ?2",
    );
    if let Some(p) = project {
        sql.push_str(&format!(
            " AND s.project_name = '{}'",
            p.replace('\'', "''")
        ));
    }
    sql.push_str(" GROUP BY j.value, s.project_name ORDER BY total_seconds DESC");

    let mut stmt = conn.prepare(&sql)?;
    let mut map: std::collections::HashMap<String, (i64, Vec<String>)> =
        std::collections::HashMap::new();
    for row in stmt
        .query_map(params![start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .filter_map(|r| r.ok())
    {
        let (lbl, seconds, proj) = row;
        if lbl.is_empty() {
            continue;
        }
        if let Some(f) = label_filter {
            if lbl != f {
                continue;
            }
        }
        let entry = map.entry(lbl).or_insert((0, Vec::new()));
        entry.0 += seconds;
        if !entry.1.contains(&proj) {
            entry.1.push(proj);
        }
    }

    let mut result: Vec<(String, i64, Vec<String>)> =
        map.into_iter().map(|(k, (s, p))| (k, s, p)).collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.1));
    Ok(result)
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
                #[allow(clippy::unnecessary_map_or)]
                {
                    t.quadrant != 1 || t.estimated_seconds.map_or(true, |e| e <= 1800)
                }
            } else {
                true
            }
        })
        .collect())
}

pub fn sync_sessions(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, project_name, branch_name, commit_hash, description, started_at, ended_at, is_synced, task_id, estimated_seconds, labels FROM sessions WHERE is_synced = 0 AND ended_at IS NOT NULL",
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
            task_id: s.task_id.clone(),
            estimated_seconds: s.estimated_seconds,
            labels: s.labels.clone(),
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
