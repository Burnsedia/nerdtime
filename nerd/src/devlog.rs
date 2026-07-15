// SPDX-License-Identifier: AGPL-3.0-only
use crate::db;
use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use dialoguer::{Confirm, Editor, Input, Select};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn run_git(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
}

pub fn handle_cache_commit(conn: &Connection) -> Result<()> {
    let sha = match run_git(&["rev-parse", "HEAD"]) {
        Some(s) => s,
        None => {
            eprintln!("{} not in a git repository", "⚠".yellow());
            return Ok(());
        }
    };

    let subject = run_git(&["log", "-1", "--format=%s"]).unwrap_or_default();
    let branch = run_git(&["branch", "--show-current"]).unwrap_or_default();
    let date = run_git(&["log", "-1", "--format=%ai"]).unwrap_or_default();

    let stats_output = run_git(&["diff-tree", "--no-commit-id", "-r", "--numstat", "--root", "HEAD"])
        .unwrap_or_default();

    let mut files_changed: i64 = 0;
    let mut lines_added: i64 = 0;
    let mut lines_removed: i64 = 0;
    for line in stats_output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() == 3 {
            if let (Ok(a), Ok(r)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                lines_added += a;
                lines_removed += r;
                files_changed += 1;
            }
        }
    }

    let commit = db::CachedCommit {
        sha,
        subject,
        branch,
        files_changed,
        lines_added,
        lines_removed,
        committed_at: date,
        cached_at: Utc::now().to_rfc3339(),
    };

    db::cache_commit(conn, &commit)?;
    println!("{} Cached commit {}", "✓".green(), commit.sha[..7].cyan());
    Ok(())
}

pub fn handle_new(conn: &Connection) -> Result<()> {
    let title: String = Input::new()
        .with_prompt("Title")
        .interact_text()?;

    let role_idx = Select::new()
        .with_prompt("Role")
        .items(&["human", "ai", "hybrid"])
        .default(0)
        .interact()?;
    let role = ["human", "ai", "hybrid"][role_idx];

    let tags_input: String = Input::new()
        .with_prompt("Tags (comma-separated)")
        .allow_empty(true)
        .interact_text()?;
    let tags: Vec<String> = tags_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    println!("{} Opening $EDITOR for context…", "✎".cyan());
    let context: String = Editor::new()
        .edit("")
        .context("editor cancelled")?
        .unwrap_or_default()
        .trim()
        .to_string();

    println!("{} Opening $EDITOR for changes (one per line)…", "✎".cyan());
    let changes_raw: String = Editor::new()
        .edit("")
        .context("editor cancelled")?
        .unwrap_or_default();
    let changes: Vec<String> = changes_raw
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    println!("{} Opening $EDITOR for decisions (one per line)…", "✎".cyan());
    let decisions_raw: String = Editor::new()
        .edit("")
        .context("editor cancelled")?
        .unwrap_or_default();
    let decisions: Vec<String> = decisions_raw
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let last_date = db::get_last_devlog_date(conn).ok().flatten();
    let since_filter = last_date.as_deref().unwrap_or("14 days ago");

    let unlogged = get_unlogged_commits(since_filter, conn)?;

    let commits_to_include: Vec<String> = if unlogged.is_empty() {
        Vec::new()
    } else {
        println!("\nUnlogged commits found:");
        for (i, (sha, subject)) in unlogged.iter().enumerate() {
            println!("  {}. {} — {}", i + 1, sha[..7].cyan(), subject);
        }
        let include_all = Confirm::new()
            .with_prompt("Include all these commits?")
            .default(true)
            .interact()?;
        if include_all {
            unlogged.into_iter().map(|(sha, _)| sha).collect()
        } else {
            Vec::new()
        }
    };

    let now = Utc::now();
    let date = now.format("%Y-%m-%d").to_string();
    let created_at = now.to_rfc3339();

    println!("\n{}", "── Preview ──────────────────────────────".bold());
    println!("## {}: {}", date, title.bold());
    println!();
    println!("**role:** {}", role.cyan());
    if !commits_to_include.is_empty() {
        let commit_refs: Vec<String> = commits_to_include
            .iter()
            .map(|s| format!("`{}`", &s[..7]))
            .collect();
        println!("**commits:** {}", commit_refs.join(", "));
    }
    if !tags.is_empty() {
        let tag_refs: Vec<String> = tags.iter().map(|t| format!("`{}`", t)).collect();
        println!("**tags:** {}", tag_refs.join(", "));
    }
    println!();
    if !context.is_empty() {
        println!("### Context\n");
        println!("{}", context);
        println!();
    }
    if !changes.is_empty() {
        println!("### Changes\n");
        for c in &changes {
            println!("- {}", c);
        }
        println!();
    }
    if !decisions.is_empty() {
        println!("### Decisions\n");
        for d in &decisions {
            println!("- {}", d);
        }
        println!();
    }
    println!("{}", "──────────────────────────────────────────".bold());

    let confirm = Confirm::new()
        .with_prompt("Append to devlog?")
        .default(true)
        .interact()?;

    if !confirm {
        println!("{} Cancelled.", "●".yellow());
        return Ok(());
    }

    let entry = db::DevlogEntry {
        id: Uuid::new_v4().to_string(),
        date,
        title,
        role: role.to_string(),
        tags,
        context,
        changes,
        decisions,
        commits: commits_to_include,
        session_id: None,
        created_at,
    };

    db::insert_devlog_entry(conn, &entry)?;
    println!("{} Entry saved.", "✓".green());

    let regen = Confirm::new()
        .with_prompt("Regenerate DEVLOG.md?")
        .default(true)
        .interact()?;

    if regen {
        handle_generate(conn)?;
    }

    Ok(())
}

