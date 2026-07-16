// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

fn data_dir() -> Result<PathBuf> {
    let path = dirs::config_dir()
        .context("config directory not found")?
        .join("nerdtime");
    std::fs::create_dir_all(&path).context("failed to create nerdtime config directory")?;
    Ok(path)
}

pub fn db_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("data.db"))
}

pub fn init_schema(conn: &Connection) -> Result<()> {
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

    Ok(())
}

pub fn get_connection() -> Result<Connection> {
    let db_path = db_path()?;
    let conn = Connection::open(&db_path)?;
    init_schema(&conn)?;
    Ok(conn)
}
