// SPDX-License-Identifier: AGPL-3.0-only
mod advisor;
mod auth;
mod config;
mod devlog;
mod github;
mod insights;
mod tasks;
mod tui;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use colored::Colorize;
use nerdtime_db as db;
use nerdtime_db::SyncPayload;
use std::io::Write;

#[derive(Parser)]
#[command(name = "nerd", version = "0.1.0", about = "Flow-first time tracking")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start tracking time for a project
    Start {
        project: String,
        #[arg(short, long)]
        desc: Option<String>,
        #[arg(short = 't', long)]
        task: Option<String>,
        #[arg(short = 'e', long)]
        estimate: Option<String>,
        #[arg(short = 'l', long)]
        label: Vec<String>,
        #[arg(short = 'i', long)]
        issue: Option<String>,
    },
    /// Stop the active session
    Stop,
    /// Show active session status
    Status,
    /// Sync unsynced sessions to the server
    Sync,
    /// List recent sessions
    Log {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Log in interactively (or provide a token for headless auth)
    Login {
        /// API server URL (e.g. https://nerdtime.dev)
        #[arg(short, long)]
        url: Option<String>,
        /// JWT token (headless mode)
        token: Option<String>,
    },
    /// Create a new account
    Signup,
    /// Clear stored credentials
    Logout,
    /// Show configuration
    Config,
    /// Show a heatmap of tracked time (weekday x hour grid)
    Heatmap {
        #[arg(short, long, default_value = "30")]
        days: i64,
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Show productivity insights and patterns
    Insights {
        #[arg(short, long, default_value = "30")]
        days: i64,
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Structured development logging
    Devlog {
        #[command(subcommand)]
        action: DevlogCommands,
    },
    /// Manage tasks with Eisenhower Matrix
    #[command(subcommand)]
    Task(TaskCommands),
    /// Show estimation accuracy for a task or project
    Estimate {
        id: Option<String>,
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Show summary by label
    Summary {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        label: Option<String>,
        #[arg(short = 'd', long, default_value = "30")]
        days: i64,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Get a suggestion on what to work on right now
    WhatShouldIWorkOn {
        #[arg(short, long)]
        time: Option<String>,
        #[arg(short, long)]
        energy: Option<String>,
        #[arg(short, long)]
        blocked: Option<String>,
    },
    /// Launch the terminal UI
    Tui,
}

#[derive(Subcommand)]
enum TaskCommands {
    /// Add a new task
    Add {
        project: String,
        title: String,
        #[arg(short, long)]
        desc: Option<String>,
        #[arg(short = 'e', long)]
        estimate: Option<String>,
        #[arg(short = 'l', long)]
        label: Vec<String>,
        #[arg(long)]
        urgency: Option<u8>,
        #[arg(long)]
        importance: Option<u8>,
        #[arg(long)]
        q1: bool,
        #[arg(long)]
        q2: bool,
        #[arg(long)]
        q3: bool,
        #[arg(long)]
        q4: bool,
    },
    /// List tasks
    List {
        project: Option<String>,
        #[arg(short, long)]
        status: Option<String>,
    },
    /// View Eisenhower Matrix
    Matrix {
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Mark a task as completed
    Complete {
        id: String,
        #[arg(long)]
        close_issue: bool,
    },
    /// Cancel a task
    Cancel { id: String },
    /// Edit a task
    Edit {
        id: String,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short = 'e', long)]
        estimate: Option<String>,
        #[arg(long)]
        urgency: Option<u8>,
        #[arg(long)]
        importance: Option<u8>,
        #[arg(long)]
        q1: bool,
        #[arg(long)]
        q2: bool,
        #[arg(long)]
        q3: bool,
        #[arg(long)]
        q4: bool,
        #[arg(short = 'l', long)]
        label: Option<String>,
    },
    /// Import GitHub issues as tasks
    ImportGithub {
        #[arg(short, long)]
        repo: Option<String>,
        #[arg(short, long)]
        issue: Option<i64>,
        #[arg(short, long)]
        label: Option<String>,
        #[arg(short, long)]
        milestone: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum DevlogCommands {
    /// Create a new devlog entry (interactive)
    New {
        /// Auto-generate DEVLOG.md after saving (skip confirmation prompt)
        #[arg(short, long)]
        generate: bool,
    },
    /// Edit an existing entry
    Edit {
        id: String,
        /// Auto-generate DEVLOG.md after saving (skip confirmation prompt)
        #[arg(short, long)]
        generate: bool,
    },
    /// Search entries by text or tags
    Query {
        query: String,
        #[arg(short, long)]
        tags: Option<String>,
    },
    /// List recent entries
    List {
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Regenerate DEVLOG.md from the database
    Generate,
    /// Cache current commit data (used by post-commit hook)
    CacheCommit,
    /// Show a single entry
    Show { id: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let conn = db::get_connection()?;

    match &cli.command {
        Commands::Start {
            project,
            desc,
            task,
            estimate,
            label,
            issue,
        } => {
            let resolved_task_id = task
                .as_deref()
                .and_then(|t| db::resolve_task_id(&conn, t).ok());
            let issue_task_id = if let Some(issue_ref) = issue {
                let cfg = config::load().ok();
                let detected_repo = github::detect_repo().ok();
                let default_repo = cfg.as_ref().and_then(|c| c.default_github_repo.as_deref());
                let repo = detected_repo.as_deref().or(default_repo);
                let (gh_repo, gh_number) = github::parse_issue_ref(issue_ref, repo)?;
                if let Some(existing) = db::find_task_by_github_issue(&conn, &gh_repo, gh_number)? {
                    Some(existing)
                } else {
                    let token = cfg.as_ref().and_then(|c| c.github_token.as_deref());
                    let issue_data = github::get_issue(&gh_repo, gh_number, token)?;
                    let title = issue_data["title"]
                        .as_str()
                        .unwrap_or("untitled")
                        .to_string();
                    let body = issue_data["body"].as_str().unwrap_or("");
                    let tid = db::add_task(
                        &conn,
                        project,
                        &title,
                        Some(body),
                        None,
                        3,
                        3,
                        None,
                        Some(&gh_repo),
                        Some(gh_number),
                    )?;
                    println!(
                        "  {} Created task for GitHub issue #{}: {}",
                        "+".green(),
                        gh_number,
                        title.bold()
                    );
                    Some(tid)
                }
            } else {
                None
            };
            let task_id_ref = resolved_task_id.as_deref().or(issue_task_id.as_deref());
            let estimate_secs = estimate
                .as_deref()
                .map(db::parse_duration)
                .transpose()?
                .flatten();
            let parsed_labels: Vec<String> = label
                .iter()
                .flat_map(|l| {
                    l.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
                .collect();
            let labels_json = if parsed_labels.is_empty() {
                if let Some(rid) = task_id_ref {
                    db::get_task_labels(&conn, rid).ok().flatten()
                } else {
                    None
                }
            } else {
                Some(serde_json::to_string(&parsed_labels)?)
            };
            let session = db::start_session(
                &conn,
                project,
                desc.as_deref(),
                task_id_ref,
                estimate_secs,
                labels_json.as_deref(),
            )?;
            print!("{} Tracking started for {}", "✓".green(), project.bold());
            if let Some(ref b) = session.branch_name {
                print!("  branch: {}", b.cyan());
            }
            if let Some(tid) = &session.task_id {
                let short = if tid.len() > 7 { &tid[..7] } else { tid };
                print!("  task: {}", short.cyan());
            }
            println!();
            Ok(())
        }
        Commands::Stop => {
            let session = db::stop_session(&conn)?;
            let elapsed = session
                .ended_at
                .zip(Some(session.started_at))
                .map(|(end, start)| end - start)
                .unwrap_or_default();
            let duration = db::fmt_duration(elapsed.num_seconds());
            print!("{} Tracking stopped ({})", "✓".green(), duration.bold());

            if let Some(tid) = &session.task_id {
                if let Ok(task) = db::list_tasks(&conn, None, None) {
                    if let Some(t) = task.iter().find(|t| &t.id == tid) {
                        print!(" — task {}", t.title.cyan());
                        if let Some(est_total) = t.estimated_seconds {
                            let actual: i64 = conn
                                .query_row(
                                    "SELECT COALESCE(SUM(CAST((julianday(ended_at) - julianday(started_at)) * 86400 AS INTEGER)), 0) FROM sessions WHERE task_id = ?1 AND ended_at IS NOT NULL",
                                    rusqlite::params![tid],
                                    |row| row.get(0),
                                )
                                .unwrap_or(0);

                            let remaining = (est_total - actual).max(0);
                            print!(
                                ", {} estimated remaining",
                                db::fmt_duration(remaining).bold()
                            );
                        }
                    }
                }
            }

            println!();
            Ok(())
        }
        Commands::Status => {
            match db::show_status(&conn)? {
                Some(session) => {
                    let elapsed = Utc::now() - session.started_at;
                    println!("{} Active session:", "▶".green());
                    println!("  Project:    {}", session.project_name.bold());
                    if let Some(ref b) = session.branch_name {
                        println!("  Branch:     {}", b.cyan());
                    }
                    if let Some(ref d) = session.description {
                        println!("  Description: {}", d);
                    }
                    if let Some(ref t) = session.task_id {
                        if let Ok(title) = conn.query_row(
                            "SELECT title FROM tasks WHERE id = ?1",
                            rusqlite::params![t],
                            |row| row.get::<_, String>(0),
                        ) {
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
        Commands::Sync => sync_sessions(&conn),
        Commands::Log { project, limit } => {
            let sessions = db::list_sessions(&conn, project.as_deref(), *limit)?;
            for s in &sessions {
                let duration_str = if let Some(ended_at) = s.ended_at {
                    let dur = (ended_at - s.started_at).num_seconds();
                    db::fmt_duration(dur).green().to_string()
                } else {
                    "active".yellow().to_string()
                };
                let synced = if s.is_synced {
                    "✓".green().to_string()
                } else {
                    "○".yellow().to_string()
                };
                let task_tag = s
                    .task_id
                    .as_ref()
                    .and_then(|tid| {
                        conn.query_row(
                            "SELECT title FROM tasks WHERE id = ?1",
                            rusqlite::params![tid],
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
                    duration_str,
                    s.description.as_deref().unwrap_or(""),
                    task_tag,
                );
            }
            Ok(())
        }
        Commands::Login { url, token } => match token {
            Some(t) => login_headless(url.as_deref(), t),
            None => {
                let cfg = config::load()?;
                if cfg.token.is_some() {
                    let email = cfg.user_email.as_deref().unwrap_or("unknown");
                    anyhow::bail!("Already logged in as {}. Run `nerd logout` first.", email);
                }
                if let Some(u) = url {
                    let mut cfg = config::load()?;
                    cfg.api_url = u.trim_end_matches('/').to_string();
                    config::save(&cfg)?;
                }
                let email: String = dialoguer::Input::new()
                    .with_prompt("Email")
                    .validate_with(|input: &String| -> Result<(), &str> {
                        if input.contains('@') {
                            Ok(())
                        } else {
                            Err("Must contain @")
                        }
                    })
                    .interact_text()?;
                print!("Password: ");
                std::io::stdout().flush().ok();
                let password = rpassword::read_password()?;
                auth::login(&email, &password)
            }
        },
        Commands::Signup => {
            let email: String = dialoguer::Input::new()
                .with_prompt("Email")
                .validate_with(|input: &String| -> Result<(), &str> {
                    if input.contains('@') {
                        Ok(())
                    } else {
                        Err("Must contain @")
                    }
                })
                .interact_text()?;
            let name: String = dialoguer::Input::new()
                .with_prompt("Name")
                .interact_text()?;
            print!("Password: ");
            std::io::stdout().flush().ok();
            let password = rpassword::read_password()?;
            print!("Confirm password: ");
            std::io::stdout().flush().ok();
            let confirm = rpassword::read_password()?;
            if password != confirm {
                anyhow::bail!("Passwords do not match.");
            }
            auth::signup(&email, &password, &name)
        }
        Commands::Logout => {
            let cfg = config::load()?;
            if cfg.token.is_none() {
                anyhow::bail!("You are not logged in.");
            }
            auth::logout()
        }
        Commands::Config => show_config(),
        Commands::Heatmap { days, project } => {
            let cells = db::heatmap_data(&conn, *days, project.as_deref())?;
            print!("{}", insights::render_heatmap(&cells));
            Ok(())
        }
        Commands::Insights { days, project } => {
            let data = db::insights_data(&conn, *days, project.as_deref())?;
            print!("{}", insights::render_insights(&data));
            Ok(())
        }
        Commands::Devlog { action } => match action {
            DevlogCommands::New { generate } => devlog::handle_new(&conn, *generate),
            DevlogCommands::Edit { id, generate } => devlog::handle_edit(&conn, id, *generate),
            DevlogCommands::Query { query, tags } => {
                devlog::handle_query(&conn, query, tags.as_deref())
            }
            DevlogCommands::List { limit } => devlog::handle_list(&conn, *limit),
            DevlogCommands::Generate => devlog::handle_generate(&conn),
            DevlogCommands::CacheCommit => devlog::handle_cache_commit(&conn),
            DevlogCommands::Show { id } => devlog::handle_show(&conn, id),
        },
        Commands::Task(cmd) => handle_task(&conn, cmd),
        Commands::Estimate { id, project } => {
            handle_estimate(&conn, id.as_deref(), project.as_deref())
        }
        Commands::Summary {
            project,
            label,
            days,
            from,
            to,
            json,
        } => handle_summary(
            &conn,
            project.as_deref(),
            label.as_deref(),
            *days,
            from.as_deref(),
            to.as_deref(),
            *json,
        ),
        Commands::WhatShouldIWorkOn {
            time,
            energy,
            blocked,
        } => handle_advisor(
            &conn,
            time.as_deref(),
            energy.as_deref(),
            blocked.as_deref(),
        ),
        Commands::Tui => tui::run(&conn),
    }
}

fn handle_task(conn: &rusqlite::Connection, cmd: &TaskCommands) -> Result<()> {
    match cmd {
        TaskCommands::Add {
            project,
            title,
            desc,
            estimate,
            label,
            urgency,
            importance,
            q1,
            q2,
            q3,
            q4,
        } => {
            let est_secs = estimate
                .as_deref()
                .map(db::parse_duration)
                .transpose()?
                .flatten();
            let (u, i) = resolve_eisenhower(*urgency, *importance, *q1, *q2, *q3, *q4, conn)?;
            let parsed_labels: Vec<String> = label
                .iter()
                .flat_map(|l| {
                    l.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                })
                .collect();
            let labels_json = if parsed_labels.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&parsed_labels)?)
            };
            let id = db::add_task(
                conn,
                project,
                title,
                desc.as_deref(),
                est_secs,
                u,
                i,
                labels_json.as_deref(),
                None,
                None,
            )?;
            println!(
                "{} Task created: {} ({})",
                "✓".green(),
                title.bold(),
                id[..7].cyan()
            );
            Ok(())
        }
        TaskCommands::List { project, status } => {
            let tasks = db::list_tasks(conn, project.as_deref(), status.as_deref())?;
            print!("{}", tasks::render_task_list(&tasks));
            Ok(())
        }
        TaskCommands::Matrix { project } => {
            let tasks = db::list_tasks(conn, project.as_deref(), Some("active"))?;
            print!("{}", tasks::render_matrix(&tasks));
            Ok(())
        }
        TaskCommands::Complete { id, close_issue } => {
            let resolved_id = db::resolve_task_id(conn, id)?;

            if *close_issue {
                if let Ok((Some(repo), Some(number))) = db::get_task_github_info(conn, &resolved_id)
                {
                    if let Err(e) = github::close_issue(&repo, number) {
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

            db::complete_task(conn, &resolved_id)?;
            println!("{} Task completed.", "✓".green());
            Ok(())
        }
        TaskCommands::Cancel { id } => {
            db::cancel_task(conn, id)?;
            println!("{} Task cancelled.", "●".yellow());
            Ok(())
        }
        TaskCommands::Edit {
            id,
            title,
            estimate,
            urgency,
            importance,
            q1,
            q2,
            q3,
            q4,
            label,
        } => {
            let est = estimate.as_deref().map(db::parse_duration).transpose()?;
            let (u, i) = resolve_eisenhower(*urgency, *importance, *q1, *q2, *q3, *q4, conn)?;
            let labels: Option<Option<&str>> =
                label
                    .as_deref()
                    .map(|l| if l.is_empty() { None } else { Some(l) });
            db::edit_task(conn, id, title.as_deref(), est, Some(u), Some(i), labels)?;
            println!("{} Task updated.", "✓".green());
            Ok(())
        }
        TaskCommands::ImportGithub {
            repo,
            issue,
            label,
            milestone,
            dry_run,
        } => {
            let cfg = config::load().ok();
            let detected_repo = github::detect_repo().ok();
            let config_repo = cfg.as_ref().and_then(|c| c.default_github_repo.as_deref());
            let gh_repo = repo
                .as_deref()
                .or(detected_repo.as_deref())
                .or(config_repo)
                .context("Could not determine repository. Use --repo, set default_github_repo, or run from a git repo with a GitHub remote.")?;
            let token = cfg.as_ref().and_then(|c| c.github_token.as_deref());

            let issues: Vec<serde_json::Value> = if let Some(single) = issue {
                let issue_data = github::get_issue(gh_repo, *single, token)?;
                vec![issue_data]
            } else {
                github::list_issues(gh_repo, label.as_deref(), milestone.as_deref(), None, token)?
            };

            if issues.is_empty() {
                println!("  No open issues found in {}.", gh_repo);
                return Ok(());
            }

            if *dry_run {
                println!(
                    "\n  {} Would import {} issue(s) from {}:\n",
                    "ℹ".cyan(),
                    issues.len(),
                    gh_repo.bold()
                );
                for issue_data in &issues {
                    let num = issue_data["number"].as_i64().unwrap_or(0);
                    let title = issue_data["title"].as_str().unwrap_or("untitled");
                    println!("    #{} {}", num, title.cyan());
                }
                return Ok(());
            }

            println!("\n  Importing issues from {}:\n", gh_repo.bold());
            let mut count = 0;
            for issue_data in &issues {
                match github::import_issue_as_task(conn, gh_repo, issue_data, token) {
                    Ok(Some(_)) => count += 1,
                    Ok(None) => {}
                    Err(e) => eprintln!("  {} Failed to import issue: {}", "!".red(), e),
                }
            }
            println!(
                "\n  {} Imported {} issue(s) as tasks.\n",
                "✓".green(),
                count
            );
            Ok(())
        }
    }
}

fn resolve_eisenhower(
    urgency: Option<u8>,
    importance: Option<u8>,
    q1: bool,
    q2: bool,
    q3: bool,
    q4: bool,
    _conn: &rusqlite::Connection,
) -> Result<(u8, u8)> {
    if q1 {
        return Ok((5, 5));
    }
    if q2 {
        return Ok((2, 5));
    }
    if q3 {
        return Ok((5, 2));
    }
    if q4 {
        return Ok((2, 2));
    }
    let u = urgency.unwrap_or(3);
    let i = importance.unwrap_or(3);
    Ok((u, i))
}

fn handle_estimate(
    conn: &rusqlite::Connection,
    id: Option<&str>,
    project: Option<&str>,
) -> Result<()> {
    if let Some(task_id) = id {
        let (task, sessions) = db::task_estimate(conn, task_id)?;
        print!("{}", tasks::render_estimate(&task, &sessions));
    } else if let Some(proj) = project {
        let tasks = db::list_tasks(conn, Some(proj), None)?;
        print_task_project_estimate(&tasks, proj);
    } else {
        let tasks = db::list_tasks(conn, None, None)?;
        print!("{}", tasks::render_task_list(&tasks));
    }
    Ok(())
}

fn print_task_project_estimate(tasks: &[db::TaskRow], project: &str) {
    println!("\n  Project: {}\n", project.bold());
    let mut total_est: i64 = 0;
    let mut total_act: i64 = 0;

    for t in tasks {
        let est_str = t
            .estimated_seconds
            .map(db::fmt_duration)
            .unwrap_or_else(|| "—".to_string());
        let act_str = db::fmt_duration(t.actual_seconds);
        if let Some(est) = t.estimated_seconds {
            total_est += est;
            total_act += t.actual_seconds;
            let status = if t.actual_seconds <= est {
                format!("{} under", "✓".green())
            } else {
                format!(
                    "{} over by {}",
                    "✗".red(),
                    db::fmt_duration(t.actual_seconds - est)
                )
            };
            println!(
                "  {:<28} {} est → {} act  {}",
                t.title.bold(),
                est_str.cyan(),
                act_str,
                status
            );
        } else {
            println!("  {:<28} —         → {} act", t.title.bold(), act_str);
        }
    }

    println!();
    println!("  {}", "Project totals:".bold());
    println!(
        "  {} tracked across {} tasks",
        db::fmt_duration(total_act).bold(),
        tasks.len()
    );
    if total_est > 0 {
        println!("  {} total estimate", db::fmt_duration(total_est).bold());
        let remaining = total_est - total_act;
        if remaining > 0 {
            println!(
                "  {} remaining (in active tasks)",
                db::fmt_duration(remaining).bold()
            );
        }
    }
}

fn handle_summary(
    conn: &rusqlite::Connection,
    project: Option<&str>,
    label: Option<&str>,
    days: i64,
    from: Option<&str>,
    to: Option<&str>,
    _json: bool,
) -> Result<()> {
    let end = to.unwrap_or("2099-12-31").to_string();
    let start = match from {
        Some(s) => s.to_string(),
        None => {
            let d = Utc::now() - chrono::Duration::days(days);
            d.format("%Y-%m-%d").to_string()
        }
    };

    let rows = db::label_summary(conn, project, label, &start, &end)?;
    if rows.is_empty() {
        println!("  No sessions found for the given filters.");
        return Ok(());
    }

    println!("\n  Summary ({} to {}):\n", start, end);
    println!(
        "  {:<16} {:<10} {}",
        "Label".bold(),
        "Time".bold(),
        "Projects".bold()
    );
    let mut total_seconds: i64 = 0;
    for (lbl, secs, projs) in &rows {
        println!(
            "  {:<16} {:<10} {}",
            lbl.cyan(),
            db::fmt_duration(*secs).bold(),
            projs.join(", "),
        );
        total_seconds += secs;
    }
    println!("  {}", "─".repeat(50));
    println!(
        "  {:<16} {:<10} {} label(s)",
        "Total".bold(),
        db::fmt_duration(total_seconds).bold(),
        rows.len(),
    );
    println!();
    Ok(())
}

fn handle_advisor(
    conn: &rusqlite::Connection,
    time: Option<&str>,
    energy: Option<&str>,
    blocked: Option<&str>,
) -> Result<()> {
    match (time, energy) {
        (Some(t), Some(e)) => {
            let available_seconds = db::parse_duration(t)?.unwrap_or(3600);
            let input = db::AdvisorInput {
                available_seconds,
                energy: e.to_string(),
                blocked: blocked.map(|s| s.to_string()),
            };
            let result = db::decide(conn, &input)?;
            println!("\n  {}: {}", "Suggestion".bold(), result.task_title.bold());
            println!("  {}", result.reason);
            Ok(())
        }
        _ => advisor::run_interactive(conn),
    }
}

fn login_headless(url: Option<&str>, token: &str) -> Result<()> {
    let mut cfg = config::load()?;
    if let Some(u) = url {
        cfg.api_url = u.trim_end_matches('/').to_string();
    }
    cfg.token = Some(token.to_string());
    config::save(&cfg)?;
    println!("{} Authentication saved for {}", "✓".green(), cfg.api_url);
    Ok(())
}

fn show_config() -> Result<()> {
    let cfg = config::load()?;
    println!("API URL:  {}", cfg.api_url);
    println!(
        "Token:    {}",
        if cfg.token.is_some() {
            "✓ set".green()
        } else {
            "not set".yellow()
        }
    );
    println!(
        "User:     {}",
        cfg.user_email.as_deref().unwrap_or("not logged in")
    );
    println!(
        "GitHub token: {}",
        if cfg.github_token.is_some() {
            "✓ set".green()
        } else {
            "not set".yellow()
        }
    );
    println!(
        "Default repo: {}",
        cfg.default_github_repo.as_deref().unwrap_or("not set")
    );
    Ok(())
}

fn sync_sessions(conn: &rusqlite::Connection) -> Result<()> {
    let sessions = db::get_unsynced_sessions(conn)?;

    if sessions.is_empty() {
        println!("{} Nothing to sync.", "●".yellow());
        return Ok(());
    }

    println!("{} Syncing {} session(s)...", "↻".cyan(), sessions.len());

    let payload: Vec<SyncPayload> = sessions
        .iter()
        .map(|s| SyncPayload {
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
            db::mark_synced(conn)?;
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