fn get_unlogged_commits(since: &str, conn: &Connection) -> Result<Vec<(String, String)>> {
    let used_shas = db::get_used_commit_shas(conn).unwrap_or_default();
    let cached = db::get_cached_commits(conn).unwrap_or_default();

    let mut unlogged: Vec<(String, String)> = cached
        .into_iter()
        .filter(|c| !used_shas.contains(&c.sha))
        .map(|c| (c.sha, c.subject))
        .collect();

    let seen_shas: std::collections::HashSet<String> =
        unlogged.iter().map(|(s, _)| s.clone()).collect();

    if let Some(git_output) = run_git(&[
        "log",
        "--since",
        since,
        "--format=%H|%s",
        "--no-merges",
    ]) {
        for line in git_output.lines() {
            if let Some((sha, subject)) = line.split_once('|') {
                let sha = sha.to_string();
                let subject = subject.to_string();
                if !used_shas.contains(&sha) && !seen_shas.contains(&sha) {
                    unlogged.push((sha, subject));
                }
            }
        }
    }

    unlogged.sort();
    unlogged.dedup();
    Ok(unlogged)
}

pub fn handle_list(conn: &Connection, limit: usize) -> Result<()> {
    let entries = db::list_devlog_entries(conn, limit)?;

    if entries.is_empty() {
        println!("  No devlog entries found. Create one with `nerd devlog new`.");
        return Ok(());
    }

    println!(
        "  {:<12}  {:<6}  {:<30}  {}",
        "Date".bold(),
        "Role".bold(),
        "Title".bold(),
        "Commits".bold(),
    );
    for e in &entries {
        let commit_count = e.commits.len();
        println!(
            "  {:<12}  {:<6}  {:<30}  {} commit(s)",
            e.date,
            e.role.cyan(),
            truncate(&e.title, 28),
            commit_count,
        );
    }
    Ok(())
}

