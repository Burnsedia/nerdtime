// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use nerdtime_core::{CachedCommit, DevlogEntry};
use rusqlite::{params, Connection};

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

pub fn render_devlog_md(conn: &Connection) -> Result<String> {
    let entries = list_devlog_entries(conn, 1000)?;
    let commit_cache = get_cached_commit_map(conn).unwrap_or_default();

    let mut out = String::from("# nerdtime.dev — Development Log\n\n");

    for entry in entries {
        out.push_str(&format!("## {}: {}\n\n", entry.date, entry.title));
        out.push_str(&format!("**role:** {}\n", entry.role));

        if !entry.commits.is_empty() {
            let commit_strs: Vec<String> = entry
                .commits
                .iter()
                .map(|sha| {
                    if let Some((files, added, removed)) = commit_cache.get(sha) {
                        format!(
                            "[`{}`](https://github.com/Burnsedia/nerdtime/commit/{}) (+{} / -{} lines, {} file{})",
                            &sha[..7.min(sha.len())],
                            sha,
                            added,
                            removed,
                            files,
                            if *files == 1 { "" } else { "s" },
                        )
                    } else {
                        format!(
                            "[`{}`](https://github.com/Burnsedia/nerdtime/commit/{})",
                            &sha[..7.min(sha.len())],
                            sha,
                        )
                    }
                })
                .collect();
            out.push_str(&format!("**commits:** {}\n", commit_strs.join(", ")));
        }

        if !entry.tags.is_empty() {
            let tag_strs: Vec<String> = entry.tags.iter().map(|t| format!("`{}`", t)).collect();
            out.push_str(&format!("**tags:** {}\n", tag_strs.join(", ")));
        }

        out.push('\n');

        if !entry.context.is_empty() {
            out.push_str("### Context\n\n");
            out.push_str(&entry.context);
            out.push_str("\n\n");
        }

        if !entry.changes.is_empty() {
            out.push_str("### Changes\n\n");
            for change in &entry.changes {
                out.push_str(&format!("- {}\n", change));
            }
            out.push('\n');
        }

        if !entry.decisions.is_empty() {
            out.push_str("### Decisions\n\n");
            for decision in &entry.decisions {
                out.push_str(&format!("- {}\n", decision));
            }
            out.push('\n');
        }

        out.push_str("---\n\n");
    }

    Ok(out)
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