pub fn handle_query(conn: &Connection, query: &str, tags: Option<&str>) -> Result<()> {
    let entries = db::search_devlog_entries(conn, query, tags)?;

    if entries.is_empty() {
        println!("  No entries matching \"{}\".", query);
        return Ok(());
    }

    println!("Found {} entr(ies):\n", entries.len());
    for e in &entries {
        println!(
            "  {} — {} ({})",
            e.date.bold(),
            e.title.bold(),
            e.role.cyan(),
        );
        let excerpt = e
            .context
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(120)
            .collect::<String>();
        if !excerpt.is_empty() {
            println!("    {}", excerpt.dimmed());
        }
        println!();
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
struct DevlogToml {
    date: String,
    title: String,
    role: String,
    tags: Vec<String>,
    context: String,
    changes: String,
    decisions: String,
}

pub fn handle_edit(conn: &Connection, id: &str) -> Result<()> {
    let entry = db::get_devlog_entry(conn, id)?;

    let toml_content = DevlogToml {
        date: entry.date,
        title: entry.title,
        role: entry.role,
        tags: entry.tags,
        context: entry.context,
        changes: entry.changes.join("\n"),
        decisions: entry.decisions.join("\n"),
    };

    let raw = toml::to_string_pretty(&toml_content)?;

    println!("{} Opening $EDITOR to edit entry…", "✎".cyan());
    let edited = Editor::new()
        .edit(&raw)
        .context("editor cancelled")?
        .unwrap_or_default();

    let updated: DevlogToml = toml::from_str(&edited)
        .context("failed to parse edited TOML — entry unchanged")?;

    let now = Utc::now().to_rfc3339();

    let updated_entry = db::DevlogEntry {
        id: id.to_string(),
        date: updated.date,
        title: updated.title,
        role: updated.role,
        tags: updated.tags,
        context: updated.context,
        changes: updated
            .changes
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        decisions: updated
            .decisions
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        commits: entry.commits,
        session_id: entry.session_id,
        created_at: now,
    };

    db::update_devlog_entry(conn, &updated_entry)?;
    println!("{} Entry updated.", "✓".green());

    let regen = Confirm::new()
        .with_prompt("Regenerate DEVLOG.md?")
        .default(true)
        .interact()?;

    if regen {
        handle_generate(conn)?;
    }

    Ok(())
}

pub fn handle_generate(conn: &Connection) -> Result<()> {
    let markdown = generate_devlog_md(conn)?;
    std::fs::write("DEVLOG.md", &markdown)
        .context("failed to write DEVLOG.md")?;
    println!("{} DEVLOG.md regenerated ({} lines)", "✓".green(), markdown.lines().count());
    Ok(())
}

fn generate_devlog_md(conn: &Connection) -> Result<String> {
    let entries = db::list_devlog_entries(conn, 1000)?;
    if entries.is_empty() {
        return Ok(String::new());
    }

    let commit_cache = db::get_cached_commit_map(conn).unwrap_or_default();

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
                            &sha[..7],
                            sha,
                            added,
                            removed,
                            files,
                            if *files == 1 { "" } else { "s" },
                        )
                    } else {
                        format!(
                            "[`{}`](https://github.com/Burnsedia/nerdtime/commit/{})",
                            &sha[..7],
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

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

pub fn handle_show(conn: &Connection, id: &str) -> Result<()> {
    let entry = db::get_devlog_entry(conn, id)?;
    let commit_cache = db::get_cached_commit_map(conn).unwrap_or_default();
    render_single_entry(&entry, &commit_cache);
    Ok(())
}

fn render_single_entry(
    entry: &db::DevlogEntry,
    commit_cache: &std::collections::HashMap<String, (i64, i64, i64)>,
) {
    println!("## {}: {}", entry.date, entry.title.bold());
    println!();
    println!("**role:** {}", entry.role.cyan());

    if !entry.commits.is_empty() {
        let commit_strs: Vec<String> = entry
            .commits
            .iter()
            .map(|sha| {
                if let Some((files, added, removed)) = commit_cache.get(sha) {
                    format!(
                        "`{}` (+{} / -{} lines, {} file{})",
                        &sha[..7],
                        added,
                        removed,
                        files,
                        if *files == 1 { "" } else { "s" },
                    )
                } else {
                    format!("`{}`", &sha[..7])
                }
            })
            .collect();
        println!("**commits:** {}", commit_strs.join(", "));
    }

    if !entry.tags.is_empty() {
        let tag_strs: Vec<String> = entry.tags.iter().map(|t| format!("`{}`", t)).collect();
        println!("**tags:** {}", tag_strs.join(", "));
    }
    println!();

    if !entry.context.is_empty() {
        println!("### Context\n");
        println!("{}", entry.context);
        println!();
    }
    if !entry.changes.is_empty() {
        println!("### Changes\n");
        for c in &entry.changes {
            println!("- {}", c);
        }
        println!();
    }
    if !entry.decisions.is_empty() {
        println!("### Decisions\n");
        for d in &entry.decisions {
            println!("- {}", d);
        }
        println!();
    }
}
